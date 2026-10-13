#!/usr/bin/env bash
# Sample RSS / phys_footprint / GPU-ish bytes during one scripted flight.
# Run from the repo root inside `nix develop`. Does not start tracy-capture
# or xctrace — this machine is one game or capture at a time.
#
#   profiling/memory-flight.sh <scenario|script.jsonl> <phase> <repeat>
#
# Daily set: crowd-0, dusk-night-raid (dusk-spawn), biome-traverse.
# Always MAYBRAID_VSYNC=off. Never send `stats vsync`. Stay windowed.
set -euo pipefail

usage="usage: profiling/memory-flight.sh <scenario|script.jsonl> <phase> <repeat>"
target="${1:?$usage}"
phase="${2:?$usage}"
repeat="${3:?$usage}"

root="$(cd "$(dirname "$0")/.." && pwd)"
source "$root/profiling/resolve-script.sh"
resolve_script "$target"
cd "$root"

bin="${MAYBRAID_BIN:-$root/target/release/maybraid}"
if [[ ! -x "$bin" ]]; then
	echo "missing $bin — build with:" >&2
	echo "  cargo build -p maybraid --release" >&2
	echo "  (do not enable bevy/trace_tracy — with no listener it buffers zones)" >&2
	exit 2
fi

if [[ "${MAYBRAID_FULLSCREEN:-}" == 1 || "${MAYBRAID_FULLSCREEN:-}" == true ]]; then
	echo "refusing MAYBRAID_FULLSCREEN — stay windowed on this machine" >&2
	exit 2
fi
unset MAYBRAID_FULLSCREEN || true

# Tracy builds buffer zones until tracy-capture attaches. That is not a
# memory flight — refuse a binary that looks Tracy-enabled.
if command -v strings >/dev/null && strings "$bin" | grep -q 'tracy-client'; then
	echo "refusing $bin: looks like a bevy/trace_tracy build" >&2
	echo "memory flights use a plain release; Tracy is for tracy-flight.sh only" >&2
	exit 2
fi

# MAYBRAID_BIN copies are named maybraid-base / maybraid-after, not maybraid.
live="$(pgrep -x maybraid || true; pgrep -x maybraid-base || true; pgrep -x maybraid-after || true)"
if [[ -n "$live" ]]; then
	echo "maybraid is already running (pid $live); this machine is one-at-a-time" >&2
	exit 3
fi
if pgrep -x tracy-capture >/dev/null || pgrep -x tracy-profiler >/dev/null || pgrep -x tracy >/dev/null; then
	echo "a Tracy listener is already running" >&2
	exit 3
fi
if pgrep -x xctrace >/dev/null; then
	echo "xctrace is already running" >&2
	exit 3
fi

# Display must be awake: this machine has panicked the display driver when
# a flight ran against a sleeping panel. Online + not-asleep + caffeinate -d.
if ! system_profiler SPDisplaysDataType 2>/dev/null | grep -q 'Online: Yes'; then
	echo "no online display — wake the panel before profiling" >&2
	exit 3
fi
if system_profiler SPDisplaysDataType 2>/dev/null | grep -q 'Display Asleep: Yes'; then
	echo "display is asleep — wake the panel before profiling" >&2
	exit 3
fi

case "$scenario" in
	origin-idle) timeout_secs=180 ;;
	origin-walk) timeout_secs=120 ;;
	locomotion-mix) timeout_secs=240 ;;
	biome-traverse) timeout_secs=420 ;;
	*) timeout_secs=300 ;;
esac
timeout_secs="${MAYBRAID_FLIGHT_TIMEOUT:-$timeout_secs}"
sample_hz="${MAYBRAID_MEMORY_SAMPLE_SECS:-1}"
footprint_every="${MAYBRAID_FOOTPRINT_EVERY:-2}"
# Protect this machine: a Tracy build without tracy-capture, or a leak,
# can fill RAM. Default cap is 8 GiB phys / RSS.
cap_bytes="${MAYBRAID_MEMORY_CAP_BYTES:-$((8 * 1024 * 1024 * 1024))}"
cap_rss_kb="$((cap_bytes / 1024))"

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
captures_root="${MAYBRAID_TRACY_CAPTURES:-$root/profiling/captures}"
out_dir="${captures_root}/memory/${phase}/${scenario}/r${repeat}"
mkdir -p "$out_dir"
base="${out_dir}/${scenario}-${phase}-r${repeat}-${stamp}"

events_jsonl="${base}-events.jsonl"
meta_file="${base}-meta.txt"
samples_csv="${base}-memory.csv"
summary_txt="${base}-memory.txt"
summary_json="${base}-memory.json"
game_log="${base}-game.log"
vmmap_txt="${base}-vmmap.txt"

