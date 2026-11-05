# Android build and device verification

The public app builds without the private finance-agent repository or model
weights. Native receipt capture/import and bundled Latin OCR are provided by
`plugins/receipt-capture`; Rust validates drafts and stores explicitly confirmed
transactions locally. Category-model availability must never block manual entry.

## Install the published test APK

The [v0.1.0-alpha.2 release](https://github.com/timfewi/pocket-finance-android/releases/tag/v0.1.0-alpha.2)
provides a **public ARM64 debug APK built in GitHub Actions**, signed with a
development key, plus `SHA256SUMS` and build provenance. Download it on an ARM64 Android 7.0+ phone
and allow installation from the browser when prompted. No local build is needed.
This is an alpha test build; physical-device verification is still pending. Its
debug manifest retains Internet permission. Test with synthetic receipts and turn
on airplane mode before first launch to verify bundled OCR startup offline.
Future signing-key changes may require uninstalling the app, deleting local data.
Each fresh CI runner generates a development key; stable production signing is
not configured. The release's `BUILDINFO.json` identifies its source commit,
workflow run, checksum and certificate. These differ from the historical local
build artifacts recorded below.

Maintainers can run the **Publish Android test APK** workflow from GitHub Actions with
an unused tag such as `v0.1.0-alpha.3`. It checks and builds the public app, verifies
its signature/alignment, uploads the APK and checksum, verifies the uploaded bytes,
and publishes a prerelease. It refuses to replace an already published release.
No private repository access or production signing secret is required.
If publication fails after a successful build, rerun the workflow with the same
tag and `artifact_run_id` set to that run's numeric ID (within its seven-day
artifact retention). The workflow validates the source run and artifact metadata,
then publishes those exact bytes without rebuilding or changing the signing key.
If GitHub denies release-metadata edits to the workflow token, a maintainer can
prepare the matching draft (tag, source commit and exact `release.md` notes), then
select `upload_only` together with `artifact_run_id`. That mode uploads and verifies
the assets while leaving the prepared draft for the maintainer to publish.

## Toolchain

The Android project is generated with Tauri CLI 2.12.4 and maintained under
`src-tauri/gen/android`. The build uses:

| Component | Version |
| --- | --- |
| Rust | 1.99.1; `aarch64-linux-android` target |
| Java JDK | Eclipse Temurin 21.0.13+9 (a JDK with `javac`, not only a JRE) |
| Android command-line tools | Revision 23 |
| Android platform | API 36, revision 2 |
| Android build tools | 36.0.0 |
| Android platform tools | 37.0.1 |
| Android NDK | 29.0.14206865 |
| Gradle wrapper | 9.6.1; distribution SHA-256 pinned in wrapper properties |
| Android Gradle plugin | 9.3.1 |
| Kotlin Gradle plugin | 2.2.10 |
| Minimum Android API | 24 |
| Compile/target Android API | 36 |
| Bundled Latin OCR | `com.google.mlkit:text-recognition:16.0.1` |

The current Tauri template generated compile/target API 37, but the official SDK
repository did not offer `platforms;android-37` during verification. The generated
project deliberately pins API 36 and its installed build tools. Do not silently
regenerate the native tree and lose these pins or the release manifest overlay.

## Install prerequisites

Use Android Studio's SDK Manager or Google's official command-line tools. Install
JDK 21, Node/npm and the pinned Rust toolchain. Example shell configuration:

```sh
export JAVA_HOME=/path/to/jdk-21
export ANDROID_HOME="$HOME/Android/Sdk"
export PATH="$JAVA_HOME/bin:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
sdkmanager --licenses
sdkmanager 'platform-tools' 'platforms;android-36' 'build-tools;36.0.0' 'ndk;29.0.14206865'
rustup target add --toolchain 1.99.1 aarch64-linux-android
npm ci
```

`scripts/android-env.sh` checks the JDK/SDK/NDK and exports their paths for one
command. It does not download SDKs, accept licenses, configure credentials or
change the user's global environment. Override `NDK_HOME` only when intentionally
testing another supported NDK. Keep SDKs, downloaded distributions, signing keys,
Gradle caches and APKs outside Git.

Build dependency downloads require network access. In a proxy environment,
configure Gradle/Java proxy properties and the organization's trusted CA normally;
never disable TLS verification. Do not put proxy credentials in tracked Gradle
properties. Android SDK/NDK and ML Kit have third-party terms independent of this
repository's MIT license; install them from official sources and review their
terms. SDK binaries are not redistributed by this repository.

## Build and install

A normal checkout already contains the Android project:

```sh
scripts/android-env.sh npm run tauri -- android build --debug --target aarch64 --apk --ci
```

Use `--debug` for a locally signed development APK. Omit it for a release build;
release signing needs your own key/configuration and must follow the official
Tauri Android signing guide. Never commit a keystore, password or signing file.
The command above creates the following debug APK in a normal checkout. The
directory is named `universal` by the Tauri template; this build contains only
ARM64 because `--target aarch64` was selected. Enable USB debugging on the Seeker,
connect it and authorize the laptop, then install:

