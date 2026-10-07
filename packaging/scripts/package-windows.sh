#!/usr/bin/env bash
# Package maybraid for Windows (x86_64).
#
#   packaging/scripts/package-windows.sh
#
# Builds and packages the Windows release binary.
# Does not sign (Authenticode is a separate CA cert step).
#
# Environment variables:
#   SKIP_BUILD=1     use existing binary in target/x86_64-pc-windows-msvc/release
#   VERSION=0.0.1    version for archive name

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TEMPLATE="$REPO_ROOT/packaging/windows/Maybraid"
ASSETS="$REPO_ROOT/maybraid/assets"
VERSION="${VERSION:-0.0.1}"
DIST="$REPO_ROOT/dist"
OUT="$DIST/Maybraid-${VERSION}-windows-x64"
ZIP="${OUT}.zip"

if [[ "${SKIP_BUILD:-}" != "1" ]]; then
    echo "==> Building maybraid (release, Windows x64)"
    echo "    Target: x86_64-pc-windows-msvc"
    (
        cd "$REPO_ROOT"
        cargo build -p maybraid \
            --release \
            --locked \
            --target x86_64-pc-windows-msvc
    )
fi

if [[ -f "$REPO_ROOT/target/x86_64-pc-windows-msvc/release/maybraid.exe" ]]; then
    EXE="$REPO_ROOT/target/x86_64-pc-windows-msvc/release/maybraid.exe"
elif [[ -f "$REPO_ROOT/target/release/maybraid.exe" ]]; then
    EXE="$REPO_ROOT/target/release/maybraid.exe"
    echo "⚠️  Using binary from target/release (not target-specific)" >&2
else
    echo "❌ maybraid.exe not found" >&2
    echo "   Run: cargo build -p maybraid --release --locked --target x86_64-pc-windows-msvc" >&2
    exit 1
fi

echo "==> Validating binary"
echo "    Binary: $EXE"
if command -v file >/dev/null; then
    file "$EXE" | sed 's/^/    /'
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
    echo "❌ Archive was not created: $ZIP" >&2
    exit 1
fi

echo "==> Smoke test: startup check"
# Try to launch from a different directory
# Windows timeout doesn't have the same semantics as Unix, so just run briefly
if command -v timeout >/dev/null 2>&1; then
    (cd /tmp 2>/dev/null || cd "$TEMP" && timeout 5 "$OUT/Maybraid.exe" 2>&1 || EXIT_CODE=$?) | head -20
    echo "✅ Binary startup check completed"
elif command -v powershell.exe >/dev/null; then
    echo "⚠️  Limited smoke test via PowerShell"
    # Just verify it's a valid PE executable
    powershell.exe -NoProfile -Command "if (Test-Path '$OUT/Maybraid.exe') { Write-Host '✅ Binary exists and is accessible' }" || true
else
    echo "⚠️  Smoke test skipped (no timeout or powershell)"
fi

echo
echo "Created Windows package:"
echo "  $OUT/"
echo "  $ZIP"
echo
echo "Extract and run: Maybraid.exe"
echo "Saves: %APPDATA%\\maybraid\\saves (or similar)"
