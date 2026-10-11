#!/usr/bin/env bash
# Capture eye-glow preview frames under xvfb (off / on / mid + night distance).
set -euo pipefail
WORKSPACE="$(cd "$(dirname "$0")/../../../../../../.." && pwd)"
BIN="$WORKSPACE/target/debug/characters-playground"
PREVIEW="$WORKSPACE/maybraid/world/layers/mobs/characters/playground/preview/eye-glow-1129"
mkdir -p "$PREVIEW"

export WGPU_BACKEND=vk
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
export MAYBRAID_PLAYGROUND_CAPTURE=1
export MAYBRAID_FACE_CAPTURE_RANGE=close

run_frame() {
	local species="$1"
	local prefix="$2"
	local glow_state="$3"
	local extra_env="${4:-}"
	local script_path="$PREVIEW/.capture-${species}.script"
	cat >"$script_path" <<EOF
character assemble --species ${species} --animation still
EOF
	xvfb-run -a -s "-screen 0 1280x720x24" bash -c "
		set -e
		env MAYBRAID_EYE_GLOW_DEMO='$glow_state' $extra_env '$BIN' script --path '$script_path' &
		app_pid=\$!
		sleep 35
		scrot -o '$PREVIEW/${prefix}-${glow_state}.png'
		kill \$app_pid 2>/dev/null || true
		wait \$app_pid 2>/dev/null || true
	"
}

capture_species() {
	local species="$1"
	local prefix="$2"
	run_frame "$species" "$prefix" off ""
	run_frame "$species" "$prefix" on ""
	run_frame "$species" "$prefix" mid ""
}

capture_species epiphant epiphant-eye-glow
capture_species braidman braidman-eye-glow

cat >"$PREVIEW/.capture-night.script" <<EOF
character assemble --species braidman --animation still
EOF
export MAYBRAID_FACE_CAPTURE_RANGE=warning-distance
xvfb-run -a -s "-screen 0 1280x720x24" bash -c "
	env MAYBRAID_PLAYGROUND_CAPTURE_NIGHT=1 MAYBRAID_EYE_GLOW_DEMO=on '$BIN' script --path '$PREVIEW/.capture-night.script' &
	app_pid=\$!
	sleep 30
	scrot -o '$PREVIEW/braidman-eye-glow-night-distance.png'
	kill \$app_pid 2>/dev/null || true
	wait \$app_pid 2>/dev/null || true
"

rm -f "$PREVIEW"/.capture-*.script
echo "Wrote previews to $PREVIEW"
