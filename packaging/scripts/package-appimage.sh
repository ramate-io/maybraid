#!/usr/bin/env bash
# Standalone AppImage of the sniper ELF via linuxdeploy (same SDK libs).
# https://docs.appimage.org/packaging-guide/from-source/linuxdeploy-user-guide.html
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${MAYBRAID_LINUX_BIN:-$ROOT/target/sniper-release/x86_64-unknown-linux-gnu/release/maybraid}"
VERSION="${VERSION:-$("$ROOT/packaging/version.sh")}"
SDK="$(tr -d '[:space:]' < "$ROOT/packaging/linux/image")"
DIST="$ROOT/dist"
APPDIR="$DIST/Maybraid.AppDir"
APPIMAGE="$DIST/Maybraid-${VERSION}-linux-x64.AppImage"

LINUXDEPLOY_VER="1-alpha-20251107-1"
LINUXDEPLOY_URL="https://github.com/linuxdeploy/linuxdeploy/releases/download/${LINUXDEPLOY_VER}/linuxdeploy-x86_64.AppImage"
LINUXDEPLOY_SHA256="c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d"
LINUXDEPLOY="$ROOT/target/linuxdeploy-${LINUXDEPLOY_VER}-x86_64.AppImage"

[[ -f "$BIN" ]] || { echo "package-appimage: run packaging/scripts/build-linux.sh" >&2; exit 1; }
"$ROOT/packaging/validate.sh" elf "$BIN"
command -v docker >/dev/null || { echo "package-appimage: docker is required" >&2; exit 1; }

if [[ ! -f "$LINUXDEPLOY" ]]; then
	mkdir -p "$(dirname "$LINUXDEPLOY")"
	curl -L -o "$LINUXDEPLOY.partial" "$LINUXDEPLOY_URL"
	mv "$LINUXDEPLOY.partial" "$LINUXDEPLOY"
fi
echo "${LINUXDEPLOY_SHA256}  ${LINUXDEPLOY}" | sha256sum -c -
chmod +x "$LINUXDEPLOY"

rm -rf "$APPDIR"
mkdir -p "$DIST" "$APPDIR/usr/bin/assets" "$APPDIR/usr/share/doc/maybraid"
cp "$BIN" "$APPDIR/usr/bin/maybraid"
chmod +x "$APPDIR/usr/bin/maybraid"
cp -R "$ROOT/maybraid/assets/." "$APPDIR/usr/bin/assets/"

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

ICON="$ROOT/maybraid/assets/iconography/maybraid_logo_icon_home.png"
[[ -f "$ICON" ]] && cp "$ICON" "$APPDIR/maybraid.png"

cat > "$APPDIR/AppRun" << 'EOF'
#!/bin/bash
SELF="$(readlink -f "$0")"
HERE="${SELF%/*}"
export MAYBRAID_ASSETS="$HERE/usr/bin/assets"
export MAYBRAID_PACKAGED=1
export MAYBRAID_SAVES="${MAYBRAID_SAVES:-${XDG_DATA_HOME:-$HOME/.local/share}/maybraid/saves}"
exec "$HERE/usr/bin/maybraid" "$@"
EOF
chmod +x "$APPDIR/AppRun"

cat > "$APPDIR/usr/share/doc/maybraid/THIRD_PARTY_NOTICES.txt" << EOF
Maybraid ${VERSION} AppImage

Shared libraries come from the Steam Runtime 3.0 sniper SDK via linuxdeploy.
libc, libstdc++, libgcc, and GPU/display drivers are excluded and come from
the host. Baseline: glibc 2.31+ (Ubuntu 20.04 / Debian 11 / SteamOS 3).
EOF

echo "==> linuxdeploy ${LINUXDEPLOY_VER} (in $SDK)"
rm -f "$DIST"/Maybraid*.AppImage
docker run --rm \
	--user 0:0 \
	-v "$ROOT:/src:rw" \
	-w /src/dist \
	-e APPIMAGE_EXTRACT_AND_RUN=1 \
	-e ARCH=x86_64 \
	-e HOST_UID="$(id -u)" \
	-e HOST_GID="$(id -g)" \
	-e LINUXDEPLOY="/src/target/$(basename "$LINUXDEPLOY")" \
	"$SDK" \
	bash -lc '
		set -euo pipefail
		chmod +x "$LINUXDEPLOY"
		"$LINUXDEPLOY" \
			--appdir /src/dist/Maybraid.AppDir \
			--executable /src/dist/Maybraid.AppDir/usr/bin/maybraid \
			--desktop-file /src/dist/Maybraid.AppDir/maybraid.desktop \
			--icon-file /src/dist/Maybraid.AppDir/maybraid.png \
			--exclude-library "libGL*" \
			--exclude-library "libEGL*" \
			--exclude-library "libGLdispatch*" \
			--exclude-library "libGLX*" \
			--exclude-library "libOpenGL*" \
			--exclude-library "libvulkan*" \
			--exclude-library "libdrm*" \
			--exclude-library "libgbm*" \
			--exclude-library "libcuda*" \
			--exclude-library "libnvidia*" \
			--output appimage
		chown -R "$HOST_UID:$HOST_GID" /src/dist || true
	'

built="$(find "$DIST" -maxdepth 1 -name '*.AppImage' | head -1)"
[[ -n "$built" ]] || { echo "package-appimage: linuxdeploy produced no AppImage" >&2; exit 1; }
mv "$built" "$APPIMAGE"
chmod +x "$APPIMAGE"
echo "Created $APPIMAGE"
