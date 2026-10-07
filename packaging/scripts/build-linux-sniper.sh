#!/usr/bin/env bash
# Build maybraid for Linux inside Valve's Steam Runtime 3.0 "sniper" SDK.
#
#   packaging/scripts/build-linux-sniper.sh
#
# Produces a single x86_64 Linux binary with sniper SDK glibc/dependencies.
# The binary is reused for both Steam depot and standalone AppImage packaging.
#
# Environment variables:
#   SNIPER_IMAGE  Steam runtime image (default: pinned digest)
#   VERSION       Game version (default: 0.0.1)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${VERSION:-0.0.1}"

# Pin Steam Runtime 3.0 sniper SDK by immutable digest
# Latest as of 2026-10-07: BUILD_ID 3.0.20250108.112707
# See: https://gitlab.steamos.cloud/steamrt/sniper/sdk
SNIPER_IMAGE="${SNIPER_IMAGE:-registry.gitlab.steamos.cloud/steamrt/sniper/sdk@sha256:901b229c92b4743f77f7ffe02e604e035f656cacd1352ea644d680317d4db916}"

# Use separate target directory for sniper builds
SNIPER_TARGET_DIR="$REPO_ROOT/target/sniper-release"
BINARY_OUT="$SNIPER_TARGET_DIR/x86_64-unknown-linux-gnu/release/maybraid"

# Rust installation will go in a writable directory outside the workspace
RUST_HOME="$SNIPER_TARGET_DIR/rust-home"

echo "==> Building maybraid for Linux (Steam Runtime sniper SDK)"
echo "    SDK image: $SNIPER_IMAGE"
echo "    Target: x86_64-unknown-linux-gnu"
echo "    Build dir: $SNIPER_TARGET_DIR"

# Check for podman or docker
if command -v podman >/dev/null 2>&1; then
    CONTAINER_CMD=podman
elif command -v docker >/dev/null 2>&1; then
    CONTAINER_CMD=docker
else
    echo "❌ Neither podman nor docker found. Install one to build with sniper SDK." >&2
    exit 1
fi

echo "    Container: $CONTAINER_CMD"

# Ensure writable directories exist with correct permissions
mkdir -p "$SNIPER_TARGET_DIR"
mkdir -p "$RUST_HOME"

$CONTAINER_CMD run --rm \
    --user "$(id -u):$(id -g)" \
    -v "$REPO_ROOT:/workspace:rw" \
    -v "$RUST_HOME:/rust-home:rw" \
    -w /workspace \
    -e HOME=/rust-home \
    -e CARGO_HOME=/rust-home/.cargo \
    -e RUSTUP_HOME=/rust-home/.rustup \
    -e CARGO_TARGET_DIR=/workspace/target/sniper-release \
    "$SNIPER_IMAGE" \
    bash -c '
        set -euo pipefail
        
        # Install rustup if not present
        if [ ! -f "$CARGO_HOME/bin/rustup" ]; then
            echo "==> Installing Rust toolchain"
            curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | \
                sh -s -- -y --default-toolchain none --no-modify-path
        fi
        
        export PATH="$CARGO_HOME/bin:$PATH"
        
        # Install the pinned toolchain from rust-toolchain.toml
        rustup show
        
        # Build for x86_64 Linux
        echo "==> Building release binary"
        cargo build -p maybraid \
            --release \
            --locked \
            --target x86_64-unknown-linux-gnu
        
        echo "==> Build complete"
    '

if [[ ! -f "$BINARY_OUT" ]]; then
    echo "❌ Build failed: binary not found at $BINARY_OUT" >&2
    exit 1
fi

echo "==> Validating binary"

# Check ELF dependencies
echo "Binary dependencies:"
if command -v ldd >/dev/null; then
    ldd "$BINARY_OUT" | sed 's/^/    /' || true
fi

if command -v readelf >/dev/null; then
    echo
    echo "ELF interpreter:"
    readelf -l "$BINARY_OUT" | grep 'program interpreter' | sed 's/^/    /'
    
    echo
    echo "Required symbol versions:"
    readelf -V "$BINARY_OUT" 2>/dev/null | grep -A 999 "Version needs section" | grep -E "^\s+(0x|File:|Name:)" | head -20 | sed 's/^/    /'
fi

# Check for forbidden paths
if strings "$BINARY_OUT" | grep -q "/nix/store"; then
    echo "❌ Binary contains /nix/store references" >&2
    strings "$BINARY_OUT" | grep "/nix/store" | head -5 | sed 's/^/    /' >&2
    exit 1
fi

echo "✅ Binary validation passed"
echo
echo "Built: $BINARY_OUT"
echo "Ready for Steam and AppImage packaging"
