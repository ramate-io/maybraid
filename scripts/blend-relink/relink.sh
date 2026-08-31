#!/usr/bin/env bash
# Relink Blender libraries after the bodies/clothes subdirectory reorg.
#
#   scripts/blend-relink/relink.sh [blend ...]
#
# With no arguments, relinks every tracked .blend under
# maybraid/art/characters/{bodies,clothes}.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RELINK_SCRIPT="$REPO_ROOT/scripts/blend-relink/main.py"
ART_ROOT="$REPO_ROOT/maybraid/art/characters"

MAP_JSON="$(python3 - <<PY
import json
art = "$ART_ROOT"
print(json.dumps({
    "humanoid_rig.blend": f"{art}/bodies/biped/humanoid_rig.blend",
    "quadruped_rig.blend": f"{art}/bodies/quadruped/quadruped_rig.blend",
    "forelimbed_rig.blend": f"{art}/bodies/forelimbed/forelimbed_rig.blend",
    "humanoid_full_body.blend": f"{art}/bodies/biped/humanoid_full_body.blend",
    "meerkat_head.blend": f"{art}/heads/meerkat_head.blend",
}))
PY
)"

if ! command -v blender >/dev/null 2>&1; then
    echo "blender command not found." >&2
    echo "   macOS: install Blender 5.1.2 to /Applications/Blender.app and enter nix develop" >&2
    exit 1
fi

if [ "$#" -gt 0 ]; then
    blends=("$@")
else
    blends=()
    while IFS= read -r blend; do
        [ -n "$blend" ] && blends+=("$blend")
    done < <(find "$ART_ROOT/bodies" "$ART_ROOT/clothes" -type f -name '*.blend' | sort)
fi

if [ "${#blends[@]}" -eq 0 ]; then
    echo "No .blend files to relink."
    exit 0
fi

relinked=0
for blend in "${blends[@]}"; do
    echo "Relinking ${blend}"
    if ! blender --background "$blend" --python "$RELINK_SCRIPT" -- --map-json "$MAP_JSON"; then
        echo "Failed to relink ${blend}" >&2
        exit 1
    fi
    relinked=$((relinked + 1))
done

echo "Relinked ${relinked} .blend file(s)."
