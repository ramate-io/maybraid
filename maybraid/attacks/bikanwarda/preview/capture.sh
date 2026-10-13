#!/usr/bin/env bash
# Headless capture of bikanwarda-attacks-playground.
# Bevy Screenshot GPU-readback writes the PNGs; X11 scrot of lavapipe stays black.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
PREVIEW="$(cd "$(dirname "$0")" && pwd)"
BIN="${BIKANWARDA_PLAYGROUND_BIN:-$ROOT/target/debug/bikanwarda-attacks-playground}"

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
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/bikanwarda-attacks-xdg-runtime}"
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
export BIKANWARDA_PREVIEW_DIR="$PREVIEW"
rm -f "$PREVIEW"/{parade,god-beam,bray}-*.png "$PREVIEW/cancelled.png"

if [[ ! -x "$BIN" ]]; then
	echo "missing $BIN; build with: cargo build -p bikanwarda-attacks-playground" >&2
	exit 1
fi

"$BIN" --capture
echo "wrote stills under $PREVIEW"
