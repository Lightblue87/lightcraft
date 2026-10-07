#!/bin/sh
# Build the Android cdylib, package a NativeActivity APK (no Gradle: aapt2 + zipalign + apksigner,
# signed with a throwaway debug key) and, with --run, install and start it on the adb device.
# Needs ANDROID_HOME (default ~/Library/Android/sdk), an NDK and `rustup target add aarch64-linux-android`.
set -eu
cd "$(dirname "$0")/../../.."
CARGO=${CARGO:-cargo}
SDK=${ANDROID_HOME:-$HOME/Library/Android/sdk}
NDK=${ANDROID_NDK_HOME:-$(ls -d "$SDK"/ndk/* | sort -V | tail -1)}
BT=$(ls -d "$SDK"/build-tools/* | sort -V | tail -1)
PLATFORM=$(ls -d "$SDK"/platforms/android-3[5-9] | sort -V | tail -1)
API=26
BIN=$(ls -d "$NDK"/toolchains/llvm/prebuilt/*/bin)
export CC_aarch64_linux_android="$BIN/aarch64-linux-android$API-clang"
export AR_aarch64_linux_android="$BIN/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$BIN/aarch64-linux-android$API-clang"
$CARGO build --release -p lightcraft-mobile --lib --target aarch64-linux-android
OUT=${CARGO_TARGET_DIR:-target}/aarch64-linux-android/release
WORK=$OUT/apk
rm -rf "$WORK" && mkdir -p "$WORK/stage/lib/arm64-v8a"
cp "$OUT/liblightcraft_mobile.so" "$WORK/stage/lib/arm64-v8a/"
"$BT/aapt2" link -o "$WORK/base.apk" --manifest apps/lightcraft-mobile/android/AndroidManifest.xml -I "$PLATFORM/android.jar"
(cd "$WORK/stage" && zip -q -0 -r ../base.apk lib)
"$BT/zipalign" -f -p 4 "$WORK/base.apk" "$WORK/aligned.apk"
KS=$OUT/lightcraft-debug.keystore
[ -f "$KS" ] || keytool -genkeypair -keystore "$KS" -storepass android -keypass android -alias debug \
    -keyalg RSA -validity 3650 -dname "CN=LightCraft debug" >/dev/null 2>&1
"$BT/apksigner" sign --ks "$KS" --ks-pass pass:android --out "$OUT/lightcraft-mobile.apk" "$WORK/aligned.apk"
echo "$OUT/lightcraft-mobile.apk"
if [ "${1:-}" = --run ]; then
    ADB="$SDK/platform-tools/adb"
    "$ADB" install -r "$OUT/lightcraft-mobile.apk"
    "$ADB" shell am start -S -n ai.storyteller.lightcraft.mobile/android.app.NativeActivity
fi
