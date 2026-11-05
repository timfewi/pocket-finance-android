# Receipt core

Public Rust library for the first Austrian receipt workflow. It depends only on public crates and never invokes a model, camera, shell, network, or receipt file reader. The native host chooses the SQLite database path and verifies app-owned image references before passing opaque IDs here. All test receipts are synthetic.

`parse_receipt(ocr_text)` produces editable fields, labelled amount candidates and review warnings. A positive labelled gross total is preferred over subtotal, tax-only, discount, cash tendered or change. A total label may use the immediately following amount-only line; the parser never skips through payment lines. Conflicting totals, malformed amounts and non-EUR currencies leave the automatic total empty. Merchant detection is only a heading heuristic and is always marked for review. Dates accept exact `DD.MM.YYYY` or `YYYY-MM-DD` and are returned as ISO dates. Missing currency produces an explicit warning; EUR is the supported app currency, not a claim that EUR was printed.

Money uses checked integer cents, bounded by JavaScript's exact integer range. `parse_eur_amount` accepts decimal comma or decimal point with exactly two fraction digits, compatible grouped thousands, or whole euros. Ambiguous values such as `1.234` are rejected. There is no FX conversion, inferred tax or addition of line items.

`ReceiptStore::open` initializes a local EUR account and an allowlist of categories. `create_draft` and `create_draft_with_preview` persist incomplete drafts, including empty OCR for manual fallback. `update_draft` persists edits without committing a transaction. `get_draft` retrieves a draft after restart. OCR text remains in the local draft data and must never be logged. SQLite is not encrypted by this crate; the native app must use private app storage and its Android backup policy.

An image reference identifies exactly one retained draft. Repeated creation returns that existing draft, including saved status, and never replaces OCR, preview identity or edited fields. A unique SQLite index and an immediate write transaction enforce this across connections. The native host must still serialize creation/discard/file cleanup because database and filesystem operations cannot form one SQLite transaction. Pre-release version-1 databases containing duplicate image references fail migration safely; no records are silently deleted or merged.

`save_receipt` requires `SaveReceiptRequest.confirmed == true` plus a valid merchant, date, positive total, EUR and an existing category. The UI must show the exact edited values before setting that flag. The draft ID is the persistent idempotency key. Save inserts a transaction and marks the draft saved in one SQLite write transaction. Repeating the same canonical payload returns the same transaction; changing a saved payload returns `ConflictingSave`. Saved drafts cannot be edited or discarded through draft methods. Models have no commit interface.

The serializable API uses camelCase and rejects unknown request fields. Frontend callers receive opaque `imageRef`/`previewRef` IDs, never paths or SQL. Errors contain no OCR text, images, file paths or database implementation details. `warnings` and OCR text are untrusted display strings and must be rendered as text.

Run from the repository root:

```sh
cargo test -p receipt-core
cargo clippy -p receipt-core --all-targets -- -D warnings
```

Tests cover cash/change discrimination, neighbouring OCR lines, inclusive-tax labels, ambiguity, locale parsing, missing fields, invalid dates/currency/amounts, integer overflow, explicit confirmation, persisted edits, restart persistence and duplicate/conflicting save retries. Parser accuracy beyond these synthetic fixtures and Android/device SQLite execution require separate verification.
