# Native app host

Tauri 2.12.5 is pinned in the workspace; `Cargo.lock` records all resolved
versions. Rust 1.99.1 is pinned for the reproducible Android toolchain. The host
uses the bundled native receipt plugin and public no-model category provider by
default. A documented local Cargo patch can replace the category provider; the
public dependency graph contains no private Git source.

The frontend invokes these camelCase argument shapes:

| Command | Arguments | Result |
| --- | --- | --- |
| `list_transactions` | none | Most recent 200 transactions |
| `list_categories` | none | Stored category catalog |
| `capture_receipt` | none | `imageRef`, `previewRef`, dimensions and MIME type |
| `import_receipt` | none | Same captured-receipt shape |
| `draft_from_receipt` | `{ imageRef }` | Persisted editable `ReceiptDraft` |
| `suggest_category` | `{ draftId }` | Optional validated suggestion/status |
| `save_receipt` | `{ request: SaveReceiptRequest }` | Confirmed saved transaction |
| `discard_receipt` | `{ draftId }` | `{ discarded: true, imageCleanupPending }`; saved drafts cannot be discarded |

Receipt references are canonical UUIDs, never paths or content URIs. The webview
uses `convertFileSrc(previewRef, 'receipt')` for the custom receipt protocol. Its
handler serves only the matching app-private JPEG preview, with a 4 MiB cap,
strict UUID/path checks, symlink rejection, and no-store headers. Original images
are retained separately at their original resolution and checked against a
32 MiB host limit; the native plugin applies its own image/decode limits. Image
bytes never cross an invoke JSON payload. The optional asset protocol is also
scoped to the app's receipts directory; no filesystem or shell plugin is exposed.

SQLite opens under the app-specific data directory. Opening, parsing, and DB
operations use at most four blocking workers; preview reads use at most two.
Listing is bounded to the newest 200 records. A receipt lifecycle mutex prevents
draft creation from racing discard/image cleanup. The core enforces one persisted
draft per image, explicit confirmation, version-free immutable saved receipts,
and idempotency keyed by draft ID.

OCR is capped at 25 seconds and 32 KiB of text. Failure produces an editable
empty-text draft while retaining its image. Category suggestions have a separate
three-second timeout and are checked against the stored category IDs. Provider
unavailability or failure never authorizes a save and never disables manual
editing. No receipt text, image, prompt, or financial record is logged by the host.

Discard first removes an unsaved DB draft, then asks the native plugin to delete
its owned image renditions. A failed cleanup returns `imageCleanupPending: true`,
allowing the UI to dismiss the discarded draft and show an honest warning.
Process termination or a failed native deletion can leave an orphaned
image; automatic orphan collection is not implemented. Saved receipt images are
never deleted by this command. Android backup/cloud/device-transfer exclusion
rules live in the generated Android application owned by the build setup.

The headless public workspace defaults to `receipt-core`, so `cargo test --locked`
does not require desktop GTK. Validate the standalone category provider with
`cargo test --manifest-path crates/category-provider/Cargo.toml`. Android host
build commands are in `docs/ANDROID.md`. Browser preview uses an explicit synthetic
demo or a test-only fixture bridge; it does not claim to exercise Android camera,
ML Kit, or the custom image protocol.

Standalone host asset-security tests, without GTK or an Android device:

```sh
mkdir -p artifacts
rustc --edition=2021 --test src-tauri/src/security.rs -o artifacts/host-security-tests
artifacts/host-security-tests
```

The included `icons/icon.png` and Android launcher renditions are generated from
the public app's `static/favicon.svg` receipt mark.
