#!/usr/bin/env bash
# Short motion clips for God Beam / sonic-wave PR evidence.
set -euo pipefail
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$root"
export CXX=g++-13 CC=gcc-13 LIBRARY_PATH=/usr/lib/gcc/x86_64-linux-gnu/13
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
export WGPU_BACKEND=vk
export VFX_CAPTURE=1 VFX_NOLOOP=1
out="$root/maybraid/vfx/playground/evidence"
mkdir -p "$out"
bin="${CARGO_TARGET_DIR:-$root/target}/release/vfx-playground"
if [[ ! -x "$bin" ]]; then
	cargo build --release -p vfx-playground --locked
fi

record() {
	local name="$1"
	local duration="$2"
	shift 2
	xvfb-run -a -s "-screen 0 1280x720x24" bash -c "
		set -e
		export DISPLAY
		$bin \"\$@\" &
		pid=\$!
		sleep 0.8
		ffmpeg -y -loglevel error -f x11grab -video_size 1280x720 -framerate 30 -i \"\$DISPLAY\" \
			-t \"$duration\" -c:v libx264 -pix_fmt yuv420p \"$out/$name\"
		kill \$pid 2>/dev/null || true
		wait \$pid 2>/dev/null || true
	" -- "$@"
}

export VFX_FRAME=side
record sonic_boss_20m_side_day.mp4 3.2 sonic-wave --range 20 --speed 22 --count 3 --radius 0.85 --thickness 0.70 --interval 0.28 --seed 2
unset VFX_FRAME
export VFX_FRAME=three-quarter
record sonic_boss_20m_three_quarter_day.mp4 3.2 sonic-wave --range 20 --speed 22 --count 3 --radius 0.85 --thickness 0.70 --interval 0.28 --seed 2
unset VFX_FRAME

export VFX_FRAME=side
record sonic_ordinary_8m_side_day.mp4 2.4 sonic-wave --range 8 --speed 16 --count 3 --radius 0.4 --thickness 0.40 --interval 0.18 --seed 2
export VFX_FRAME=three-quarter
record sonic_ordinary_8m_three_quarter_day.mp4 2.4 sonic-wave --range 8 --speed 16 --count 3 --radius 0.4 --thickness 0.40 --interval 0.18 --seed 2
unset VFX_FRAME

export VFX_NIGHT=1 VFX_FRAME=three-quarter
record sonic_boss_20m_three_quarter_night.mp4 3.2 sonic-wave --range 20 --speed 22 --count 3 --radius 0.85 --thickness 0.70 --interval 0.28 --seed 2
unset VFX_NIGHT VFX_FRAME

export VFX_CANCEL_AT=1.1
record sonic_cancel_mid_burst.mp4 2.6 sonic-wave --range 20 --speed 22 --count 3 --radius 0.85 --thickness 0.70 --interval 0.28 --seed 2
unset VFX_CANCEL_AT

record god_beam_7m_side_day.mp4 4.0 god-beam --width 7 --height 96 --lifetime 10 --seed 2
export VFX_FRAME=base
record god_beam_7m_scale_day.mp4 3.0 god-beam --width 7 --height 96 --lifetime 10 --seed 2
unset VFX_FRAME
export VFX_NIGHT=1
record god_beam_7m_side_night.mp4 4.0 god-beam --width 7 --height 96 --lifetime 10 --seed 2
unset VFX_NIGHT

echo "Wrote motion clips to $out"
