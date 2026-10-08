#!/usr/bin/env bash
# Shared helpers for packaging scripts. Source this file; do not execute it.

maybraid_resolve_target_dir() {
	local raw="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
	case "$raw" in
		/*) printf '%s\n' "$raw" ;;
		*) printf '%s\n' "$REPO_ROOT/$raw" ;;
	esac
}

maybraid_sniper_bin() {
	printf '%s\n' "$REPO_ROOT/target/sniper-release/x86_64-unknown-linux-gnu/release/maybraid"
}

maybraid_docker_usable() {
	command -v docker >/dev/null 2>&1 || return 1
	if command -v timeout >/dev/null 2>&1; then
		timeout 15 docker info >/dev/null 2>&1
	else
		docker info >/dev/null 2>&1
	fi
}

# Prefer a working docker daemon; fall back to podman. Empty if neither works.
maybraid_container_cmd() {
	if maybraid_docker_usable; then
		printf '%s\n' docker
	elif command -v podman >/dev/null 2>&1; then
		printf '%s\n' podman
	fi
}

maybraid_require_sniper_bin() {
	local bin="${MAYBRAID_LINUX_BIN:-$(maybraid_sniper_bin)}"
	if [[ ! -f "$bin" ]]; then
		echo "❌ Linux maybraid binary not found: $bin" >&2
		echo "   Run: nix develop .#release-linux --command packaging/scripts/build-linux-sniper.sh" >&2
		echo "   Override: MAYBRAID_LINUX_BIN=/path/to/maybraid" >&2
		exit 1
	fi
	printf '%s\n' "$bin"
}

# Host GPU / driver libraries are provided by the OS or Steam, never shipped.
maybraid_linux_gpu_lib() {
	case "$1" in
		libGL.so*|libEGL.so*|libGLdispatch.so*|libGLX.so*|libOpenGL.so*| \
		libvulkan.so*|libdrm.so*|libcuda.so*|libnvidia-*|libgbm.so*)
			return 0
			;;
	esac
	return 1
}

maybraid_validate_linux_elf() {
	local bin="$1"
	echo "==> Validating Linux ELF: $bin"

	if ! command -v readelf >/dev/null; then
		echo "⚠️  readelf not found; limited ELF checks" >&2
	else
		local interp
		interp="$(readelf -l "$bin" | sed -n 's/.*\[Requesting program interpreter: \(.*\)\]/\1/p')"
		echo "    ELF interpreter: ${interp:-unknown}"
		if [[ -n "$interp" && "$interp" != /lib64/ld-linux-x86-64.so.2 ]]; then
			echo "❌ Unexpected ELF interpreter: $interp" >&2
			exit 1
		fi

		local glibc
		glibc="$(readelf -V "$bin" 2>/dev/null | sed -n 's/.*GLIBC_//p' | sort -V | tail -1)"
		echo "    Max GLIBC symbol: ${glibc:-unknown}"

		if readelf -d "$bin" | grep -E 'RPATH|RUNPATH' | grep -Eq '/nix/store|/opt/homebrew|/usr/local|/home/'; then
			echo "❌ Binary rpath/runpath points at a build-machine location:" >&2
			readelf -d "$bin" | grep -E 'RPATH|RUNPATH' >&2
			exit 1
		fi
	fi

	if command -v ldd >/dev/null; then
		echo "    ldd:"
		ldd "$bin" | sed 's/^/      /' || true
		local missing=""
		while read -r line; do
			if [[ "$line" == *"not found"* ]]; then
				local soname
				soname="$(printf '%s\n' "$line" | awk '{print $1}')"
				if maybraid_linux_gpu_lib "$soname"; then
					echo "    allowing unresolved GPU/driver lib: $soname"
					continue
				fi
				missing+="    $line"$'\n'
			fi
		done < <(ldd "$bin" || true)
		if [[ -n "$missing" ]]; then
			echo "❌ Unresolved non-GPU libraries:" >&2
			printf '%s' "$missing" >&2
			exit 1
		fi
	fi

	if command -v strings >/dev/null && strings "$bin" | grep -q '/nix/store'; then
		echo "❌ Binary contains /nix/store references:" >&2
		strings "$bin" | grep '/nix/store' | head -5 | sed 's/^/    /' >&2
		exit 1
	fi

	echo "✅ Linux ELF validation passed"
}

maybraid_validate_macho() {
	local bin="$1"
	echo "==> Validating Mach-O: $bin"
	if ! command -v otool >/dev/null; then
		echo "⚠️  otool not found; skipping Mach-O validation" >&2
		return 0
	fi

	echo "    dependencies:"
	otool -L "$bin" | tail -n +2 | sed 's/^/      /'

	local deps
	deps="$(otool -L "$bin")"
	if printf '%s\n' "$deps" | grep -q '/nix/store'; then
		echo "❌ Binary contains /nix/store references:" >&2
		printf '%s\n' "$deps" | grep '/nix/store' | sed 's/^/    /' >&2
		exit 1
	fi
	if printf '%s\n' "$deps" | grep -q '/opt/homebrew'; then
		echo "❌ Binary contains /opt/homebrew references:" >&2
		printf '%s\n' "$deps" | grep '/opt/homebrew' | sed 's/^/    /' >&2
		exit 1
	fi
	if printf '%s\n' "$deps" | grep -Eq '/usr/local/(lib|opt)'; then
		echo "❌ Binary contains /usr/local references:" >&2
		printf '%s\n' "$deps" | grep -E '/usr/local/(lib|opt)' | sed 's/^/    /' >&2
		exit 1
	fi

	local rpaths
	rpaths="$(otool -l "$bin" | awk '/cmd LC_RPATH/{p=1} p && /path /{print $2; p=0}')"
	if [[ -n "$rpaths" ]]; then
		echo "    rpaths:"
		printf '%s\n' "$rpaths" | sed 's/^/      /'
		if printf '%s\n' "$rpaths" | grep -Eq '/nix/store|/opt/homebrew|/usr/local|/Users/'; then
			echo "❌ Binary rpath points at a build-machine location" >&2
			exit 1
		fi
	fi

	echo "✅ Mach-O dependencies are clean (system libs/frameworks and bundle-relative paths only)"
}

# Launch from /tmp so asset-path assumptions are visible. Fail on loader errors.
# Staying up until timeout is a loader/window check only — not full runtime success.
# A crash after start is reported as incomplete, not as a green runtime pass.
maybraid_smoke_unix() {
	local bin="$1"
	local label="$2"
	local timeout_sec="${3:-8}"
	local log pid elapsed=0
	log="$(mktemp)"

	echo "==> Smoke test: $label (from /tmp, ${timeout_sec}s)"
	(
		cd /tmp
		"$bin"
	) >"$log" 2>&1 &
	pid=$!

	while kill -0 "$pid" 2>/dev/null && (( elapsed < timeout_sec )); do
		sleep 1
		elapsed=$((elapsed + 1))
	done

	if kill -0 "$pid" 2>/dev/null; then
		pkill -P "$pid" 2>/dev/null || true
		kill -TERM "$pid" 2>/dev/null || true
		sleep 1
		pkill -KILL -P "$pid" 2>/dev/null || true
		kill -KILL "$pid" 2>/dev/null || true
		wait "$pid" 2>/dev/null || true
		sed -n '1,30p' "$log" | sed 's/^/    /'
		echo "✅ $label loader check: process stayed up ${timeout_sec}s"
		echo "   Limitation: CI GPU / windowing are not validated. This is not full runtime success."
		rm -f "$log"
		return 0
	fi

	wait "$pid" || true
	local code=$?
	echo "    process exited after ${elapsed}s with code $code"
	sed -n '1,40p' "$log" | sed 's/^/    /'
	if grep -Eiq 'Library not loaded|dyld\[|error while loading shared libraries|cannot open shared object|GLIBC_|Image not found|failed to map segment' "$log"; then
		echo "❌ $label failed to load required libraries" >&2
		rm -f "$log"
		return 1
	fi
	echo "⚠️  $label exited before the ${timeout_sec}s loader window (code $code)."
	echo "   Not counted as full runtime success. GPU/display-less CI often cannot complete a real launch."
	rm -f "$log"
	return 0
}
