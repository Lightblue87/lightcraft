#!/bin/sh
# Build lightcraft-ios for iPhones and iPads and package it as an UNSIGNED .ipa for sideloading
# (AltStore Classic / AltServer, SideStore …), which sign it with the user's own Apple ID when they
# install it. No certificate, provisioning profile, team id or Apple ID is used or needed here; for a
# signed development install on your own devices use ios-device.sh instead.
#
# Needs macOS with Xcode (actool, PlistBuddy) and `rustup target add aarch64-apple-ios`.
#
#   IOS_BUNDLE_ID      default local.lightcraft.ios: keep it stable between builds, so a new .ipa
#                      installs as an update of the old one (AltStore may append the team id)
#   IOS_BUILD_NUMBER   CFBundleVersion, default the commit count of HEAD (grows with every commit)
#   IOS_VERSION        CFBundleShortVersionString, default the workspace version (Cargo.toml)
#   PROFILE            cargo profile, default release (debug is slow on a phone)
#   IPA_DIR            where the .ipa goes, default dist/ios
#
# Prints the path of the .ipa as its last line. The binary's dSYM (if cargo made one) is zipped
# next to it, for symbolicating crash logs from the phone.
set -eu
cd "$(dirname "$0")/../../.."
ROOT=$(pwd)
CARGO=${CARGO:-cargo}
TARGET=aarch64-apple-ios
PROFILE=${PROFILE:-release}
BUNDLE_ID=${IOS_BUNDLE_ID:-local.lightcraft.ios}
IPA_DIR=${IPA_DIR:-dist/ios}
# the deployment target of the binary matches MinimumOSVersion in ios/Info.plist
MIN_IOS=$(/usr/libexec/PlistBuddy -c "Print :MinimumOSVersion" apps/lightcraft-mobile/ios/Info.plist)
export IPHONEOS_DEPLOYMENT_TARGET="$MIN_IOS"

if [ -z "${IOS_VERSION:-}" ]; then
    IOS_VERSION=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -n 1)
fi
[ -n "$IOS_VERSION" ] || { echo "ios-ipa: no version (set IOS_VERSION)" >&2; exit 1; }
COMMIT=$(git rev-parse --short=12 HEAD 2>/dev/null || echo unknown)
BUILD=${IOS_BUILD_NUMBER:-$(git rev-list --count HEAD 2>/dev/null || echo 1)}
case $BUILD in '' | *[!0-9.]*) echo "ios-ipa: IOS_BUILD_NUMBER must be numeric, got '$BUILD'" >&2; exit 1 ;; esac

[ "$PROFILE" = release ] && FLAG=--release || FLAG="--profile $PROFILE"
[ "$PROFILE" = debug ] && FLAG=
# shellcheck disable=SC2086 # FLAG is one or two words, or none
$CARGO build $FLAG -p lightcraft-mobile --bin lightcraft-ios --target $TARGET
OUT=${CARGO_TARGET_DIR:-target}/$TARGET/$PROFILE
case $OUT in /*) ;; *) OUT=$ROOT/$OUT ;; esac

# the .app, as ios-device.sh builds it, minus the signing
STAGE=$OUT/ipa-stage
APP=$STAGE/Payload/LightCraft.app
rm -rf "$STAGE" && mkdir -p "$APP"
cp "$OUT/lightcraft-ios" "$APP/"
chmod 755 "$APP/lightcraft-ios"
cp apps/lightcraft-mobile/ios/Info.plist "$APP/"
xcrun actool assets/app-icon/ios/Assets.xcassets --compile "$APP" --platform iphoneos \
    --minimum-deployment-target "$MIN_IOS" --target-device iphone --target-device ipad --app-icon AppIcon \
    --output-partial-info-plist "$OUT/AppIcon.plist" --output-format human-readable-text --notices --warnings >/dev/null
PB=/usr/libexec/PlistBuddy
PLIST=$APP/Info.plist
$PB -c "Merge $OUT/AppIcon.plist" "$PLIST" >/dev/null
$PB -c "Set :CFBundleIdentifier $BUNDLE_ID" "$PLIST"
$PB -c "Set :CFBundleShortVersionString $IOS_VERSION" "$PLIST"
$PB -c "Set :CFBundleVersion $BUILD" "$PLIST"
# keys Xcode writes into every device build; some installers look for them
set_key() { $PB -c "Delete :$1" "$PLIST" >/dev/null 2>&1 || true; $PB -c "Add :$1 $2 $3" "$PLIST"; }
set_key CFBundleInfoDictionaryVersion string 6.0
set_key CFBundleDisplayName string LightCraft
set_key DTPlatformName string iphoneos
$PB -c "Delete :CFBundleSupportedPlatforms" "$PLIST" >/dev/null 2>&1 || true
$PB -c "Add :CFBundleSupportedPlatforms array" -c "Add :CFBundleSupportedPlatforms:0 string iPhoneOS" "$PLIST"
# which commit this is, shown nowhere but readable from the installed app
set_key LCBuildCommit string "$COMMIT"
plutil -lint "$PLIST" >/dev/null

mkdir -p "$IPA_DIR"
IPA_DIR=$(cd "$IPA_DIR" && pwd)
NAME=LightCraft-iOS-$IOS_VERSION-$BUILD-$COMMIT-unsigned
IPA=$IPA_DIR/$NAME.ipa
rm -f "$IPA"
# zip from inside the stage, so the archive starts at Payload/; -y keeps symlinks, -X drops extra
# attributes; ditto would add __MACOSX entries
(cd "$STAGE" && zip -qry -X "$IPA" Payload)
if [ -d "$OUT/lightcraft-ios.dSYM" ]; then
    rm -f "$IPA_DIR/$NAME.dSYM.zip"
    (cd "$OUT" && zip -qry -X "$IPA_DIR/$NAME.dSYM.zip" lightcraft-ios.dSYM)
fi
echo "ios-ipa: bundle id $BUNDLE_ID, version $IOS_VERSION ($BUILD), commit $COMMIT, minimum iOS $MIN_IOS" >&2
echo "$IPA"
