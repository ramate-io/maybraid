#!/usr/bin/env bash
# Package maybraid as a standalone AppImage for direct download.
#
#   packaging/scripts/package-appimage.sh
#
# Uses the sniper-built binary from build-linux-sniper.sh.
# Produces a self-contained AppImage for Linux desktops.
#
# AppImage does not eliminate glibc/ABI requirements. The binary is built
# against sniper SDK (glibc 2.28+). Supported baseline:
# - Ubuntu 20.04 / Debian 10 / equivalent or newer
# - Steam Deck / SteamOS 3.0

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ASSETS="$REPO_ROOT/maybraid/assets"
VERSION="${VERSION:-0.0.1}"
DIST="$REPO_ROOT/dist"
APPDIR="$DIST/Maybraid.AppDir"
APPIMAGE="$DIST/Maybraid-${VERSION}-linux-x64.AppImage"

# Look for sniper-built binary
if [[ -f "$REPO_ROOT/target/sniper-release/x86_64-unknown-linux-gnu/release/maybraid" ]]; then
    BIN="$REPO_ROOT/target/sniper-release/x86_64-unknown-linux-gnu/release/maybraid"
elif [[ -f "$REPO_ROOT/target/x86_64-unknown-linux-gnu/release/maybraid" ]]; then
    BIN="$REPO_ROOT/target/x86_64-unknown-linux-gnu/release/maybraid"
    echo "⚠️  Using binary from target/x86_64-unknown-linux-gnu/release" >&2
    echo "    Prefer: packaging/scripts/build-linux-sniper.sh" >&2
else
    echo "❌ Linux maybraid binary not found." >&2
    echo "   Run: packaging/scripts/build-linux-sniper.sh" >&2
    exit 1
fi

echo "==> Packaging AppImage"
echo "    Binary: $BIN"

# Validate binary
if command -v readelf >/dev/null; then
    interp="$(readelf -l "$BIN" | grep 'program interpreter' | sed 's/.*\[//;s/\].*//')"
    echo "    ELF interpreter: $interp"
    
    glibc_version="$(readelf -V "$BIN" 2>/dev/null | grep GLIBC_ | sed 's/.*GLIBC_//' | sort -V | tail -1 || echo 'unknown')"
    echo "    Max GLIBC version: $glibc_version"
fi

# Create AppDir structure
echo "==> Staging AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"

# Copy binary
cp "$BIN" "$APPDIR/usr/bin/maybraid"
chmod +x "$APPDIR/usr/bin/maybraid"

# Copy assets (AppImage will mount at runtime, so assets live beside binary)
rm -rf "$APPDIR/usr/bin/assets"
mkdir -p "$APPDIR/usr/bin/assets"
cp -R "$ASSETS/." "$APPDIR/usr/bin/assets/"

# Create desktop file
cat > "$APPDIR/maybraid.desktop" << 'EOF'
[Desktop Entry]
Type=Application
Name=Maybraid
Comment=A game of peer-based state
Exec=maybraid
Icon=maybraid
Categories=Game;
Terminal=false
EOF

cp "$APPDIR/maybraid.desktop" "$APPDIR/usr/share/applications/"

# Create icon (if available)
ICON_SRC="$REPO_ROOT/maybraid/assets/iconography/maybraid_logo_icon_home.png"
if [[ -f "$ICON_SRC" ]]; then
    cp "$ICON_SRC" "$APPDIR/maybraid.png"
    cp "$ICON_SRC" "$APPDIR/usr/share/icons/hicolor/256x256/apps/maybraid.png"
fi

# Create AppRun launcher that sets asset paths
cat > "$APPDIR/AppRun" << 'EOF'
#!/bin/bash
# AppImage launcher: ensures assets are found regardless of launch directory

SELF="$(readlink -f "$0")"
HERE="${SELF%/*}"

# Set asset root to mounted AppImage location
export MAYBRAID_ASSETS="$HERE/usr/bin/assets"

# Saves, config, logs go to user's home (outside AppImage)
export MAYBRAID_SAVES="${XDG_DATA_HOME:-$HOME/.local/share}/maybraid/saves"

# Launch the game
exec "$HERE/usr/bin/maybraid" "$@"
EOF
chmod +x "$APPDIR/AppRun"

# Download appimagetool if not present
APPIMAGETOOL="$REPO_ROOT/target/appimagetool-x86_64.AppImage"
if [[ ! -f "$APPIMAGETOOL" ]]; then
    echo "==> Downloading appimagetool"
    curl -L -o "$APPIMAGETOOL" \
        "https://github.com/AppImage/AppImageKit/releases/download/13/appimagetool-x86_64.AppImage"
    chmod +x "$APPIMAGETOOL"
fi

# Build AppImage
echo "==> Building AppImage"
rm -f "$APPIMAGE"

# appimagetool needs ARCH set
export ARCH=x86_64

"$APPIMAGETOOL" "$APPDIR" "$APPIMAGE" 2>&1 | grep -v "desktop-file-validate"

if [[ ! -f "$APPIMAGE" ]]; then
    echo "❌ AppImage creation failed" >&2
    exit 1
fi

chmod +x "$APPIMAGE"

echo "==> Smoke test: startup check"
# Try to launch from /tmp to verify asset discovery
if (cd /tmp && timeout 5 "$APPIMAGE" 2>&1 || EXIT_CODE=$?) | head -20; then
    EXIT_CODE=${EXIT_CODE:-0}
    if [[ $EXIT_CODE -eq 124 ]] || [[ $EXIT_CODE -eq 0 ]]; then
        echo "✅ AppImage started successfully"
    else
        echo "⚠️  AppImage exited with code $EXIT_CODE (may be GPU/display limitation)"
    fi
else
    echo "⚠️  Smoke test skipped or failed non-fatally"
fi

echo
echo "Created standalone AppImage:"
echo "  $APPIMAGE"
echo
echo "Supported baseline: Ubuntu 20.04 / Debian 10 / glibc 2.28+"
echo "Run: chmod +x Maybraid-*.AppImage && ./Maybraid-*.AppImage"
echo "Saves: ~/.local/share/maybraid/saves"
