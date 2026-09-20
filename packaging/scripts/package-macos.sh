#!/usr/bin/env bash
# Assemble Maybraid.app from packaging/macos and wrap a UDZO DMG.
#
#   packaging/scripts/package-macos.sh
#
# Unsigned by default. After Developer ID is in the login keychain:
#
#   SIGN_IDENTITY="Developer ID Application: Ramate LLC (TEAMID)" \
#   NOTARY_PROFILE=maybraid-notary \
#     packaging/scripts/package-macos.sh
#
#   SKIP_BUILD=1     use an existing target/release/maybraid
#   SKIP_NOTARY=1    sign only
#   VERSION=0.0.1    CFBundleVersion / DMG name (default: workspace 0.0.1)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TEMPLATE="$REPO_ROOT/packaging/macos/Maybraid.app"
ENTITLEMENTS="$REPO_ROOT/packaging/macos/entitlements.plist"
ASSETS="$REPO_ROOT/maybraid/assets"
ICON_SRC="$REPO_ROOT/maybraid/assets/iconography/maybraid_logo_icon_home.png"
BINARY="${BINARY:-$REPO_ROOT/target/release/maybraid}"
VERSION="${VERSION:-0.0.1}"
DIST="$REPO_ROOT/dist"
APP="$DIST/Maybraid.app"
DMG_STAGE="$DIST/dmg-root"
DMG="$DIST/Maybraid-${VERSION}-macos-arm64.dmg"

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "package-macos.sh must run on macOS." >&2
    exit 1
fi

if [[ "${SKIP_BUILD:-}" != "1" ]]; then
    echo "==> Building maybraid (release)"
    (cd "$REPO_ROOT" && cargo build -p maybraid --release)
fi

if [[ ! -f "$BINARY" ]]; then
    echo "Game binary not found: $BINARY" >&2
    echo "   cargo build -p maybraid --release" >&2
    exit 1
fi

if [[ ! -d "$TEMPLATE" || ! -d "$ASSETS" ]]; then
    echo "Missing template or assets." >&2
    exit 1
fi

echo "==> Staging app bundle"
rm -rf "$DIST"
mkdir -p "$DIST"
/usr/bin/ditto "$TEMPLATE" "$APP"
rm -f "$APP/Contents/MacOS/.gitkeep"
rm -rf "$APP/Contents/Resources/assets"
/usr/bin/ditto "$BINARY" "$APP/Contents/MacOS/maybraid"
chmod +x "$APP/Contents/MacOS/maybraid"
/usr/bin/ditto "$ASSETS" "$APP/Contents/Resources/assets"

if [[ -x /usr/libexec/PlistBuddy ]]; then
    /usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${VERSION}" "$APP/Contents/Info.plist"
    /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString ${VERSION}" "$APP/Contents/Info.plist"
fi

if [[ -f "$ICON_SRC" ]] && command -v sips >/dev/null && command -v iconutil >/dev/null; then
    echo "==> AppIcon.icns"
    iconset="$DIST/Maybraid.iconset"
    rm -rf "$iconset"
    mkdir -p "$iconset"
    for sz in 16 32 128 256 512; do
        sips -z "$sz" "$sz" "$ICON_SRC" --out "$iconset/icon_${sz}x${sz}.png" >/dev/null
        sips -z "$((sz * 2))" "$((sz * 2))" "$ICON_SRC" --out "$iconset/icon_${sz}x${sz}@2x.png" >/dev/null
    done
    iconutil -c icns "$iconset" -o "$APP/Contents/Resources/AppIcon.icns"
    rm -rf "$iconset"
fi

if [[ -n "${SIGN_IDENTITY:-}" ]]; then
    echo "==> Signing"
    /usr/bin/codesign --force --options runtime --timestamp \
        --entitlements "$ENTITLEMENTS" \
        --sign "$SIGN_IDENTITY" \
        "$APP/Contents/MacOS/maybraid"
    /usr/bin/codesign --force --options runtime --timestamp \
        --entitlements "$ENTITLEMENTS" \
        --sign "$SIGN_IDENTITY" \
        "$APP"
    /usr/bin/codesign --verify --deep --strict --verbose=2 "$APP"
else
    echo "==> Unsigned (set SIGN_IDENTITY to Developer ID Application: …)"
fi

echo "==> DMG"
rm -rf "$DMG_STAGE"
mkdir -p "$DMG_STAGE"
/usr/bin/ditto "$APP" "$DMG_STAGE/Maybraid.app"
ln -s /Applications "$DMG_STAGE/Applications"
/usr/bin/hdiutil create \
    -volname "Maybraid" \
    -srcfolder "$DMG_STAGE" \
    -ov \
    -format UDZO \
    "$DMG"

if [[ -n "${SIGN_IDENTITY:-}" ]]; then
    /usr/bin/codesign --timestamp --sign "$SIGN_IDENTITY" "$DMG"
    if [[ "${SKIP_NOTARY:-}" != "1" && -n "${NOTARY_PROFILE:-}" ]]; then
        echo "==> Notarizing"
        /usr/bin/xcrun notarytool submit "$DMG" --keychain-profile "$NOTARY_PROFILE" --wait
        /usr/bin/xcrun stapler staple "$DMG"
        /usr/sbin/spctl --assess --type open --context context:primary-signature -v "$DMG"
    fi
fi

echo
echo "Created:"
echo "  $APP"
echo "  $DMG"
echo "Open the app:  open \"$APP\""
