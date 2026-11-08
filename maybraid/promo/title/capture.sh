#!/usr/bin/env bash
# Headless, frame-exact title-sequence capture.
# Bevy Screenshot GPU-readback writes the PNG sequence; ffmpeg encodes the MP4.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
BIN="${PROMO_TITLE_BIN:-$ROOT/target/release/promo-title}"

if [[ -f /usr/share/vulkan/icd.d/lvp_icd.json ]]; then
	export WGPU_BACKEND="${WGPU_BACKEND:-vulkan}"
	export VK_ICD_FILENAMES="${VK_ICD_FILENAMES:-/usr/share/vulkan/icd.d/lvp_icd.json}"
	export VK_DRIVER_FILES="${VK_DRIVER_FILES:-$VK_ICD_FILENAMES}"
else
	export WGPU_BACKEND="${WGPU_BACKEND:-gl}"
	export LIBGL_ALWAYS_SOFTWARE="${LIBGL_ALWAYS_SOFTWARE:-1}"
fi
export WINIT_UNIX_BACKEND=x11
export WAYLAND_DISPLAY=
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/promo-title-xdg-runtime}"
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"

if [[ ! -x "$BIN" ]]; then
	echo "building promo-title (release)…" >&2
	(cd "$ROOT" && cargo build -p promo-title --release)
fi

run_bin() {
	"$BIN" "$@"
}

if [[ -n "${DISPLAY:-}" && "${PROMO_TITLE_NO_XVFB:-0}" == "1" ]]; then
	run_bin "$@"
	exit 0
fi

if ! command -v xvfb-run >/dev/null 2>&1; then
	echo "xvfb-run not found; set PROMO_TITLE_NO_XVFB=1 to use the current DISPLAY" >&2
	run_bin "$@"
	exit 0
fi

# Match the requested size when present so the window is not letterboxed.
WIDTH=1920
HEIGHT=1080
prev=""
for arg in "$@"; do
	if [[ "$prev" == "--width" ]]; then WIDTH="$arg"; fi
	if [[ "$prev" == "--height" ]]; then HEIGHT="$arg"; fi
	prev="$arg"
done

xvfb-run -a -s "-screen 0 ${WIDTH}x${HEIGHT}x24" "$BIN" "$@"
