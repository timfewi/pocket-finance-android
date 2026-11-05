#!/bin/sh
# Run a command with the pinned Android toolchain. SDKs/JDKs are installed separately.
set -eu
: "${ANDROID_HOME:=${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
: "${NDK_HOME:=$ANDROID_HOME/ndk/29.0.14206865}"
: "${JAVA_HOME:?Set JAVA_HOME to a JDK 21 installation (not a JRE)}"
if [ ! -x "$JAVA_HOME/bin/javac" ]; then
  echo 'JAVA_HOME must contain bin/javac; install JDK 21.' >&2
  exit 1
fi
if [ ! -x "$ANDROID_HOME/platform-tools/adb" ] || [ ! -d "$NDK_HOME/toolchains/llvm" ]; then
  echo 'Install Android SDK platform-tools and NDK 29.0.14206865; set ANDROID_HOME/NDK_HOME.' >&2
  exit 1
fi
export JAVA_HOME ANDROID_HOME NDK_HOME
ANDROID_SDK_ROOT="$ANDROID_HOME"
ANDROID_NDK_HOME="$NDK_HOME"
PATH="$JAVA_HOME/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
export ANDROID_SDK_ROOT ANDROID_NDK_HOME PATH
if [ "$#" -eq 0 ]; then
  echo 'Usage: JAVA_HOME=/path/to/jdk ANDROID_HOME=/path/to/sdk scripts/android-env.sh npm run tauri -- android build --target aarch64 --apk' >&2
  exit 2
fi
exec "$@"
