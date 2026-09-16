#!/usr/bin/env bash
# Sidecar folder + zip for Windows Intel (x86_64).
#
#   packaging/scripts/package-windows.sh
#
# Looks for maybraid.exe in target/x86_64-pc-windows-msvc/release or
# target/release. Does not sign (Authenticode is a later CA cert).
# GitHub windows-latest has neither ditto nor zip; use Compress-Archive.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TEMPLATE="$REPO_ROOT/packaging/windows/Maybraid"
ASSETS="$REPO_ROOT/maybraid/assets"
VERSION="${VERSION:-0.0.1}"
DIST="$REPO_ROOT/dist"
OUT="$DIST/Maybraid-${VERSION}-windows-x64"
ZIP="${OUT}.zip"

if [[ -f "$REPO_ROOT/target/x86_64-pc-windows-msvc/release/maybraid.exe" ]]; then
    EXE="$REPO_ROOT/target/x86_64-pc-windows-msvc/release/maybraid.exe"
elif [[ -f "$REPO_ROOT/target/release/maybraid.exe" ]]; then
    EXE="$REPO_ROOT/target/release/maybraid.exe"
else
    echo "maybraid.exe not found. On Windows:" >&2
    echo "   cargo build -p maybraid --release --target x86_64-pc-windows-msvc" >&2
    exit 1
fi

echo "==> Staging Windows sidecar"
rm -rf "$OUT"
mkdir -p "$OUT"
cp "$TEMPLATE/Maybraid.exe.manifest" "$OUT/"
cp "$EXE" "$OUT/Maybraid.exe"
rm -rf "$OUT/assets"
if command -v ditto >/dev/null; then
    ditto "$ASSETS" "$OUT/assets"
else
    mkdir -p "$OUT/assets"
    cp -R "$ASSETS/." "$OUT/assets/"
fi

echo "==> Zip"
rm -f "$ZIP"
if command -v ditto >/dev/null; then
    ditto -c -k --keepParent "$OUT" "$ZIP"
elif command -v zip >/dev/null; then
    (cd "$DIST" && zip -r "$(basename "$ZIP")" "$(basename "$OUT")")
elif command -v powershell.exe >/dev/null; then
    src="$OUT"
    dest="$ZIP"
    if command -v cygpath >/dev/null; then
        src="$(cygpath -w "$OUT")"
        dest="$(cygpath -w "$ZIP")"
    fi
    powershell.exe -NoProfile -Command \
        "Compress-Archive -LiteralPath '${src}' -DestinationPath '${dest}' -Force"
else
    echo "Need ditto, zip, or powershell.exe to write ${ZIP}" >&2
    exit 1
fi

if [[ ! -f "$ZIP" ]]; then
    echo "Archive was not created: $ZIP" >&2
    exit 1
fi

echo
echo "Created:"
echo "  $OUT"
echo "  $ZIP"
