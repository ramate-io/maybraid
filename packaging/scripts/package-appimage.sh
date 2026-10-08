#!/usr/bin/env bash
# Package the sniper-built Linux binary as a standalone AppImage.
#
#   packaging/scripts/package-appimage.sh
#
# Uses the same executable as package-steam.sh. AppImage does not remove the
# glibc/ABI floor of that binary (Steam Runtime 3.0 sniper / glibc 2.31).
#
# Supported standalone baseline (binary + unbundled system/runtime libs):
#   glibc 2.31+  — Debian 11, Ubuntu 20.04, SteamOS 3, or newer
# Host GPU drivers and core libc/libstdc++ are not bundled.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=packaging/scripts/common.sh
source "$REPO_ROOT/packaging/scripts/common.sh"

ASSETS="$REPO_ROOT/maybraid/assets"
VERSION="${VERSION:-0.0.1}"
DIST="$REPO_ROOT/dist"
APPDIR="$DIST/Maybraid.AppDir"
APPIMAGE="$DIST/Maybraid-${VERSION}-linux-x64.AppImage"

# Maintained appimagetool (not the retired AppImageKit tree), pinned by tag + sha256.
APPIMAGETOOL_VERSION="1.9.1"
APPIMAGETOOL_URL="https://github.com/AppImage/appimagetool/releases/download/${APPIMAGETOOL_VERSION}/appimagetool-x86_64.AppImage"
APPIMAGETOOL_SHA256="ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0"
APPIMAGETOOL="$REPO_ROOT/target/appimagetool-${APPIMAGETOOL_VERSION}-x86_64.AppImage"

BIN="$(maybraid_require_sniper_bin)"

echo "==> Packaging standalone AppImage"
echo "    Binary: $BIN"
maybraid_validate_linux_elf "$BIN"

if [[ ! -d "$ASSETS" ]]; then
	echo "Missing assets." >&2
	exit 1
fi

echo "==> Staging AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$APPDIR/usr/share/doc/maybraid"

cp "$BIN" "$APPDIR/usr/bin/maybraid"
chmod +x "$APPDIR/usr/bin/maybraid"

# Sidecar next to the executable so assets_beside_executable() works without env.
rm -rf "$APPDIR/usr/bin/assets"
mkdir -p "$APPDIR/usr/bin/assets"
cp -R "$ASSETS/." "$APPDIR/usr/bin/assets/"

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

ICON_SRC="$REPO_ROOT/maybraid/assets/iconography/maybraid_logo_icon_home.png"
if [[ -f "$ICON_SRC" ]]; then
	cp "$ICON_SRC" "$APPDIR/maybraid.png"
	cp "$ICON_SRC" "$APPDIR/usr/share/icons/hicolor/256x256/apps/maybraid.png"
fi

# No extra .so files are copied. Do not bundle libc, libstdc++, libgcc, or
# host graphics drivers (libGL, libEGL, libvulkan, libdrm, NVIDIA/AMD).
cat > "$APPDIR/usr/share/doc/maybraid/THIRD_PARTY_NOTICES.txt" << EOF
Maybraid ${VERSION} standalone AppImage

This package redistributes the Maybraid executable and game assets only.
It does not bundle glibc, libstdc++, libgcc_s, or GPU/display drivers.

Those libraries come from the host OS. The executable was built in Valve's
Steam Runtime 3.0 (sniper) SDK, so the host must provide at least:

  - glibc 2.31 (Debian 11 / Ubuntu 20.04 / SteamOS 3 or newer)
  - A working Vulkan or OpenGL driver from the OS/GPU vendor

See packaging/README.md in the source tree for the Steam-runtime archive,
which is a different artifact and expects Steam Linux Runtime 3.0.
EOF
cp "$APPDIR/usr/share/doc/maybraid/THIRD_PARTY_NOTICES.txt" "$APPDIR/THIRD_PARTY_NOTICES.txt"

cat > "$APPDIR/AppRun" << 'EOF'
#!/bin/bash
# Find assets from the mounted AppImage, not the caller's working directory.
# Writable saves/config stay on the host (the AppImage mount is read-only).

SELF="$(readlink -f "$0")"
HERE="${SELF%/*}"

export MAYBRAID_ASSETS="$HERE/usr/bin/assets"
export MAYBRAID_PACKAGED=1
export MAYBRAID_SAVES="${MAYBRAID_SAVES:-${XDG_DATA_HOME:-$HOME/.local/share}/maybraid/saves}"

exec "$HERE/usr/bin/maybraid" "$@"
EOF
chmod +x "$APPDIR/AppRun"

if [[ ! -f "$APPIMAGETOOL" ]]; then
	echo "==> Downloading appimagetool ${APPIMAGETOOL_VERSION}"
	mkdir -p "$(dirname "$APPIMAGETOOL")"
	curl -L -o "$APPIMAGETOOL.partial" "$APPIMAGETOOL_URL"
	mv "$APPIMAGETOOL.partial" "$APPIMAGETOOL"
fi
echo "${APPIMAGETOOL_SHA256}  ${APPIMAGETOOL}" | sha256sum -c -
chmod +x "$APPIMAGETOOL"

echo "==> Building AppImage"
rm -f "$APPIMAGE"
export ARCH=x86_64
export APPIMAGE_EXTRACT_AND_RUN=1
"$APPIMAGETOOL" "$APPDIR" "$APPIMAGE"

if [[ ! -f "$APPIMAGE" ]]; then
	echo "❌ AppImage creation failed" >&2
	exit 1
fi
chmod +x "$APPIMAGE"

maybraid_smoke_unix "$APPIMAGE" "standalone AppImage"

echo
echo "Created standalone AppImage (not the Steam depot archive):"
echo "  $APPIMAGE"
echo
echo "Supported baseline: glibc 2.31+ (Ubuntu 20.04 / Debian 11 / SteamOS 3)"
echo "Saves: ~/.local/share/maybraid/saves"
echo "This file does not include or activate Steam Linux Runtime 3.0."
