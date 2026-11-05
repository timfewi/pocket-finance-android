set shell := ["bash", "-euo", "pipefail", "-c"]

check:
    cargo test -p receipt-core --locked
    cargo test --manifest-path crates/category-provider/Cargo.toml --locked
    npm run check
    npm run build

fixture-bridge:
    cargo run -p receipt-core --example receipt_fixture_bridge -- 8787

browser-test:
    RECEIPT_BRIDGE_URL=http://127.0.0.1:8787 npm run test:e2e

android-debug:
    scripts/android-env.sh npm run tauri -- android build --debug --target aarch64 --apk --ci

android-dev:
    scripts/android-env.sh npm run tauri -- android dev --target aarch64