```sh
scripts/android-env.sh adb devices -l
scripts/android-env.sh adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
scripts/android-env.sh adb shell am start -n io.github.timfewi.pocketfinance/.MainActivity
```

To use live development with an attached device, enable USB debugging, authorize
the host, then run `scripts/android-env.sh npm run tauri -- android dev --target
aarch64`. Debug manifests retain Internet permission for the local development
server. This is separate from the release permission policy.

## Offline release policy

`app/src/release/AndroidManifest.xml` removes `INTERNET` and
`ACCESS_NETWORK_STATE` permissions, including any introduced by transitive SDKs.
Bundled ML Kit Latin recognition needs no runtime model download. The system
camera and photo picker are separate Android components; a user-selected media
provider may have its own network behavior. This app's release process must inspect
the **merged packaged manifest**, not merely the source overlay:

```sh
"$ANDROID_HOME/build-tools/36.0.0/aapt" dump permissions src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
```

The app sets `allowBackup=false` and `fullBackupContent=false`. Android 12+
`dataExtractionRules` exclude app files, databases, preferences, external app
storage and device-protected data from cloud backup and device transfer. Device
behavior still needs acceptance testing; this is the declared package policy.

The plugin uses a nonexported, scoped `FileProvider`, external camera intent and
system photo picker/document picker. It does not request broad storage access.
Runtime images, OCR and financial records must not be included in logs or tests.

## Verification status

On 2026-11-05, the public default app successfully cross-compiled on Linux x86-64
to both an Android ARM64 debug APK and an optimized release APK. Native Kotlin/Java
compilation and all three `ReceiptFilesTest` JVM tests passed. The release APK was
inspected after packaging: it has no Internet/network-state permission, disables
backup, includes cloud/device-transfer exclusions, and exposes only the scoped
nonexported receipt provider. Bundled Latin model assets and the OCR native library
are present. Both ARM64 native libraries have 16 KiB ELF LOAD alignment; ZIP
alignment checks passed. These are package checks, not 16 KiB device runtime tests.

`adb devices -l` returned an empty list: no physical Android phone or Solana Seeker
is connected. Installation, camera, actual OCR recognition, Android SQLite
persistence, process recreation, rotation, permission denial and thermal/battery
performance remain unverified on a device.

The preserved build artifacts are ignored by Git:

| Artifact under `artifacts/apk/public/` | Bytes | Signing |
| --- | ---: | --- |
| `pocket-finance-public-arm64-debug.apk` | 168,747,978 | Development key; v2 signature verified |
| `pocket-finance-public-arm64-release-unsigned.apk` | 30,296,079 | Unsigned; sign before installation/distribution |

Debug SHA-256: `b5825b6df49e8185ff7441ec0d01c318a3dd941dbd6058214011bbe916d71083`.

Release SHA-256: `a1fc4628a7606fb0e60e3d03aa52aadd06ef1de44902e74f69da6449822b438a`.

[ANDROID-BUILD-VERIFICATION.json](ANDROID-BUILD-VERIFICATION.json) records build
commands and package checks. The archived APK copies are not downloaded by cloning
this repository; reproduce them with the commands above, or use an APK separately
provided for testing. Normal builds write the generated Android paths shown above.
The debug artifact is larger because it retains
debug symbols and keeps development network permissions. These artifact names do
not imply publication or a production signing identity.

Reproduce the native unit tests after the first Tauri Android build generates its
local Gradle integration:

```sh
scripts/android-env.sh src-tauri/gen/android/gradlew -p src-tauri/gen/android \
  :tauri-plugin-receipt-capture:testDebugUnitTest --no-daemon --max-workers=3
```

The generated Tauri/Wry build currently emits deprecation warnings for legacy
Android/Gradle APIs and Java 8 bytecode targeting. The pinned toolchain builds;
Android Gradle plugin 10 and future toolchain upgrades are unverified.

A device acceptance run should use synthetic receipts and verify capture, picker
cancel/error handling, Latin OCR with Austrian comma-decimal amounts, explicit
total selection, category/manual edits, duplicate-save prevention, persistence
across restart and a complete receipt-to-transaction flow in airplane mode. Failed
OCR must leave manual saving usable. Record the phone model, Android version, app
build hash and observed outcomes separately from Linux/browser/Rust tests.

## Primary references

- [Tauri Android prerequisites](https://v2.tauri.app/start/prerequisites/#android)
- [Tauri Android signing](https://v2.tauri.app/distribute/sign/android/)
- [Android command-line tools](https://developer.android.com/tools)
- [Android SDK Manager](https://developer.android.com/tools/sdkmanager)
- [Bundled ML Kit text recognition](https://developers.google.com/ml-kit/vision/text-recognition/v2/android)
- [Android SDK terms](https://developer.android.com/studio/terms)
- [Temurin JDK releases](https://adoptium.net/temurin/releases/)
