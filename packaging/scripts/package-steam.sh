#!/usr/bin/env bash
# Package the sniper-built Linux binary for Steam depot staging.
#
#   packaging/scripts/package-steam.sh
#
# Produces Maybraid-<version>-steam-linux-x64.tar.xz.
# Extracting this archive does not install or activate Steam Linux Runtime 3.0.
# Steamworks Linux launch configuration must select:
#   Steam Linux Runtime 3.0 (sniper)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=packaging/scripts/common.sh
source "$REPO_ROOT/packaging/scripts/common.sh"

TEMPLATE="$REPO_ROOT/packaging/steamos/Maybraid"
ASSETS="$REPO_ROOT/maybraid/assets"
VERSION="${VERSION:-0.0.1}"
DIST="$REPO_ROOT/dist"
OUT="$DIST/Maybraid-${VERSION}-steam-linux-x64"
TAR="${OUT}.tar.xz"

BIN="$(maybraid_require_sniper_bin)"

echo "==> Packaging Steam depot archive"
echo "    Binary: $BIN"
maybraid_validate_linux_elf "$BIN"

if [[ ! -d "$TEMPLATE" || ! -d "$ASSETS" ]]; then
	echo "Missing Steam template or assets." >&2
	exit 1
fi

echo "==> Staging Steam depot"
rm -rf "$OUT"
mkdir -p "$OUT"
cp "$TEMPLATE/maybraid.desktop" "$OUT/"
if [[ -f "$TEMPLATE/steam_appid.txt" ]]; then
	cp "$TEMPLATE/steam_appid.txt" "$OUT/"
fi
cp "$BIN" "$OUT/maybraid"
chmod +x "$OUT/maybraid"
rm -rf "$OUT/assets"
mkdir -p "$OUT/assets"
cp -R "$ASSETS/." "$OUT/assets/"

cat > "$OUT/README-STEAM.txt" << EOF
Maybraid ${VERSION} — Steam Linux depot (x86_64)

This archive is for Steam depot staging only.

It does not install, bundle, or activate Steam Linux Runtime 3.0.
In the Steamworks partner site, set the Linux launch option to:

    Steam Linux Runtime 3.0 (sniper)

The game executable was compiled inside that SDK and expects that runtime
for glibc and common libraries. Host GPU drivers stay on the Steam Deck /
desktop; they are not in this archive.

Assets are the assets/ directory next to maybraid. Saves go to
~/.local/share/maybraid/saves (or MAYBRAID_SAVES).
EOF

echo "==> tar.xz"
rm -f "$TAR"
tar -C "$DIST" -cJf "$TAR" "$(basename "$OUT")"

SNIPER_PLATFORM_IMAGE="${SNIPER_PLATFORM_IMAGE:-registry.gitlab.steamos.cloud/steamrt/sniper/platform@sha256:8cd1bdfc418afd8b019a69d6f5a9305865a5f3bb5554102006fdb8f6853d5f67}"

CONTAINER_CMD="$(maybraid_container_cmd)"

if [[ -n "$CONTAINER_CMD" ]]; then
	echo "==> Loader check inside Steam Runtime 3.0 platform"
	$CONTAINER_CMD run --rm --user 0:0 --entrypoint bash \
		-v "$OUT:/opt/maybraid:ro" \
		"$SNIPER_PLATFORM_IMAGE" \
		-lc '
			set -euo pipefail
			echo "    platform: $(. /etc/os-release 2>/dev/null; echo ${PRETTY_NAME:-unknown} BUILD_ID=${BUILD_ID:-unknown})"
			ldd /opt/maybraid/maybraid | sed "s/^/    /"
			if ldd /opt/maybraid/maybraid | grep -vE "libGL|libEGL|libGLdispatch|libGLX|libOpenGL|libvulkan|libdrm|libcuda|libnvidia|libgbm" | grep -q "not found"; then
				echo "unresolved non-GPU libraries in sniper platform" >&2
				exit 1
			fi
		'
	echo "✅ Steam runtime loader check passed (GPU/window not validated)"
else
	echo "⚠️  No docker/podman; skipped Steam runtime container loader check"
fi

maybraid_smoke_unix "$OUT/maybraid" "Steam-staged maybraid (host, not Steam runtime)"

echo
echo "Created Steam depot archive:"
echo "  $OUT/"
echo "  $TAR"
echo
echo "Steamworks launch configuration must select Steam Linux Runtime 3.0 (sniper)."
echo "Extracting this archive does not install that runtime."
