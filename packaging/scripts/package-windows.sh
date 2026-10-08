#!/usr/bin/env bash
# Package maybraid for Windows (x86_64).
#
#   packaging/scripts/package-windows.sh
#
# Builds and packages the Windows release zip. Does not Authenticode-sign.
#
#   SKIP_BUILD=1     use existing binary under $CARGO_TARGET_DIR
#   VERSION=0.0.1    version for archive name
#   CARGO_TARGET_DIR  separate from development target/ (CI: target/release-packaging)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=packaging/scripts/common.sh
source "$REPO_ROOT/packaging/scripts/common.sh"

TEMPLATE="$REPO_ROOT/packaging/windows/Maybraid"
ASSETS="$REPO_ROOT/maybraid/assets"
VERSION="${VERSION:-0.0.1}"
DIST="$REPO_ROOT/dist"
OUT="$DIST/Maybraid-${VERSION}-windows-x64"
ZIP="${OUT}.zip"
TARGET_DIR="$(maybraid_resolve_target_dir)"
EXE="$TARGET_DIR/x86_64-pc-windows-msvc/release/maybraid.exe"

if [[ "${SKIP_BUILD:-}" != "1" ]]; then
	echo "==> Building maybraid (release, Windows x64)"
	echo "    Target:   x86_64-pc-windows-msvc"
	echo "    Profile:  release"
	echo "    Features: default (maybraid has no optional package features)"
	echo "    Locked:   Cargo.lock (--locked)"
	echo "    Build dir: $TARGET_DIR"
	(
		cd "$REPO_ROOT"
		cargo build -p maybraid \
			--release \
			--locked \
			--target x86_64-pc-windows-msvc
	)
fi

if [[ ! -f "$EXE" ]]; then
	echo "❌ maybraid.exe not found: $EXE" >&2
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

cat > "$OUT/README-WINDOWS.txt" << EOF
Maybraid ${VERSION} for Windows x64

Extract this folder and run Maybraid.exe. Assets are the assets/ directory
next to the executable; they are found regardless of the working directory.

This build uses the dynamic MSVC CRT. Install the Microsoft Visual C++
Redistributable for Visual Studio 2015-2022 (x64) if Windows reports a
missing VCRUNTIME140.dll or similar:

  https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist

Saves: %APPDATA%\\Maybraid\\saves
EOF

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

echo "==> Smoke test: extracted layout"
EXTRACT_SMOKE="$DIST/windows-smoke"
rm -rf "$EXTRACT_SMOKE"
mkdir -p "$EXTRACT_SMOKE"
if command -v unzip >/dev/null; then
	unzip -q "$ZIP" -d "$EXTRACT_SMOKE"
elif command -v powershell.exe >/dev/null; then
	src="$ZIP"
	dest="$EXTRACT_SMOKE"
	if command -v cygpath >/dev/null; then
		src="$(cygpath -w "$ZIP")"
		dest="$(cygpath -w "$EXTRACT_SMOKE")"
	fi
	powershell.exe -NoProfile -Command \
		"Expand-Archive -LiteralPath '${src}' -DestinationPath '${dest}' -Force"
fi

SMOKE_EXE="$(find "$EXTRACT_SMOKE" -name Maybraid.exe -type f | head -1)"
if [[ -z "$SMOKE_EXE" ]]; then
	echo "❌ Extracted zip did not contain Maybraid.exe" >&2
	exit 1
fi
echo "    extracted: $SMOKE_EXE"

if command -v timeout >/dev/null 2>&1 && [[ "$(uname -s)" != *NT* && "$(uname -o 2>/dev/null)" != "Msys" ]]; then
	maybraid_smoke_unix "$SMOKE_EXE" "Windows Maybraid.exe" || true
else
	echo "⚠️  Windows CI loader check is limited (no Unix timeout / no GPU)."
	echo "    Confirm VCRUNTIME140.dll is present on the machine before playtesting."
fi
rm -rf "$EXTRACT_SMOKE"

echo
echo "Created Windows package:"
echo "  $OUT/"
echo "  $ZIP"
echo
echo "Extract and run Maybraid.exe (needs the VC++ 2015-2022 x64 runtime)."
