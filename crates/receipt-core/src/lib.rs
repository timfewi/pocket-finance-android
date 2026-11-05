//! Public, offline receipt extraction and persistence. This crate has no camera,
//! model, network, or arbitrary-file access capability. All receipt text is private.
mod parser;
mod store;
pub use parser::{parse_eur_amount, parse_receipt};
pub use store::ReceiptStore;

use serde::{Deserialize, Serialize};

pub const MAX_SAFE_CENTS: i64 = 9_007_199_254_740_991;
pub const MAX_OCR_BYTES: usize = 32_768;

#[derive(Debug, thiserror::Error)]
pub enum ReceiptError {
    #[error("{0}")]
    Validation(String),
    #[error("Receipt draft was not found")]
    DraftNotFound,
    #[error("Explicit confirmation is required before saving")]
    ConfirmationRequired,
    #[error("This receipt was already saved with different details")]
    ConflictingSave,
    #[error("A saved receipt cannot be edited or discarded as a draft")]
    DraftAlreadySaved,
    #[error("Local receipt storage is unavailable")]
    Storage,
}
pub type Result<T> = std::result::Result<T, ReceiptError>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AmountCandidate {
    pub id: String,
    pub label: String,
    pub amount_cents: i64,
    pub line_index: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiptExtraction {
    pub merchant: Option<String>,
    pub date: Option<String>,
    pub total_cents: Option<i64>,
    pub currency: String,
    pub amount_candidates: Vec<AmountCandidate>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DraftStatus {
    Draft,
    Saved,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiptDraft {
    pub id: String,
    pub image_ref: String,
    pub preview_ref: Option<String>,
    pub ocr_text: String,
    pub merchant: Option<String>,
    pub date: Option<String>,
    pub total_cents: Option<i64>,
    pub currency: String,
    pub category_id: Option<String>,
    pub amount_candidates: Vec<AmountCandidate>,
    pub warnings: Vec<String>,
    pub status: DraftStatus,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiptEdits {
    pub merchant: Option<String>,
    pub date: Option<String>,
    pub total_cents: Option<i64>,
    pub currency: String,
    pub category_id: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveReceiptRequest {
    pub draft_id: String,
    pub merchant: String,
    pub date: String,
    pub total_cents: i64,
    pub currency: String,
    pub category_id: String,
    pub confirmed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Transaction {
    pub id: String,
    pub draft_id: String,
    pub merchant: String,
    pub date: String,
    pub total_cents: i64,
    pub currency: String,
    pub category_id: String,
    pub image_ref: String,
    pub created_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Category {
    pub id: String,
    pub label: String,
}

fn invalid(message: &str) -> ReceiptError {
    ReceiptError::Validation(message.to_owned())
}
fn validate_text(value: &str, field: &str, max: usize) -> Result<()> {
    if value.trim().is_empty() || value.chars().count() > max || value.chars().any(char::is_control)
    {
        return Err(invalid(&format!(
            "{field} must contain 1–{max} printable characters"
        )));
    }
    Ok(())
}
fn validate_ref(value: &str) -> Result<()> {
    validate_text(value, "Receipt reference", 160)?;
    if !value
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || b"-_:".contains(&c))
    {
        return Err(invalid("Receipt reference must be an opaque identifier"));
    }
    Ok(())
}
fn validate_date(value: &str) -> Result<()> {
    let date = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| invalid("Enter a valid date as YYYY-MM-DD"))?;
    if date.format("%Y-%m-%d").to_string() != value
        || !("1900-01-01"..="2200-12-31").contains(&value)
    {
        return Err(invalid(
            "Enter a valid date between 1900 and 2200 as YYYY-MM-DD",
        ));
    }
    Ok(())
}
fn validate_cents(value: i64) -> Result<()> {
    if !(1..=MAX_SAFE_CENTS).contains(&value) {
        return Err(invalid(
            "Receipt total must be positive EUR cents within the supported integer range",
        ));
    }
    Ok(())
}
