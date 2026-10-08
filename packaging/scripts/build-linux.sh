#!/usr/bin/env bash
# One sniper-SDK image (Dockerfile) and one compile invocation.
set -euo pipefail

ROOT="${MAYBRAID_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
CHANNEL="$("$ROOT/packaging/version.sh" --rust)"
IMAGE="maybraid-sniper:${CHANNEL}"
OUT="$ROOT/target/sniper-release/x86_64-unknown-linux-gnu/release/maybraid"

if ! command -v docker >/dev/null; then
	echo "build-linux: docker is required" >&2
	exit 1
fi

echo "==> builder $IMAGE (Rust $CHANNEL)"
docker build \
	--build-arg "RUST_CHANNEL=$CHANNEL" \
	-t "$IMAGE" \
	-f "$ROOT/packaging/linux/Dockerfile" \
	"$ROOT/packaging/linux"

mkdir -p "$ROOT/target/sniper-release"
echo "==> cargo build -p maybraid --release --locked --target x86_64-unknown-linux-gnu"
docker run --rm \
	--user 0:0 \
	-v "$ROOT:/src:rw" \
	-w /src \
	-e CARGO_HOME=/opt/rust/cargo \
	-e RUSTUP_HOME=/opt/rust/rustup \
	-e CARGO_TARGET_DIR=/src/target/sniper-release \
	-e HOST_UID="$(id -u)" \
	-e HOST_GID="$(id -g)" \
	"$IMAGE" \
	bash -lc 'cargo build -p maybraid --release --locked --target x86_64-unknown-linux-gnu
		chown -R "$HOST_UID:$HOST_GID" /src/target/sniper-release || true'

[[ -f "$OUT" ]] || { echo "build-linux: missing $OUT" >&2; exit 1; }
chmod +x "$OUT"
"$ROOT/packaging/validate.sh" elf "$OUT"
echo "Built $OUT"
