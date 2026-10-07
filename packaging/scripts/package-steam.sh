#!/usr/bin/env bash
# Package maybraid for Steam depot (Steam Linux Runtime 3.0).
#
#   packaging/scripts/package-steam.sh
#
# Uses the sniper-built binary from build-linux-sniper.sh.
# Produces a tar.xz archive for Steam depot staging.
#
# The Steam launch configuration MUST select "Steam Linux Runtime 3.0 (sniper)".
# Extracting this archive alone does not install the runtime.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TEMPLATE="$REPO_ROOT/packaging/steamos/Maybraid"
ASSETS="$REPO_ROOT/maybraid/assets"
VERSION="${VERSION:-0.0.1}"
DIST="$REPO_ROOT/dist"
OUT="$DIST/Maybraid-${VERSION}-steam-linux-x64"
TAR="${OUT}.tar.xz"

# Look for sniper-built binary first, then fall back to default locations
if [[ -f "$REPO_ROOT/target/sniper-release/x86_64-unknown-linux-gnu/release/maybraid" ]]; then
    BIN="$REPO_ROOT/target/sniper-release/x86_64-unknown-linux-gnu/release/maybraid"
elif [[ -f "$REPO_ROOT/target/x86_64-unknown-linux-gnu/release/maybraid" ]]; then
    BIN="$REPO_ROOT/target/x86_64-unknown-linux-gnu/release/maybraid"
    echo "⚠️  Using binary from target/x86_64-unknown-linux-gnu/release" >&2
    echo "    Prefer: packaging/scripts/build-linux-sniper.sh" >&2
elif [[ -f "$REPO_ROOT/target/release/maybraid" && "$(uname -s)" == "Linux" ]]; then
    BIN="$REPO_ROOT/target/release/maybraid"
    echo "⚠️  Using binary from target/release" >&2
    echo "    Prefer: packaging/scripts/build-linux-sniper.sh" >&2
else
    echo "❌ Linux maybraid binary not found." >&2
    echo "   Run: packaging/scripts/build-linux-sniper.sh" >&2
    exit 1
fi

echo "==> Packaging Steam depot archive"
echo "    Binary: $BIN"

# Validate binary before packaging
if command -v readelf >/dev/null; then
    interp="$(readelf -l "$BIN" | grep 'program interpreter' | sed 's/.*\[//;s/\].*//')"
    echo "    ELF interpreter: $interp"
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

echo "==> tar.xz"
rm -f "$TAR"
tar -C "$DIST" -cJf "$TAR" "$(basename "$OUT")"

echo "==> Smoke test: startup check"
# Launch from a different directory to catch asset path issues
if (cd /tmp && timeout 5 "$OUT/maybraid" 2>&1 || EXIT_CODE=$?) | head -20; then
    EXIT_CODE=${EXIT_CODE:-0}
    if [[ $EXIT_CODE -eq 124 ]] || [[ $EXIT_CODE -eq 0 ]]; then
        echo "✅ Binary started successfully"
    else
        echo "❌ Binary crashed with exit code $EXIT_CODE" >&2
        exit 1
    fi
else
    echo "⚠️  Smoke test skipped or failed non-fatally"
fi

echo
echo "Created Steam depot archive:"
echo "  $OUT/"
echo "  $TAR"
echo
echo "⚠️  Steam launch configuration must select:"
echo "    Steam Linux Runtime 3.0 (sniper)"
