# Mobile-first verification

The phone is the primary design target. One Svelte interface expands from bottom
navigation on phones/tablets to a sidebar from 1024px. The app keeps the same
overview, capture/review and receipt-list functions at every size.

Implemented behavior:

- Labelled bottom navigation: Übersicht, Erfassen, Belege; active section and draft indicator.
- Draft edits survive section/brand navigation within the running app.
- Camera/import choices remain reachable after saving the first receipt.
- At least 48px action targets and 16px form inputs; narrow forms use one column.
- Long names/large amounts wrap; bottom padding keeps the last control clear of navigation.
- Photo/discard dialogs fit the available viewport and have accessible close/confirm controls.
- Android applies system-bar, display-cutout and keyboard insets to WebView margins;
  overlapping bottom insets use their maximum, not their sum. Applied insets are consumed.
- Short-view editing hides bottom navigation and keeps the focused field visible.

## Executed checks

On the Linux host, `npm run check` reported zero errors/warnings and `npm run build`
produced the static SPA. Chromium with the real local Rust/SQLite fixture bridge
passed **11 browser tests**; three duplicated desktop geometry cases were
deliberately skipped because their width matrix runs once in the mobile project.

The checks cover 320, 360, 393, 412, 768 and 1365 CSS pixel widths, a long synthetic
merchant, a large valid EUR amount, touch-target bounds, input font size, dialog
width, navigation/unsaved-edit retention, actual fixture saving and the existing
1249-cent, ambiguity, missing-model, failed-OCR and duplicate-save cases. A
393×440 viewport simulation verifies focused-field visibility; it does not open
an Android keyboard. Desktop/narrow screenshots were inspected separately.

Native Kotlin compiled and **3 new JVM tests passed** for system/keyboard inset
overlap, keyboard dismissal without accumulated margins, and landscape cutout/side
navigation insets. Run after Android build setup:

```sh
scripts/android-env.sh src-tauri/gen/android/gradlew -p src-tauri/gen/android \
  :app:testUniversalDebugUnitTest -x :app:rustBuildUniversalDebug \
  --no-daemon --max-workers=2
```

Reproduce browser checks using two terminals:

```sh
cargo run --locked -p receipt-core --example receipt_fixture_bridge -- 8787
```

```sh
RECEIPT_BRIDGE_URL=http://127.0.0.1:8787 npm run test:e2e
```

## Still requires a device

No physical Seeker/Android phone is attached. Check real keyboard show/hide,
gesture/three-button navigation, cutouts, portrait/landscape, Android font/display
scaling, camera return, accessibility services and first-launch airplane-mode OCR.
No claim of pixel-perfect Seeker rendering or full accessibility compliance is
made from browser and JVM tests. Native desktop capture/import/OCR remain
unimplemented; desktop-width browser operation uses labelled synthetic fixtures.
