# Receipt milestone verification — 2026-11-05

This report separates synthetic OCR-text checks, actual browser behavior, native
compilation and physical-device behavior. No real receipt or personal financial
record was used. No camera/Photo Picker/ML Kit execution on a phone is claimed.

[BUILD-SOURCE-SHA256SUMS](BUILD-SOURCE-SHA256SUMS) records the published source/input
snapshot, including dependency locks. It is not a guarantee of bit-for-bit APK
reproduction across machines or signing keys. Two generated Gradle files had
trailing blank lines removed after packaging; no executable behavior changed.

## Executed checks

| Check | Observed result |
| --- | --- |
| Public receipt-core Rust tests | 17 passed |
| Public no-model category-provider tests | 3 passed |
| Standalone host asset-security Rust tests | 3 passed |
| Native receipt-file JVM tests | 3 passed; Kotlin/Java plugin compiled |
| Svelte/TypeScript checks | 0 errors, 0 warnings |
| Static SPA production build | Passed |
| Public Android ARM64 APK builds | Debug and unsigned release passed; packaged Latin models and release permission policy inspected |
| Chromium browser + real Rust/SQLite fixture bridge | 6 passed across desktop and Pixel 7 viewport |
| Private category provider unit tests | 7 passed; private crate Clippy passed |
| Private provider Android ARM64 cross-check | Passed; desktop HTTP runtime excluded on Android |
| Private-provider Android ARM64 debug APK | Built and signature verified; host resolves to the private library, public lockfile/APKs preserved |
| Physical Android / Solana Seeker | Not executed; `adb devices -l` returned no devices |

The browser tests use real rendered Svelte components and the real Rust parser and
SQLite store through a test-only localhost bridge. The images and OCR strings are
fixed synthetic fixtures. The bridge does not execute camera capture or
image-to-text OCR. Pixel 7 is a browser viewport configuration, not a connected
physical Pixel or Seeker. Desktop and narrow screenshots were inspected for
legibility, hierarchy and clipping; this is not a full accessibility audit.

Cases covered include:

- `12,49 €` labelled total, `20,00 €` cash tendered and `7,51 €` change become a
  saved transaction of exactly **1249 cents**.
- Two distinct labelled totals leave the amount blank until reviewed; selecting
  `14,49 €` saves **1449 cents** and clears any earlier confirmation.
- Missing model leaves category selection and saving usable.
- Double click and direct replay of an identical save create one transaction.
- Different replay payloads are rejected; same-image draft creation is idempotent
  even through independent SQLite connections.
- OCR failure leaves a manual draft; saving and confirmed discard work.
- Foreign currencies are not automatically converted; amount/date/overflow,
  category, opaque-reference, symlink and preview-size checks are exercised.

## Reproduce the public checks

Run from the repository root with the pinned toolchain and npm dependencies:

```sh
npm ci
cargo test -p receipt-core --locked
cargo test --manifest-path crates/category-provider/Cargo.toml --locked
cargo clippy -p receipt-core --all-targets --locked -- -D warnings
mkdir -p target
rustc --edition=2021 --test src-tauri/src/security.rs -o target/receipt-security-tests
target/receipt-security-tests
npm run check
npm run build
```

For browser integration, use two terminals from the repository root:

```sh
# Terminal 1: synthetic-only test service with a local fixture SQLite database.
cargo run -p receipt-core --example receipt_fixture_bridge -- 8787
```

```sh
# Terminal 2: install the test browser once, then run the real browser checks.
npx playwright install chromium
RECEIPT_BRIDGE_URL=http://127.0.0.1:8787 npm run test:e2e
```

The bridge binds only loopback and accepts a bounded set of fixture commands. Its
`/__test/reset` endpoint clears only `artifacts/browser-fixtures/receipts.sqlite`;
never use that database for actual receipts. Test reports and screenshots are
written under ignored `artifacts/`. The ordinary browser demo without this bridge
uses in-memory synthetic responses and is not evidence of Rust persistence.
The development bridge branch is absent from production frontend chunks.

Native plugin compilation/tests use the generated Android project and installed
SDK/JDK/NDK. The complete commands and final APK status are recorded in
[ANDROID.md](ANDROID.md). Android artifacts must be built from the final source;
an earlier successful dependency build is not the final app artifact.
The final public release APK is 30,296,079 bytes; the signed development APK is
168,747,978 bytes. Hashes and packaged checks are preserved in
[ANDROID-BUILD-VERIFICATION.json](ANDROID-BUILD-VERIFICATION.json). The optimized
release APK is unsigned, and must be signed before installation or distribution.

## Private provider observations

The local patch was verified against both public and private libraries, including
child Cargo discovery from the wrapper's `run` mode. Success, error and handled
signal paths restore the public lockfile and remove temporary configuration.
An existing local Cargo config is preserved rather than overwritten.

One real Linux Qwen3-0.6B Q8_0 smoke returned the allowed category `household` for a
synthetic bread-and-apples grocery receipt. This is a **wrong category**, not an
accuracy success. Missing-model execution returned `unavailable`. The experiment
verifies the optional library/inference path and fallback only. The user must
review a suggestion; money and confirmation never depend on it.

The private provider has an injectable backend boundary, but this app milestone
contains no Android GGUF runtime or bundled Qwen weights. Public builds use the
no-model provider. A private library build on Android also remains nonblocking
when no native model backend is installed. See [PRIVATE-AGENT.md](PRIVATE-AGENT.md).
The private-provider debug APK is 177,282,442 bytes, SHA-256
`12709abef0326bb018bc02e604b89a83eb695dacb3851d5e8b0547aa3cf84ab8`.
Cargo resolution and the packaged native-library hash were checked against the
private build. The signature verifies; no model runtime or physical-device use is
implied. [PRIVATE-BUILD-VERIFICATION.json](PRIVATE-BUILD-VERIFICATION.json) records
the build command and restoration checks without private source or host paths.

## Remaining device acceptance

On a fresh installation, switch the phone into airplane mode **before the first
launch**, then photograph a synthetic Austrian receipt. Check the full-resolution
original, preview orientation, OCR draft, editing, selected amount, category,
explicit save, duplicate tap, transaction visibility and persistence after app
restart. Repeat with an imported image, a cancelled picker, denied/unavailable
camera, unclear totals and unreadable text. Do not use `tauri android dev` for
this offline acceptance test; it normally needs the live development server.

Record phone/Android version and APK checksum. Camera applications, Photo Picker,
ML Kit first initialization, process death, rotation, permission denial, sustained
memory and battery behavior remain unverified here. App-private storage has no
implemented encrypted database or export/restore interface. Saved-receipt deletion
and automatic orphan-file cleanup are also outside this milestone.
