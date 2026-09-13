#!/usr/bin/env bash
# Sidecar folder + tar.xz for SteamOS / Steam Deck (x86_64 Linux).
#
#   packaging/scripts/package-steamos.sh
#
# Looks for maybraid in target/x86_64-unknown-linux-gnu/release or
# target/release. No OS code signature. Prefer building inside the Steam
# Linux Runtime 3.0 (sniper) SDK for Deck glibc.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TEMPLATE="$REPO_ROOT/packaging/steamos/Maybraid"
ASSETS="$REPO_ROOT/maybraid/assets"
VERSION="${VERSION:-0.0.1}"
DIST="$REPO_ROOT/dist"
OUT="$DIST/Maybraid-${VERSION}-steamos-x64"
TAR="${OUT}.tar.xz"

if [[ -f "$REPO_ROOT/target/x86_64-unknown-linux-gnu/release/maybraid" ]]; then
    BIN="$REPO_ROOT/target/x86_64-unknown-linux-gnu/release/maybraid"
elif [[ -f "$REPO_ROOT/target/release/maybraid" && "$(uname -s)" == "Linux" ]]; then
    BIN="$REPO_ROOT/target/release/maybraid"
else
    echo "Linux maybraid binary not found." >&2
    echo "   cargo build -p maybraid --release --target x86_64-unknown-linux-gnu" >&2
    echo "   (or build inside the Steam Runtime sniper SDK)" >&2
    exit 1
fi

echo "==> Staging SteamOS sidecar"
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

echo "==> tar.xz"
rm -f "$TAR"
tar -C "$DIST" -cJf "$TAR" "$(basename "$OUT")"

echo
echo "Created:"
echo "  $OUT"
echo "  $TAR"
