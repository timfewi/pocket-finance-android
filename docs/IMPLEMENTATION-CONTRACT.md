# Receipt integration contract

The public Rust workspace contains `receipt-core`, the native receipt plugin and
the Tauri host. The public category-provider stub is a normal patched dependency,
excluded from workspace membership so a developer can substitute the private
same-name/version library without a Cargo lockfile collision. See
[PRIVATE-AGENT.md](PRIVATE-AGENT.md). Default public builds fetch no private code.

All serialized DTOs use camelCase. IDs are opaque strings; dates use
`YYYY-MM-DD`; EUR amounts use positive integer cents within JavaScript's safe
integer range. The app never accepts arbitrary filesystem paths or SQL from the
frontend. Native capture/import returns an opaque reference. OCR text stays in
private drafts and is never an instruction or log payload.

## Tauri commands

| Command | Arguments | Result |
| --- | --- | --- |
| `list_transactions` | none | Up to 200 most recently saved transactions |
| `list_categories` | none | Valid `{id,label}` category choices |
| `capture_receipt` | none | `{imageRef,previewRef,width,height,mimeType}` |
| `import_receipt` | none | Same image-reference shape |
| `draft_from_receipt` | `{imageRef}` | `ReceiptDraft` with editable fields, OCR, candidates and warnings |
| `suggest_category` | `{draftId}` | `{categoryId,status}`; missing model is nonblocking |
| `save_receipt` | `{request: SaveReceiptRequest}` | The saved `Transaction`; identical retries return it again |
| `discard_receipt` | `{draftId}` | `{discarded:true,imageCleanupPending:boolean}` after draft removal |

Exact Rust types and validation are in [receipt-core](../crates/receipt-core/README.md)
and [host documentation](../src-tauri/README.md). The native `receipt` URI protocol
serves only a bounded preview for a validated opaque reference. No image bytes or
arbitrary local path are returned by capture/import commands. On a narrow screen,
the image is above the editable fields; on wide screens they are side by side.

A save invocation after reviewing a synthetic draft looks like this:

```json
{
  "request": {
    "draftId": "01234567-89ab-4cde-8fab-0123456789ab",
    "merchant": "ALPENMARKT",
    "date": "2026-11-05",
    "totalCents": 1249,
    "currency": "EUR",
    "categoryId": "groceries",
    "confirmed": true
  }
}
```

The draft ID must come from this app's persisted draft; the example ID is not an
existing record. `confirmed:false` fails. The app validates all fields again and
uses the draft ID as a persistent idempotency key. Replaying changed fields after
a successful save fails instead of changing the record or creating a duplicate.
The frontend renders saved status only from a successful store response.

## Text-only extraction entry point

`receipt_core::parse_receipt(ocr_text: &str) -> Result<ReceiptExtraction>` is the
pure Rust entry point for fixed OCR-text fixtures. The store's
`create_draft_with_preview(image_ref, preview_ref, ocr_text)` persists its result.
Empty/unavailable OCR produces a manually editable draft. Reusing an image ID
returns the existing draft without overwriting reviewed fields or saved state.

Recognition of pixels belongs to the Android plugin. The browser fixture bridge
uses synthetic text and the real Rust store; it does not claim image recognition.
See [VERIFICATION.md](VERIFICATION.md) for its commands and evidence boundary.

## Optional category provider

Both public and private implementations expose:

```rust
pub struct CategoryChoice { pub id: String, pub label: String }
pub struct CategorySuggestion {
    pub category_id: Option<String>,
    pub status: SuggestionStatus,
}
pub enum SuggestionStatus { Suggested, Unavailable, Failed }
pub async fn suggest_category(
    ocr_text: &str,
    categories: &[CategoryChoice],
) -> CategorySuggestion;
```

The public stub returns `{"categoryId":null,"status":"unavailable"}`. The private
library reuses the agent's model-backend boundary and validates the exact category
allowlist. The host validates it again and bounds response time. A late suggestion
must not replace a manual choice; applying one clears prior confirmation. The
provider never supplies monetary fields or saves records. Android may have no
local model backend installed, and saving still works in that state.
