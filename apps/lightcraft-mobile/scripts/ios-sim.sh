#!/bin/sh
# Build lightcraft-ios for the iOS Simulator, bundle it as an .app and run it on a booted
# simulator (or the one named by $1). Needs Xcode and `rustup target add aarch64-apple-ios-sim`.
set -eu
cd "$(dirname "$0")/../../.."
CARGO=${CARGO:-cargo}
TARGET=aarch64-apple-ios-sim
PROFILE=${PROFILE:-debug}
[ "$PROFILE" = release ] && FLAG=--release || FLAG=
$CARGO build $FLAG -p lightcraft-mobile --bin lightcraft-ios --target $TARGET
OUT=${CARGO_TARGET_DIR:-target}/$TARGET/$PROFILE
APP=$OUT/LightCraft.app
rm -rf "$APP" && mkdir -p "$APP"
cp "$OUT/lightcraft-ios" "$APP/"
cp apps/lightcraft-mobile/ios/Info.plist "$APP/"
codesign -f -s - "$APP"
DEVICE=${1:-booted}
xcrun simctl install "$DEVICE" "$APP"
exec xcrun simctl launch --console-pty --terminate-running-process "$DEVICE" ai.storyteller.lightcraft.mobile
