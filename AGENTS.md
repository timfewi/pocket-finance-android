# Pocket Finance Android

This is a new public, MIT-licensed Android app: Tauri 2, Svelte 5, SvelteKit static SPA, TypeScript and shadcn-svelte. Keep a single app. The first milestone is an Austrian receipt capture/import → bundled on-device Latin OCR → Rust draft → explicit review → local SQLite transaction.

Privacy and source boundaries:
- Do not copy private repository implementation, real receipts, OCR, credentials, model weights, or build artifacts into this public repository.
- `pocket-finance-agent` stays private. Default public builds must require no private repository access. A documented Cargo patch can substitute an API-compatible private category-provider library when explicitly selected.
- Never log receipt images, OCR text, model prompts or financial records. Synthetic fixtures only in tests.
- Rust owns amount/date/category validation. Money is EUR integer cents. Cash tendered and change are not the total. Ambiguous totals require user selection or entry.
- Saving requires explicit UI confirmation. A persistent draft ID/idempotency key must prevent repeated saves.
- OCR/model failures never disable manual editing and saving. No model may commit a transaction.

Paths are repository-relative. Generated files belong under ignored artifacts/ or target/. Never document internal executor paths. Coordinate file ownership before parallel edits. Use official Tauri/Android interfaces; report native build and physical-device verification separately from Rust/browser tests.
