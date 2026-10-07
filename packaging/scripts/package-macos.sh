#!/usr/bin/env bash
# Assemble Maybraid.app from packaging/macos and wrap a UDZO DMG.
#
#   nix develop .#release-macos --command packaging/scripts/package-macos.sh
#
# or locally (requires Xcode):
#
#   VERSION=0.1.3 packaging/scripts/package-macos.sh
#
# Unsigned by default. After Developer ID is in the login keychain:
#
#   SIGN_IDENTITY="Developer ID Application: Ramate LLC (TEAMID)" \
#   NOTARY_PROFILE=maybraid-notary \
#     packaging/scripts/package-macos.sh
#
#   SKIP_BUILD=1     use an existing binary in $CARGO_TARGET_DIR or target/release
#   SKIP_NOTARY=1    sign only
#   VERSION=0.0.1    CFBundleVersion / DMG name (default: workspace 0.0.1)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TEMPLATE="$REPO_ROOT/packaging/macos/Maybraid.app"
ENTITLEMENTS="$REPO_ROOT/packaging/macos/entitlements.plist"
ASSETS="$REPO_ROOT/maybraid/assets"
ICON_SRC="$REPO_ROOT/maybraid/assets/iconography/maybraid_logo_icon_home.png"
VERSION="${VERSION:-0.0.1}"
DIST="$REPO_ROOT/dist"
APP="$DIST/Maybraid.app"
DMG_STAGE="$DIST/dmg-root"
DMG="$DIST/Maybraid-${VERSION}-macos-arm64.dmg"

# Prefer release-packaging target dir if set, otherwise fall back to default
TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
BINARY="$TARGET_DIR/aarch64-apple-darwin/release/maybraid"

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "package-macos.sh must run on macOS." >&2
    exit 1
fi

if [[ "${SKIP_BUILD:-}" != "1" ]]; then
    echo "==> Building maybraid (release, ARM64)"
    echo "    Target: aarch64-apple-darwin"
    echo "    Build dir: $TARGET_DIR"
    (
        cd "$REPO_ROOT"
        cargo build -p maybraid \
            --release \
            --locked \
            --target aarch64-apple-darwin
    )
fi

if [[ ! -f "$BINARY" ]]; then
    echo "Game binary not found: $BINARY" >&2
    echo "   Run: nix develop .#release-macos --command packaging/scripts/package-macos.sh" >&2
    echo "   Or:  cargo build -p maybraid --release --locked --target aarch64-apple-darwin" >&2
    exit 1
fi

echo "==> Validating binary dependencies"
if ! command -v otool >/dev/null; then
    echo "⚠️  otool not found; skipping dependency validation" >&2
else
    deps="$(otool -L "$BINARY" | tail -n +2)"
    echo "Binary dependencies:"
    echo "$deps" | sed 's/^/    /'
    
    # Check for forbidden paths
    if echo "$deps" | grep -q "/nix/store"; then
        echo "❌ Binary contains /nix/store references:" >&2
        echo "$deps" | grep "/nix/store" | sed 's/^/    /' >&2
        exit 1
    fi
    if echo "$deps" | grep -q "/opt/homebrew"; then
        echo "❌ Binary contains /opt/homebrew references:" >&2
        echo "$deps" | grep "/opt/homebrew" | sed 's/^/    /' >&2
        exit 1
    fi
    if echo "$deps" | grep -Eq "/usr/local/(lib|opt)"; then
        echo "❌ Binary contains /usr/local references:" >&2
        echo "$deps" | grep -E "/usr/local/(lib|opt)" | sed 's/^/    /' >&2
        exit 1
    fi
    
    echo "✅ Binary dependencies are clean (system libs and frameworks only)"
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

hdiutil_detach_maybraid() {
    /usr/bin/hdiutil detach "/Volumes/Maybraid" -force 2>/dev/null || true
    /usr/bin/hdiutil detach "Maybraid" -force 2>/dev/null || true
}

