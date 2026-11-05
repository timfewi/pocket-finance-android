# Android receipt capture and bundled OCR

This Tauri 2 plugin is called by trusted host Rust. It deliberately exposes no
JavaScript commands and its default permission set is empty. Register
`tauri_plugin_receipt_capture::init()` in the app, import `ReceiptCaptureExt`, and
call `app.receipt_capture()` from the validated host commands.

```rust,ignore
use tauri_plugin_receipt_capture::ReceiptCaptureExt;
let image = app.receipt_capture().capture().await?;
// Or: app.receipt_capture().import_receipt().await?
let result = app.receipt_capture().recognize(&image.file_ref).await;
// On failure, preserve the image and create an editable manual draft.
```

Methods are `capture`, `import_receipt`, `recognize(file_ref)`, `storage_root`,
`discard(file_ref)`, and `cancel`. Non-Android targets report `Unavailable`.
Capture/import returns camelCase `fileRef`, `previewRef`, `width`, `height`,
`mimeType`, `previewWidth`, and `previewHeight`. The two references identify the
same opaque UUID asset, with different renditions. No path, external content URI,
image bytes, or OCR text accompanies capture/import. `recognize` returns `{text}`
only to the Rust host; the host may use it to construct the private editable draft.

## Capture and private files

The external camera receives `ACTION_IMAGE_CAPTURE` with `EXTRA_OUTPUT` pointing
to a FileProvider URI for a temporary full-resolution JPEG. The app uses neither
the thumbnail Intent payload nor an embedded camera preview. The FileProvider is
non-exported, grants access only for the capture, and is scoped to `files/receipts/`.
Camera URI grants are revoked after completion or cancellation. No app CAMERA or
broad media/storage permission is requested.

Import uses AndroidX `PickVisualMedia(ImageOnly)`. AndroidX selects the platform
Photo Picker, a supported fallback picker, or `ACTION_OPEN_DOCUMENT` when needed.
The selected `content:` stream is copied into app-private storage before decoding
or returning its reference. No persistent provider permission is needed afterward.
A cloud-only selection may be unavailable offline; the user can select a local
image or enter the receipt manually.

Native storage is `Activity.filesDir/receipts`, obtained by Rust through
`storage_root()` rather than assuming equivalence with another app path API:

- `<uuid>.original`: unchanged full-resolution imported/captured bytes.
- `<uuid>.preview.jpg`: JPEG preview, maximum edge 1280 pixels, EXIF orientation applied.
- `<uuid>.capture.jpg`: temporary camera output, renamed after successful capture.

Original metadata dimensions describe the encoded image; preview dimensions
reflect EXIF rotation. Native input references must be canonical lowercase UUID
strings. All joined paths are checked against the canonical app-owned directory;
symlinks/path traversal are rejected. The host must independently validate refs,
canonicalize owned paths, and expose only the preview rendition through its scoped
receipt protocol. `storage_root()` is an internal host API, never a frontend
command. `discard` deletes both renditions; the host must reject deletion for a
saved receipt unless its separate record workflow authorizes it.

## OCR, limits, and cancellation

The plugin pins **`com.google.mlkit:text-recognition:16.0.1`**, the bundled Latin
Text Recognition v2 artifact shown in Google's Android guide. It does **not** use
`com.google.android.gms:play-services-mlkit-text-recognition`. The Latin model is
packaged with the app and does not require a first-use model download. Gradle must
fetch the SDK dependencies when building the app; that is distinct from inference
on a device. Supported languages include the Latin-script English/German receipts
used in this milestone; actual Austrian receipt quality requires device tests.

The plugin retains the original, checks a 32 MiB encoded-byte limit and 100 MP
pixel limit, and rejects invalid dimensions before bitmap allocation. OCR uses a
sampled image with maximum edge 4096 pixels and applies EXIF orientation before
`InputImage.fromBitmap`. Preview and OCR sampling are bounded independently.
OCR output is limited to 64,000 characters. One operation runs at a time. OCR has
a 30-second timeout; cancellation/timeout drops late results and always leaves
manual entry available. ML Kit's Task does not provide preemptive cancellation;
its kernel can finish after the app stops waiting. A user must dismiss an external
camera/picker with Android Back; the plugin cannot close another app's activity.
A malicious or broken content provider can stall its input stream; cancellation
prevents result delivery, but provider I/O completion is OS-controlled. Process
or activity destruction can lose an in-flight capture result; physical-device
recovery and orphan cleanup must be validated before release.

No image, OCR text, financial record, URI, exception body, or native response is
logged by this plugin. Rejections contain fixed messages and whitelisted error
codes. Exception objects are never passed to Tauri's `Invoke.reject`, because
that overload logs exceptions. Google SDKs have their own diagnostics policies:
using an on-device SDK alone is not proof of zero network traffic. The app's
**release merged manifest must omit INTERNET and ACCESS_NETWORK_STATE**; inspect
that merged manifest as part of release verification. This plugin declares
neither permission. Debug tooling may use a different manifest and is not an
offline-privacy release configuration.

## Dependencies and verification

Compile SDK 36, minimum SDK 24; AndroidX core 1.16.0, appcompat 1.7.1, activity
1.10.1, exifinterface 1.4.1; ML Kit bundled Latin 16.0.1; JUnit 4.13.2. The Rust
plugin shares the workspace's exact Tauri 2 pin. Its own implementation is MIT;
Google/AndroidX/Tauri dependency licenses and ML Kit terms remain applicable.

Rust tests reject path/URI references. JVM tests cover reference validation,
preview sampling, and dimension limits. These do not prove native camera,
Photo Picker, EXIF rendering, OCR accuracy, cancellation, or offline execution on
physical devices. Native compilation and device validation are reported separately
in the app's verification document. After Tauri Android initialization, run:

```sh
(cd src-tauri/gen/android && ./gradlew :tauri-plugin-receipt-capture:testDebugUnitTest)
```

The Android plugin's Kotlin/Java compilation and all three JVM tests passed in
this development session. That checks native API/dependency compatibility and
the pure validation helpers; it does not execute a camera, Photo Picker, or ML Kit
recognition on a device. Generated plugin build output, copied Tauri API files,
and generated permission schemas are ignored and are not public source artifacts.

## Primary sources checked 2026-11-05

- [ML Kit Android Text Recognition v2: bundled artifact and setup](https://developers.google.com/ml-kit/vision/text-recognition/v2/android)
- [ML Kit terms and privacy](https://developers.google.com/ml-kit/terms)
- [Android full-size camera capture](https://developer.android.com/media/camera/camera-deprecated/photobasics)
- [Android FileProvider scoped sharing](https://developer.android.com/reference/androidx/core/content/FileProvider)
- [Android Photo Picker and AndroidX fallback](https://developer.android.com/training/data-storage/shared/photopicker)
- [Tauri 2.12.5 native plugin Activity callback implementation](https://github.com/tauri-apps/tauri/blob/tauri-v2.12.5/crates/tauri/mobile/android/src/main/java/app/tauri/plugin/PluginHandle.kt)
- [Tauri 2.12.5 Rust mobile plugin API](https://github.com/tauri-apps/tauri/blob/tauri-v2.12.5/crates/tauri/src/plugin/mobile.rs)