{
	echo "scenario=$scenario"
	echo "phase=$phase"
	echo "repeat=$repeat"
	echo "stamp=$stamp"
	echo "binary=$bin"
	echo "script=$script"
	echo "vsync=off"
	echo "fullscreen=off"
	echo "sample_secs=$sample_hz"
	echo "footprint_every=$footprint_every"
	echo "memory_cap_bytes=$cap_bytes"
	echo "tracy=off (plain release only; Tracy builds must attach tracy-capture)"
	if [[ "${MAYBRAID_NO_START_AT:-}" == 1 ]]; then
		echo "start_at= (Home-first; picker and per-character resume stay live)"
	else
		echo "start_at=0,0 (a leading script start overrides)"
	fi
	echo "commit=$(git -C "$root" rev-parse HEAD 2>/dev/null || true)"
	echo "hw=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || true)"
	echo "model=$(sysctl -n hw.model 2>/dev/null || true)"
	echo "mem_bytes=$(sysctl -n hw.memsize 2>/dev/null || true)"
} >"$meta_file"

echo "t_s,rss_kb,phys_bytes,gpu_bytes" >"$samples_csv"

game_pid=""
front_pid=""
caffeinate_pid=""
vmmap_taken=0

cleanup() {
	if [[ -n "${front_pid:-}" ]]; then
		kill "$front_pid" 2>/dev/null || true
	fi
	if [[ -n "${caffeinate_pid:-}" ]]; then
		kill "$caffeinate_pid" 2>/dev/null || true
	fi
	if [[ -n "${game_pid:-}" ]] && kill -0 "$game_pid" 2>/dev/null; then
		kill "$game_pid" 2>/dev/null || true
		wait "$game_pid" 2>/dev/null || true
	fi
}
trap cleanup EXIT

# Keep the panel awake for the flight; kernel panics followed a sleeping display.
caffeinate -d -i >/dev/null 2>&1 &
caffeinate_pid=$!

export MAYBRAID_VSYNC=off
game_args=(--agent-script "$script" --agent-events "$events_jsonl" --agent-quit)
if [[ "${MAYBRAID_NO_START_AT:-}" != 1 ]]; then
	game_args=(--start-at=0,0 "${game_args[@]}")
fi

echo "memory $scenario $phase r$repeat → $out_dir"
# Windowed only. Do not pass MAYBRAID_FULLSCREEN.
# caffeinate is a sibling, not the parent, so samples hit the game pid.
"$bin" "${game_args[@]}" >"$game_log" 2>&1 &
game_pid=$!

# macOS throttles occluded windows.
(
	while kill -0 "$game_pid" 2>/dev/null; do
		osascript -e 'tell application "System Events" to set frontmost of (first process whose name contains "maybraid") to true' >/dev/null 2>&1 || true
		sleep 2
	done
) &
front_pid=$!

elapsed=0
while kill -0 "$game_pid" 2>/dev/null; do
	if (( elapsed >= timeout_secs )); then
		echo "game exceeded ${timeout_secs}s — stopping" >&2
		kill "$game_pid" 2>/dev/null || true
		break
	fi
	rss="$(ps -o rss= -p "$game_pid" 2>/dev/null | tr -d ' ' || true)"
	phys=""
	gpu=""
	if (( elapsed % footprint_every == 0 )) && command -v footprint >/dev/null; then
		parsed="$(footprint "$game_pid" 2>/dev/null | python3 "$root/profiling/sample-footprint.py" || true)"
		phys="${parsed%%,*}"
		gpu="${parsed#*,}"
		if [[ "$gpu" == "$parsed" ]]; then
			gpu=""
		fi
	fi
	echo "${elapsed},${rss},${phys},${gpu}" >>"$samples_csv"

	if [[ -n "$rss" && "$rss" -gt "$cap_rss_kb" ]]; then
		echo "RSS ${rss} KB exceeds ${cap_rss_kb} KB cap — killing maybraid" >&2
		echo "killed=rss_cap" >>"$meta_file"
		kill "$game_pid" 2>/dev/null || true
		break
	fi
	if [[ -n "$phys" && "$phys" -gt "$cap_bytes" ]]; then
		echo "phys_footprint ${phys} B exceeds ${cap_bytes} B cap — killing maybraid" >&2
		echo "killed=phys_cap" >>"$meta_file"
		kill "$game_pid" 2>/dev/null || true
		break
	fi

	# One vmmap during the measure window (after warmup), never at quit.
	if (( vmmap_taken == 0 )) && [[ -s "$events_jsonl" ]] && grep -q "\"name\":\"${scenario}\",\"mark\":\"start\"" "$events_jsonl" 2>/dev/null; then
		vmmap --summary "$game_pid" >"$vmmap_txt" 2>/dev/null || true
		vmmap_taken=1
	fi

	sleep "$sample_hz"
	elapsed=$((elapsed + sample_hz))
done
wait "$game_pid" 2>/dev/null || true
game_pid=""

python3 "$root/profiling/summarize-memory.py" \
	--samples "$samples_csv" \
	--events "$events_jsonl" \
	--scenario "$scenario" \
	--json "$summary_json" \
	--out "$summary_txt" || echo "memory summary failed" >&2

echo "wrote $samples_csv"
echo "wrote $summary_txt"
ls -lh "$samples_csv" "$summary_txt" "$events_jsonl" 2>/dev/null || true
cat "$summary_txt" 2>/dev/null || true
