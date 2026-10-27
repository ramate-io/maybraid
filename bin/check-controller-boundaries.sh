#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

command -v grep >/dev/null || {
	printf 'grep is required for the controller boundary check\n' >&2
	exit 1
}

facades=(
	maybraid/gameplay/session/src/lib.rs
	maybraid/gameplay/lib/src/lib.rs
	maybraid/world/lib/src/lib.rs
	maybraid/world/lib/src/control.rs
)

for facade in "${facades[@]}"; do
	if grep -En '^[[:space:]]*pub use[[:space:]]+maybraid_(controllers_core|session)::' "$facade"; then
		printf 'composition facade must not reexport routing or session-owned APIs: %s\n' "$facade" >&2
		exit 1
	fi
done
