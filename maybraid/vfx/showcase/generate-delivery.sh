#!/usr/bin/env bash
# Copy canonical dust runtime sources into the review delivery tree.
# Usage:
#   maybraid/vfx/showcase/generate-delivery.sh           # write copies
#   maybraid/vfx/showcase/generate-delivery.sh --check    # fail if copies drifted
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SHOWCASE="$ROOT/maybraid/vfx/showcase"
CANON_SHADER="$ROOT/maybraid/vfx/lib/src/dust/shader.wgsl"
CHANGED="$SHOWCASE/source/changed-files"
CHECK=0
if [[ "${1:-}" == "--check" ]]; then
	CHECK=1
fi

copy_file() {
	local src="$1"
	local dest="$2"
	mkdir -p "$(dirname "$dest")"
	if [[ "$CHECK" -eq 1 ]]; then
		if ! cmp -s "$src" "$dest"; then
			echo "delivery copy drifted: $dest" >&2
			diff -u "$src" "$dest" >&2 || true
			exit 1
		fi
		return
	fi
	cp "$src" "$dest"
}

RUNTIME_SOURCES=(
	"maybraid/vfx/lib/src/dust.rs"
	"maybraid/vfx/lib/src/dust/assets.rs"
	"maybraid/vfx/lib/src/dust/bounds.rs"
	"maybraid/vfx/lib/src/dust/disturbance.rs"
	"maybraid/vfx/lib/src/dust/material.rs"
	"maybraid/vfx/lib/src/dust/playback.rs"
	"maybraid/vfx/lib/src/dust/shader.wgsl"
	"maybraid/vfx/playground/examples/matched_dust.rs"
)

copy_file "$CANON_SHADER" "$SHOWCASE/runtime/shaders/shader.wgsl"
for rel in "${RUNTIME_SOURCES[@]}"; do
	copy_file "$ROOT/$rel" "$CHANGED/$rel"
done

if [[ "$CHECK" -eq 1 ]]; then
	echo "delivery copies match canonical sources"
	exit 0
fi

# Drop the previous example-private copies so they cannot drift from the crate.
rm -rf "$CHANGED/maybraid/vfx/playground/examples/matched_dust"

PATCH="$SHOWCASE/source/milestone21.patch"
: > "$PATCH"
{
	echo "# Generated from the workspace dust runtime and matched_dust example."
	echo "# This is a review snapshot, not the supported build path."
	echo "# Build from the repository root with:"
	echo "#   cargo build --locked --release -p vfx-playground --example matched_dust"
	echo
} > "$PATCH"
for rel in "${RUNTIME_SOURCES[@]}"; do
	# git diff --no-index reports 1 when files differ.
	git -C "$ROOT" diff --no-index -- /dev/null "$rel" >> "$PATCH" || true
done

python3 - "$SHOWCASE" <<'PY'
import hashlib, json, os, sys
root = sys.argv[1]
manifest_path = os.path.join(root, "SHA256.json")
with open(manifest_path) as f:
    manifest = json.load(f)
keep_missing = {"runtime/matched_dust": manifest.get("runtime/matched_dust")}
updated = {}
for dirpath, _, filenames in os.walk(root):
    for name in filenames:
        path = os.path.join(dirpath, name)
        rel = os.path.relpath(path, root).replace(os.sep, "/")
        if rel == "SHA256.json" or rel == "generate-delivery.sh":
            continue
        if name == ".DS_Store":
            continue
        with open(path, "rb") as f:
            updated[rel] = hashlib.sha256(f.read()).hexdigest()
if keep_missing["runtime/matched_dust"]:
    updated["runtime/matched_dust"] = keep_missing["runtime/matched_dust"]
with open(manifest_path, "w") as f:
    json.dump(dict(sorted(updated.items())), f, indent=2)
    f.write("\n")
print(f"updated {len(updated)} SHA256 entries")
PY
