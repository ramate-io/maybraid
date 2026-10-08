#!/usr/bin/env bash
# Build maybraid once inside Valve's Steam Runtime 3.0 "sniper" SDK.
#
#   nix develop .#release-linux --command packaging/scripts/build-linux-sniper.sh
#
# The compiler, headers, pkg-config metadata, and link libraries come from the
# pinned SDK image. Host/Nix library search paths are not passed in.
# The resulting x86_64 binary is reused by package-steam.sh and package-appimage.sh.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=packaging/scripts/common.sh
source "$REPO_ROOT/packaging/scripts/common.sh"

# Manifest digest of registry.gitlab.steamos.cloud/steamrt/sniper/sdk:latest
# as of 2026-10-07 (config blob sha256:901b229c… is not a pullable image id).
# Override with SNIPER_IMAGE if you need a newer SDK.
SNIPER_IMAGE="${SNIPER_IMAGE:-registry.gitlab.steamos.cloud/steamrt/sniper/sdk@sha256:1c33c507bc75d012e77df5727f93b0d5b8c3f7c8d4142ba5f7a16882cc92e014}"

SNIPER_TARGET_DIR="$REPO_ROOT/target/sniper-release"
BINARY_OUT="$(maybraid_sniper_bin)"
RUST_HOME="$SNIPER_TARGET_DIR/rust-home"

echo "==> Building maybraid for Linux (Steam Runtime 3.0 sniper SDK)"
echo "    SDK image: $SNIPER_IMAGE"
echo "    Target:    x86_64-unknown-linux-gnu"
echo "    Profile:   release"
echo "    Features:  default (maybraid has no optional package features)"
echo "    Locked:    Cargo.lock (--locked)"
echo "    Build dir: $SNIPER_TARGET_DIR"

CONTAINER_CMD="$(maybraid_container_cmd)"
if [[ -z "$CONTAINER_CMD" ]]; then
	echo "❌ Neither docker nor podman found. Install one to build with the sniper SDK." >&2
	exit 1
fi
echo "    Container: $CONTAINER_CMD"

mkdir -p "$SNIPER_TARGET_DIR" "$RUST_HOME/cargo" "$RUST_HOME/rustup"

# Run as the image user (root) so rustup's HOME/passwd check succeeds.
# rustup/cargo live under the workspace mount; chown back to the host user after.
# Do not set HOME, LIBRARY_PATH, or other host/Nix search paths.
$CONTAINER_CMD run --rm \
	--user 0:0 \
	--entrypoint bash \
	-v "$REPO_ROOT:/workspace:rw" \
	-w /workspace \
	-e CARGO_HOME=/workspace/target/sniper-release/rust-home/cargo \
	-e RUSTUP_HOME=/workspace/target/sniper-release/rust-home/rustup \
	-e CARGO_TARGET_DIR=/workspace/target/sniper-release \
	-e HOST_UID="$(id -u)" \
	-e HOST_GID="$(id -g)" \
	"$SNIPER_IMAGE" \
	-lc '
		set -euo pipefail
		unset LIBRARY_PATH LD_LIBRARY_PATH CPATH C_INCLUDE_PATH CPLUS_INCLUDE_PATH
		unset PKG_CONFIG_PATH NIX_LDFLAGS NIX_CFLAGS_COMPILE NIX_CC || true

		mkdir -p "$CARGO_HOME/bin" "$RUSTUP_HOME"
		export PATH="$CARGO_HOME/bin:$PATH"

		if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
			echo "==> Installing rustup into the SDK container"
			curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | \
				sh -s -- -y --default-toolchain none --profile minimal --no-modify-path
		fi

		echo "==> Installing pinned toolchain from rust-toolchain.toml"
		rustup show

		echo "==> Building release binary"
		cargo build -p maybraid \
			--release \
			--locked \
			--target x86_64-unknown-linux-gnu

		chown -R "$HOST_UID:$HOST_GID" /workspace/target/sniper-release || true
		echo "==> Build complete"
	'

if [[ ! -f "$BINARY_OUT" ]]; then
	echo "❌ Build failed: binary not found at $BINARY_OUT" >&2
	exit 1
fi
chmod +x "$BINARY_OUT"

maybraid_validate_linux_elf "$BINARY_OUT"

echo
echo "Built: $BINARY_OUT"
echo "Reuse this file for Steam depot staging and the standalone AppImage."