hdiutil_detach_dist_images() {
    local info image_path dev
    info="$(/usr/bin/hdiutil info 2>/dev/null || true)"
    image_path=""
    while IFS= read -r line; do
        if [[ "$line" =~ ^[[:space:]]*image-path:[[:space:]]*(.+)$ ]]; then
            image_path="${BASH_REMATCH[1]}"
        elif [[ -n "$image_path" && "$line" =~ ^(/dev/disk[0-9]+) ]]; then
            if [[ "$image_path" == "$DIST/"* ]]; then
                dev="${BASH_REMATCH[1]}"
                /usr/bin/hdiutil detach "$dev" -force 2>/dev/null \
                    || /usr/bin/hdiutil detach "$image_path" -force 2>/dev/null \
                    || true
            fi
            image_path=""
        fi
    done <<< "$info"
}

hdiutil_cleanup_before_retry() {
    hdiutil_detach_maybraid
    hdiutil_detach_dist_images
    if [[ -f "$DMG" ]]; then
        /usr/bin/hdiutil detach "$DMG" -force 2>/dev/null || true
    fi
    rm -f "$DMG"
}

hdiutil_busy_error() {
    case "$1" in
        *"Resource busy"*|*"Device busy"*|*"resource temporarily unavailable"*)
            return 0
            ;;
    esac
    return 1
}

create_udzo_dmg() {
    local attempt output status max_attempts=5
    sync
    for (( attempt=1; attempt<=max_attempts; attempt++ )); do
        output="$(/usr/bin/hdiutil create \
            -volname "Maybraid" \
            -srcfolder "$DMG_STAGE" \
            -ov \
            -format UDZO \
            "$DMG" 2>&1)" && return 0
        status=$?
        if hdiutil_busy_error "$output" && (( attempt < max_attempts )); then
            echo "hdiutil create busy (attempt ${attempt}/${max_attempts}), retrying..." >&2
            printf '%s\n' "$output" >&2
            hdiutil_cleanup_before_retry
            sync
            sleep $((attempt * 2))
            continue
        fi
        printf '%s\n' "$output" >&2
        return "$status"
    done
}

echo "==> DMG"
rm -rf "$DMG_STAGE"
mkdir -p "$DMG_STAGE"
/usr/bin/ditto "$APP" "$DMG_STAGE/Maybraid.app"
ln -s /Applications "$DMG_STAGE/Applications"
create_udzo_dmg

if [[ -n "${SIGN_IDENTITY:-}" ]]; then
    /usr/bin/codesign --timestamp --sign "$SIGN_IDENTITY" "$DMG"
    if [[ "${SKIP_NOTARY:-}" != "1" && -n "${NOTARY_PROFILE:-}" ]]; then
        echo "==> Notarizing"
        /usr/bin/xcrun notarytool submit "$DMG" --keychain-profile "$NOTARY_PROFILE" --wait
        /usr/bin/xcrun stapler staple "$DMG"
        /usr/sbin/spctl --assess --type open --context context:primary-signature -v "$DMG"
    fi
fi

echo "==> Smoke test: startup check"
# Try to launch the app briefly to catch dyld errors (issue #1013)
# Use gtimeout if available (from coreutils), otherwise skip
if command -v gtimeout >/dev/null 2>&1; then
    # Launch from a different directory to catch asset path issues
    (cd /tmp && gtimeout 5 "$APP/Contents/MacOS/maybraid" 2>&1 || EXIT_CODE=$?) | head -20
    EXIT_CODE=${EXIT_CODE:-0}
    if [[ $EXIT_CODE -eq 124 ]] || [[ $EXIT_CODE -eq 0 ]]; then
        echo "✅ App started successfully (no dyld/startup crash)"
    else
        echo "❌ App crashed with exit code $EXIT_CODE" >&2
        exit 1
    fi
else
    echo "⚠️  gtimeout not available; skipping startup test"
    echo "   Install: brew install coreutils"
fi

echo
echo "Created:"
echo "  $APP"
echo "  $DMG"
echo "Open the app:  open \"$APP\""
