use crate::{
    invalid, parse_receipt, validate_cents, validate_date, validate_ref, validate_text, Category,
    DraftStatus, ReceiptDraft, ReceiptEdits, ReceiptError, Result, SaveReceiptRequest, Transaction,
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
    time::Duration,
};
use uuid::Uuid;

/// SQLite is opened only by the trusted native host. Receipt file references are
/// opaque identifiers; this crate never resolves them or opens receipt images.
/// A mutex serializes operations on a connection, and SQLite's write transaction
/// enforces save idempotency across separately opened process/connection instances.
pub struct ReceiptStore {
    connection: Mutex<Connection>,
}
fn storage<T>(result: rusqlite::Result<T>) -> Result<T> {
    result.map_err(|_| ReceiptError::Storage)
}
fn encode<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(|_| ReceiptError::Storage)
}
fn decode<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value).map_err(|_| ReceiptError::Storage)
}
impl ReceiptStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::initialize(storage(Connection::open(path))?)
    }
    pub fn in_memory() -> Result<Self> {
        Self::initialize(storage(Connection::open_in_memory())?)
    }
    fn initialize(connection: Connection) -> Result<Self> {
        storage(connection.busy_timeout(Duration::from_secs(5)))?;
        let version: i64 =
            storage(connection.pragma_query_value(None, "user_version", |row| row.get(0)))?;
        if version > 2 {
            return Err(ReceiptError::Storage);
        }
        storage(connection.execute_batch("PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS receipt_accounts (id TEXT PRIMARY KEY, currency TEXT NOT NULL CHECK(currency='EUR'));
            INSERT OR IGNORE INTO receipt_accounts(id,currency) VALUES('local-eur','EUR');
            CREATE TABLE IF NOT EXISTS receipt_categories (id TEXT PRIMARY KEY, label TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS receipt_drafts (id TEXT PRIMARY KEY, data TEXT NOT NULL, created_at TEXT NOT NULL);
            CREATE UNIQUE INDEX IF NOT EXISTS receipt_drafts_image_identity
                ON receipt_drafts(json_extract(data,'$.imageRef'));
            CREATE TABLE IF NOT EXISTS receipt_transactions (
                id TEXT PRIMARY KEY,
                draft_id TEXT NOT NULL UNIQUE REFERENCES receipt_drafts(id),
                account_id TEXT NOT NULL REFERENCES receipt_accounts(id),
                category_id TEXT NOT NULL REFERENCES receipt_categories(id),
                data TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            PRAGMA user_version = 2;"))?;
        for (id, label) in [
            ("groceries", "Lebensmittel"),
            ("dining", "Essen & Trinken"),
            ("transport", "Mobilität"),
            ("household", "Haushalt"),
            ("health", "Gesundheit"),
            ("shopping", "Einkäufe"),
            ("subscriptions", "Abonnements"),
            ("other", "Sonstiges"),
        ] {
            storage(connection.execute(
                "INSERT INTO receipt_categories(id,label) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET label=excluded.label",
                params![id, label],
            ))?;
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn connection(&self) -> Result<MutexGuard<'_, Connection>> {
        self.connection.lock().map_err(|_| ReceiptError::Storage)
    }
    pub fn create_draft(&self, image_ref: &str, ocr_text: &str) -> Result<ReceiptDraft> {
        self.create_draft_with_preview(image_ref, None, ocr_text)
    }
    pub fn create_draft_with_preview(
        &self,
        image_ref: &str,
        preview_ref: Option<&str>,
        ocr_text: &str,
    ) -> Result<ReceiptDraft> {
        validate_ref(image_ref)?;
        if let Some(value) = preview_ref {
            validate_ref(value)?;
        }
        let mut connection = self.connection()?;
        let transaction =
            storage(connection.transaction_with_behavior(TransactionBehavior::Immediate))?;
        let existing: Option<String> = storage(
            transaction
                .query_row(
                    "SELECT data FROM receipt_drafts WHERE json_extract(data,'$.imageRef')=?1",
                    [image_ref],
                    |row| row.get(0),
                )
                .optional(),
        )?;
        if let Some(existing) = existing {
            // Repeated capture/import callbacks must never replace reviewed data,
            // OCR, preview identity, or a saved record with a second draft.
            return decode(&existing);
        }
        let extracted = parse_receipt(ocr_text)?;
        let draft = ReceiptDraft {
            id: Uuid::new_v4().to_string(),
            image_ref: image_ref.to_owned(),
            preview_ref: preview_ref.map(str::to_owned),
            ocr_text: ocr_text.to_owned(),
            merchant: extracted.merchant,
            date: extracted.date,
            total_cents: extracted.total_cents,
            currency: extracted.currency,
            category_id: None,
            amount_candidates: extracted.amount_candidates,
            warnings: extracted.warnings,
            status: DraftStatus::Draft,
        };
        storage(transaction.execute(
            "INSERT INTO receipt_drafts(id,data,created_at) VALUES(?1,?2,?3)",
            params![draft.id, encode(&draft)?, Utc::now().to_rfc3339()],
        ))?;
        storage(transaction.commit())?;
        Ok(draft)
    }
    pub fn get_draft(&self, id: &str) -> Result<ReceiptDraft> {
        validate_ref(id)?;
        let connection = self.connection()?;
        load_draft(&connection, id)
    }
    /// Persist an incomplete but structurally valid edit. Clearing a field is
    /// permitted; save performs complete validation and explicit confirmation.
    pub fn update_draft(&self, id: &str, edits: &ReceiptEdits) -> Result<ReceiptDraft> {
        validate_ref(id)?;
        if let Some(merchant) = &edits.merchant {
            validate_text(merchant, "Merchant", 160)?;
        }
        if let Some(date) = &edits.date {
            validate_date(date)?;
        }
        if let Some(total) = edits.total_cents {
            validate_cents(total)?;
        }
        validate_text(&edits.currency, "Currency", 8)?;
        let mut connection = self.connection()?;
        let transaction =
            storage(connection.transaction_with_behavior(TransactionBehavior::Immediate))?;
        let mut draft = load_draft(&transaction, id)?;
        if draft.status == DraftStatus::Saved {
            return Err(ReceiptError::DraftAlreadySaved);
        }
        if let Some(category) = &edits.category_id {
            require_category(&transaction, category)?;
        }
        draft.merchant = edits.merchant.as_ref().map(|value| value.trim().to_owned());
        draft.date = edits.date.clone();
        draft.total_cents = edits.total_cents;
        draft.currency = edits.currency.clone();
        draft.category_id = edits.category_id.clone();
        storage(transaction.execute(
            "UPDATE receipt_drafts SET data=?1 WHERE id=?2",
            params![encode(&draft)?, id],
        ))?;
        storage(transaction.commit())?;
        Ok(draft)
    }
    /// Save only following explicit UI confirmation of the exact edited fields.
    /// The draft ID is the durable idempotency key. Identical retries return the
    /// original transaction; a changed replay cannot create or mutate a record.
    pub fn save_receipt(&self, request: &SaveReceiptRequest) -> Result<Transaction> {
        if !request.confirmed {
            return Err(ReceiptError::ConfirmationRequired);
        }
        validate_ref(&request.draft_id)?;
        validate_text(&request.merchant, "Merchant", 160)?;
        validate_date(&request.date)?;
        validate_cents(request.total_cents)?;
        if request.currency != "EUR" {
            return Err(invalid(
                "Only EUR receipts can be saved; currency conversion is not supported",
            ));
        }
        validate_ref(&request.category_id)?;
        let mut connection = self.connection()?;
        let transaction =
            storage(connection.transaction_with_behavior(TransactionBehavior::Immediate))?;
        let mut draft = load_draft(&transaction, &request.draft_id)?;
        let existing: Option<String> = storage(
            transaction
                .query_row(
                    "SELECT data FROM receipt_transactions WHERE draft_id=?1",
                    [&request.draft_id],
                    |row| row.get(0),
                )
                .optional(),
        )?;
        if let Some(existing) = existing {
            let saved: Transaction = decode(&existing)?;
            if same_payload(&saved, request) {
                return Ok(saved);
            }
            return Err(ReceiptError::ConflictingSave);
        }
        if draft.status == DraftStatus::Saved {
            return Err(ReceiptError::Storage);
        }
        require_category(&transaction, &request.category_id)?;
        let saved = Transaction {
            id: Uuid::new_v4().to_string(),
            draft_id: request.draft_id.clone(),
            merchant: request.merchant.trim().to_owned(),
            date: request.date.clone(),
            total_cents: request.total_cents,
            currency: request.currency.clone(),
            category_id: request.category_id.clone(),
            image_ref: draft.image_ref.clone(),
            created_at: Utc::now().to_rfc3339(),
        };
        storage(transaction.execute("INSERT INTO receipt_transactions(id,draft_id,account_id,category_id,data,created_at) VALUES(?1,?2,'local-eur',?3,?4,?5)",params![saved.id,saved.draft_id,saved.category_id,encode(&saved)?,saved.created_at]))?;
        draft.merchant = Some(saved.merchant.clone());
        draft.date = Some(saved.date.clone());
        draft.total_cents = Some(saved.total_cents);
        draft.currency = saved.currency.clone();
        draft.category_id = Some(saved.category_id.clone());
        draft.status = DraftStatus::Saved;
        storage(transaction.execute(
            "UPDATE receipt_drafts SET data=?1 WHERE id=?2",
            params![encode(&draft)?, draft.id],
        ))?;
        storage(transaction.commit())?;
        Ok(saved)
    }
    pub fn discard_draft(&self, id: &str) -> Result<()> {
        validate_ref(id)?;
        let mut connection = self.connection()?;
        let transaction =
            storage(connection.transaction_with_behavior(TransactionBehavior::Immediate))?;
        let draft = load_draft(&transaction, id)?;
        if draft.status == DraftStatus::Saved {
            return Err(ReceiptError::DraftAlreadySaved);
        }
        storage(transaction.execute("DELETE FROM receipt_drafts WHERE id=?1", [id]))?;
        storage(transaction.commit())?;
        Ok(())
    }
    pub fn list_transactions(&self) -> Result<Vec<Transaction>> {
        let connection = self.connection()?;
        let mut statement = storage(
            connection
                .prepare("SELECT data FROM receipt_transactions ORDER BY created_at DESC,id DESC"),
        )?;
        let rows = storage(statement.query_map([], |row| row.get::<_, String>(0)))?;
        let mut result = vec![];
        for row in rows {
            result.push(decode(&storage(row)?)?);
        }
        Ok(result)
    }
    /// Bounded host/UI listing. The caller must label this as recent results,
    /// not as a complete history or an authoritative spending aggregate.
    pub fn list_recent_transactions(&self, limit: u32) -> Result<Vec<Transaction>> {
        if !(1..=500).contains(&limit) {
            return Err(invalid(
                "Recent transaction limit must be between 1 and 500",
            ));
        }
        let connection = self.connection()?;
        let mut statement = storage(connection.prepare(
            "SELECT data FROM receipt_transactions ORDER BY created_at DESC,id DESC LIMIT ?1",
        ))?;
        let rows = storage(statement.query_map([limit], |row| row.get::<_, String>(0)))?;
        let mut result = vec![];
        for row in rows {
            result.push(decode(&storage(row)?)?);
        }
        Ok(result)
    }
    pub fn list_categories(&self) -> Result<Vec<Category>> {
        let connection = self.connection()?;
        let mut statement =
            storage(connection.prepare("SELECT id,label FROM receipt_categories ORDER BY label"))?;
        let rows = storage(statement.query_map([], |row| {
            Ok(Category {
                id: row.get(0)?,
                label: row.get(1)?,
            })
        }))?;
        rows.map(storage).collect()
    }
}
fn load_draft(connection: &Connection, id: &str) -> Result<ReceiptDraft> {
    let data: Option<String> = storage(
        connection
            .query_row("SELECT data FROM receipt_drafts WHERE id=?1", [id], |row| {
                row.get(0)
            })
            .optional(),
    )?;
    decode(&data.ok_or(ReceiptError::DraftNotFound)?)
}
fn require_category(connection: &Connection, id: &str) -> Result<()> {
    validate_ref(id)?;
    let exists: bool = storage(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM receipt_categories WHERE id=?1)",
        [id],
        |row| row.get(0),
    ))?;
    if !exists {
        return Err(invalid("Choose one of the available categories"));
    }
    Ok(())
}
fn same_payload(saved: &Transaction, request: &SaveReceiptRequest) -> bool {
    saved.draft_id == request.draft_id
        && saved.merchant == request.merchant.trim()
        && saved.date == request.date
        && saved.total_cents == request.total_cents
        && saved.currency == request.currency
        && saved.category_id == request.category_id
}
