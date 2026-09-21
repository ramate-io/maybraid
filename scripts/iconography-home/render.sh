#!/usr/bin/env bash
# Bake each *_logo_icon.blend under maybraid/art/iconography to a home-screen PNG.
#
#   scripts/iconography-home/render.sh
#
#   maybraid/art/iconography/foo_logo_icon.blend
#     → maybraid/assets/iconography/foo_logo_icon_home.png
#
# TEXT_YELLOW mark on MENU_CLEAR wash. Does not replace the grayscale HUD
# mark written by scripts/iconography-png/render.sh.
#
# Same ortho frame as iconography-png: scale 2.2 at (0, −10, 0),
# rotation (90°, 0, 0), 512×512.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RENDER_SCRIPT="$REPO_ROOT/scripts/iconography-home/main.py"
ICON_DIR="$REPO_ROOT/maybraid/art/iconography"
ASSETS_DIR="$REPO_ROOT/maybraid/assets/iconography"

if ! command -v blender >/dev/null 2>&1; then
    echo "blender command not found." >&2
    echo "   macOS: install Blender 5.1.2 to /Applications/Blender.app and enter nix develop" >&2
    echo "   Linux/CI: ensure you're in the nix development shell" >&2
    exit 1
fi

if [ ! -f "$RENDER_SCRIPT" ]; then
    echo "Render script not found: $RENDER_SCRIPT" >&2
    exit 1
fi

if [ ! -d "$ICON_DIR" ]; then
    echo "Iconography directory not found: $ICON_DIR" >&2
    exit 1
fi

rendered=0
found=0
while IFS= read -r blend; do
    [ -z "$blend" ] && continue
    found=1
    rel="${blend#"$ICON_DIR"/}"
    out="$ASSETS_DIR/${rel%.blend}_home.png"
    mkdir -p "$(dirname "$out")"
    echo "Rendering ${blend} → ${out}"
    if ! blender --background "$blend" --python "$RENDER_SCRIPT" -- "$out"; then
        echo "Failed to render ${blend}" >&2
        echo "   blender --background \"${blend}\" --python \"${RENDER_SCRIPT}\" -- \"${out}\"" >&2
        exit 1
    fi
    rendered=$((rendered + 1))
done < <(find "$ICON_DIR" -type f -name '*_logo_icon.blend' | sort)

if [ "$found" -eq 0 ]; then
    echo "No *_logo_icon.blend files found under $ICON_DIR"
    exit 0
fi

echo "Rendered ${rendered} home-screen logo PNG(s)."
