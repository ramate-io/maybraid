#!/usr/bin/env bash
# Steam depot archive from the sniper ELF. Does not install the Steam runtime.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${MAYBRAID_LINUX_BIN:-$ROOT/target/sniper-release/x86_64-unknown-linux-gnu/release/maybraid}"
VERSION="${VERSION:-$("$ROOT/packaging/version.sh")}"
OUT="$ROOT/dist/Maybraid-${VERSION}-steam-linux-x64"
TAR="${OUT}.tar.xz"

[[ -f "$BIN" ]] || { echo "package-steam: run packaging/scripts/build-linux.sh" >&2; exit 1; }
"$ROOT/packaging/validate.sh" elf "$BIN"

rm -rf "$OUT"
mkdir -p "$OUT/assets"
cp "$ROOT/packaging/steamos/Maybraid/maybraid.desktop" "$OUT/"
cp "$BIN" "$OUT/maybraid"
chmod +x "$OUT/maybraid"
cp -R "$ROOT/maybraid/assets/." "$OUT/assets/"
cat > "$OUT/README-STEAM.txt" << EOF
Maybraid ${VERSION} — Steam Linux depot (x86_64)

Steamworks launch configuration must select Steam Linux Runtime 3.0 (sniper).
Extracting this archive does not install or activate that runtime.
EOF

rm -f "$TAR"
mkdir -p "$ROOT/dist"
tar -C "$ROOT/dist" -cJf "$TAR" "$(basename "$OUT")"
echo "Created $TAR"
