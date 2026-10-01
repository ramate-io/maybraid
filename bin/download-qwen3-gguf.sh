#!/usr/bin/env bash
# Fetch the development Qwen3 1.7B Q4 GGUF into assets/models.
# Runtime inference stays offline after this file is present.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${MAYBRAID_QWEN_GGUF:-$ROOT/maybraid/assets/models/qwen3-1.7b-q4.gguf}"
URL="${MAYBRAID_QWEN_GGUF_URL:-https://huggingface.co/unsloth/Qwen3-1.7B-GGUF/resolve/main/Qwen3-1.7B-Q4_K_M.gguf}"

mkdir -p "$(dirname "$DEST")"
if [[ -f "$DEST" ]]; then
  echo "Already present: $DEST" >&2
  exit 0
fi

echo "Downloading $URL" >&2
echo "         to $DEST" >&2
curl -L --fail --retry 3 --retry-delay 2 -o "$DEST.partial" "$URL"
mv "$DEST.partial" "$DEST"
echo "Wrote $DEST" >&2
