#!/usr/bin/env bash
# Fit body clothes onto bind-pose biped bodies.
#
#   scripts/clothes-fit/fit.sh --clothes tank_top --body igeo_biped_full_body
#   scripts/clothes-fit/fit.sh --all
#
# Canonical garments in maybraid/art/characters/clothes/body/ are not modified.
# Fitted GLBs are written to
# maybraid/assets/characters/clothes/body/{body}/{garment}.glb
#
# Outside wrap onto an inflated body, a little body-normal ease, Cloth drape,
# light smooth, then Outside keep-out on the render body.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FIT_SCRIPT="$REPO_ROOT/scripts/clothes-fit/main.py"
CLOTHES_DIR="$REPO_ROOT/maybraid/art/characters/clothes/body"
BODIES_DIR="$REPO_ROOT/maybraid/art/characters/bodies/biped"
OUT_DIR="$REPO_ROOT/maybraid/assets/characters/clothes/body"

SKIP_BODY_PATTERNS='_rig|_playground|_parts'
SKIP_CLOTHES='proto_robe'

INFLATE=0.04
EASE=0.02
COLLISION_GAP=0.015
CLOTH_FRAMES=24
SMOOTH=3
SMOOTH_FACTOR=0.35
KEEP_OUT=0.02
ALL=0
CLOTHES_STEMS=()
BODY_STEMS=()

usage() {
    cat <<EOF
Usage:
  $(basename "$0") --clothes <stem> --body <stem> [options]
  $(basename "$0") --all [options]

  --clothes         Clothing blend stem (repeatable). File: ${CLOTHES_DIR}/<stem>.blend
  --body            Bind-pose body blend stem (repeatable). File: ${BODIES_DIR}/<stem>.blend
  --all             Fit every body-slot garment onto every biped mesh body
  --inflate         Inflate the Outside-wrap target, meters (default: ${INFLATE})
  --ease            Extra push along body normals after wrap, meters (default: ${EASE})
  --collision-gap   Cloth vs body thickness, meters (default: ${COLLISION_GAP})
  --cloth-frames    Cloth simulation frames (default: ${CLOTH_FRAMES}; 0 skips cloth)
  --smooth          Smooth iterations after cloth (default: ${SMOOTH}; 0 skips)
  --smooth-factor   Smooth strength per iteration (default: ${SMOOTH_FACTOR})
  --keep-out        Post-cloth Outside clearance, meters (default: ${KEEP_OUT}; negative skips)
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --clothes)
            CLOTHES_STEMS+=("$2")
            shift 2
            ;;
        --body)
            BODY_STEMS+=("$2")
            shift 2
            ;;
        --all)
            ALL=1
            shift
            ;;
        --inflate)
            INFLATE="$2"
            shift 2
            ;;
        --ease)
            EASE="$2"
            shift 2
            ;;
        --collision-gap)
            COLLISION_GAP="$2"
            shift 2
            ;;
        --cloth-frames)
            CLOTH_FRAMES="$2"
            shift 2
            ;;
        --smooth)
            SMOOTH="$2"
            shift 2
            ;;
        --smooth-factor)
            SMOOTH_FACTOR="$2"
            shift 2
            ;;
        --keep-out)
            KEEP_OUT="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if ! command -v blender >/dev/null 2>&1; then
    echo "blender command not found." >&2
    echo "   macOS: install Blender 5.1.2 to /Applications/Blender.app and enter nix develop" >&2
    exit 1
fi

if [ ! -f "$FIT_SCRIPT" ]; then
    echo "Fit script not found: $FIT_SCRIPT" >&2
    exit 1
fi

list_clothes() {
    find "$CLOTHES_DIR" -maxdepth 1 -type f -name '*.blend' -print \
        | sed 's|.*/||; s|\.blend$||' \
        | grep -v -E "^${SKIP_CLOTHES}$" \
        | sort
}

list_bodies() {
    find "$BODIES_DIR" -maxdepth 1 -type f -name '*.blend' -print \
        | sed 's|.*/||; s|\.blend$||' \
        | grep -v -E "$SKIP_BODY_PATTERNS" \
        | sort
}

if [ "$ALL" -eq 1 ]; then
    while IFS= read -r stem; do
        CLOTHES_STEMS+=("$stem")
    done < <(list_clothes)
    while IFS= read -r stem; do
        BODY_STEMS+=("$stem")
    done < <(list_bodies)
fi

if [ "${#CLOTHES_STEMS[@]}" -eq 0 ] || [ "${#BODY_STEMS[@]}" -eq 0 ]; then
    echo "Specify --clothes and --body, or --all." >&2
    usage >&2
    exit 1
fi

fitted=0
for clothes in "${CLOTHES_STEMS[@]}"; do
    clothes_blend="$CLOTHES_DIR/${clothes}.blend"
    if [ ! -f "$clothes_blend" ]; then
        echo "Clothing blend not found: $clothes_blend" >&2
        exit 1
    fi
    for body in "${BODY_STEMS[@]}"; do
        body_blend="$BODIES_DIR/${body}.blend"
        if [ ! -f "$body_blend" ]; then
            echo "Body blend not found: $body_blend" >&2
            exit 1
        fi
        out="$OUT_DIR/${body}/${clothes}.glb"
        mkdir -p "$(dirname "$out")"
        echo "Fitting ${clothes} → ${body}"
        if ! blender --background "$clothes_blend" --python "$FIT_SCRIPT" -- \
            --body "$body_blend" \
            --output "$out" \
            --inflate "$INFLATE" \
            --ease "$EASE" \
            --collision-gap "$COLLISION_GAP" \
            --cloth-frames "$CLOTH_FRAMES" \
            --smooth "$SMOOTH" \
            --smooth-factor "$SMOOTH_FACTOR" \
            --keep-out "$KEEP_OUT"; then
            echo "Failed to fit ${clothes} onto ${body}" >&2
            exit 1
        fi
        fitted=$((fitted + 1))
    done
done

echo "Fitted ${fitted} garment/body pair(s)."
