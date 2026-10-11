#!/usr/bin/env bash
# Capture eye-glow preview frames under xvfb (off / on / mid).
set -euo pipefail
WORKSPACE="$(cd "$(dirname "$0")/../../../../../../.." && pwd)"
BIN="$WORKSPACE/target/debug/characters-playground"
OUT="${CURSOR_ARTIFACTS_DIR:-/opt/cursor/artifacts}/screenshots"
mkdir -p "$OUT"

export WGPU_BACKEND=vk
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json

capture_species() {
	local species="$1"
	local prefix="$2"
	local script_path="$WORKSPACE/maybraid/world/layers/mobs/characters/playground/scripts/eye-glow-${species}.script"
	cat >"$script_path" <<EOF
character assemble --species ${species}
character camera front
EOF
	for state in off on mid; do
		xvfb-run -a -s "-screen 0 1280x720x24" bash -c "
			env MAYBRAID_EYE_GLOW_DEMO='$state' '$BIN' script --path '$script_path' &
			app_pid=\$!
			sleep 26
			scrot -o '$OUT/${prefix}-${state}.png'
			kill \$app_pid 2>/dev/null || true
			wait \$app_pid 2>/dev/null || true
		"
	done
}

capture_species epiphant epiphant-eye-glow
capture_species braidman braidman-eye-glow
echo "Wrote previews to $OUT"
