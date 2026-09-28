#!/usr/bin/env bash
# Copy each .wav under maybraid/art/sound-effects to the matching assets path.
#
#   scripts/sound-effects/sync.sh
#
#   maybraid/art/sound-effects/weapons/firearms/foo.wav
#     → maybraid/assets/sound-effects/weapons/firearms/foo.wav
#
# Logic projects (*.logicx) stay in art. Destination .wav files with no art
# counterpart are removed so renamed clips do not linger.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ART_DIR="$REPO_ROOT/maybraid/art/sound-effects"
ASSETS_DIR="$REPO_ROOT/maybraid/assets/sound-effects"

if [ ! -d "$ART_DIR" ]; then
	echo "Art directory not found: $ART_DIR" >&2
	exit 1
fi

# Logic packages are directories; their media/undo WAVs are not runtime clips.
list_runtime_wavs() {
	find "$1" \( -name '*.logicx' -o -name '*.nosync' \) -prune -o -type f -name '*.wav' -print
}

copied=0
found=0
while IFS= read -r wav; do
	[ -z "$wav" ] && continue
	found=1
	rel="${wav#"$ART_DIR"/}"
	out="$ASSETS_DIR/$rel"
	mkdir -p "$(dirname "$out")"
	echo "Copying ${rel}"
	cp "$wav" "$out"
	copied=$((copied + 1))
done < <(list_runtime_wavs "$ART_DIR" | sort)

if [ "$found" -eq 0 ]; then
	echo "No .wav files found under $ART_DIR"
	exit 0
fi

removed=0
if [ -d "$ASSETS_DIR" ]; then
	while IFS= read -r bundle; do
		[ -z "$bundle" ] && continue
		echo "Removing ${bundle#"$ASSETS_DIR"/}"
		rm -rf "$bundle"
	done < <(find "$ASSETS_DIR" -name '*.logicx' -prune -print)
	while IFS= read -r dest; do
		[ -z "$dest" ] && continue
		rel="${dest#"$ASSETS_DIR"/}"
		if [ ! -f "$ART_DIR/$rel" ]; then
			echo "Removing stale ${rel}"
			rm "$dest"
			removed=$((removed + 1))
		fi
	done < <(find "$ASSETS_DIR" -type f -name '*.wav' | sort)
	find "$ASSETS_DIR" -type d -empty -delete
fi

echo "Copied ${copied} wav(s). Removed ${removed} stale wav(s)."
