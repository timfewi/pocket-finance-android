# Privacy and network behavior

## Local receipt processing

The app stores its receipt originals/previews and SQLite database in Android's
app-private storage. Capture delegates to the system camera with a temporary
FileProvider URI grant. Import uses the Android Photo Picker and copies the chosen
image into app-private storage; the app does not request broad photo-library
access. The picker and camera are Android/system components, with their own
behavior outside this application's control.

Receipt images travel through the frontend bridge as opaque references, not
base64 originals. A narrowly scoped local receipt protocol supplies the preview.
OCR text is untrusted data, not instructions. The native and Rust layers impose
size limits and return sanitized error codes. No receipt image, merchant, amount,
OCR text, model prompt or raw model response belongs in application logs.

The first milestone has no account login, cloud database, synchronization,
analytics exporter, hosted OCR or remote inference API. An expense is saved only
after the user's explicit review. Repeating the same confirmed draft save returns
the existing transaction; a different payload for an already-saved draft fails.

Discarding an unsaved draft removes the draft and requests deletion of its
app-owned image through the native layer. If image cleanup fails, the UI reports
that separately; an orphan image can remain in private storage. Automatic orphan
cleanup and recovery after process death are not implemented in this milestone.
Saved receipts remain with their transactions. Android backup is explicitly
disabled, with cloud-backup and device-transfer extraction exclusions. A complete saved-record deletion,
backup/export and migration interface is not part of this first milestone.
Android uninstall normally removes app-private data. Do not promise recovery
without a separately implemented backup system.

## Bundled ML Kit Latin OCR

The native plugin uses the bundled artifact
`com.google.mlkit:text-recognition:16.0.1`, not the Play-services unbundled model
artifact. The Latin recognition model is included in the Android build. According
to Google's documented behavior, the bundled option is available immediately and
recognition runs on-device; it does not require a first-use model download. The
app makes no request to a hosted OCR endpoint.

Bundled/on-device recognition is not the same as a Google-free dependency. ML Kit
has Google's SDK terms and documents collection of SDK/device usage and diagnostic
information. Review those terms and the data-disclosure guide before distribution;
do not describe the SDK as telemetry-free merely because recognition is local.
The release manifest removes `INTERNET` and `ACCESS_NETWORK_STATE` permissions
from the app's merged manifest, preventing ordinary app-UID network requests.
Debug builds retain network access for Tauri's development server. The optional
private provider's desktop loopback request is local inference, not hosted
inference; Android defaults to unavailable until a native backend is installed.

No network packet capture or fresh-install airplane-mode test on a physical
Android device has been completed unless explicitly recorded in
[VERIFICATION.md](VERIFICATION.md). Build success and fixed OCR-text fixtures do
not verify camera hardware, Photo Picker integration, SDK initialization or
recognition quality on a phone. This boundary also applies to any SDK/system
component telemetry outside the app's own network permission.

Primary SDK references:

- [Text recognition v2 on Android, bundled versus unbundled](https://developers.google.com/ml-kit/vision/text-recognition/v2/android)
- [ML Kit Android data disclosure](https://developers.google.com/ml-kit/android-data-disclosure)
- [ML Kit terms and privacy](https://developers.google.com/ml-kit/terms)
- [Android Photo Picker](https://developer.android.com/training/data-storage/shared/photopicker)

## Models and public/private sources

Public builds use a no-model category provider and require no private repository.
A developer can locally select the private library through the documented Cargo
patch. It accepts bounded OCR text and the allowed category IDs only; returned IDs
are validated. Suggestions never change authoritative receipt totals or save
records. Missing, malformed or timed-out model results leave manual category
selection available. No weights or private implementation are included in this
public repository.

Dependency acquisition (npm, Cargo, Android SDK/Gradle/Maven and optional model
downloads) uses the network during development/setup. Those downloads are distinct
from receipt processing at runtime. Check licenses and packaged dependency notices
before shipping a release APK.
