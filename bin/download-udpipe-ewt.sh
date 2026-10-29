#!/usr/bin/env bash
# Fetch the English EWT UDPipe 1 model into assets/language/udpipe.
# Runtime parsing stays offline after this file is present.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${MAYBRAID_UDPIPE:-$ROOT/maybraid/assets/language/udpipe/english-ewt.udpipe}"
# LINDAT's bitstream URL often serves an HTML license page. The UD 2.5 EWT
# model is mirrored from the same release in this GitHub archive.
URL="${MAYBRAID_UDPIPE_URL:-https://raw.githubusercontent.com/jwijffels/udpipe.models.ud.2.5/master/inst/udpipe-ud-2.5-191206/english-ewt-ud-2.5-191206.udpipe}"

mkdir -p "$(dirname "$DEST")"
if [[ -f "$DEST" ]]; then
  echo "Already present: $DEST" >&2
  exit 0
fi

echo "Downloading $URL" >&2
echo "         to $DEST" >&2
curl -L --fail --retry 3 --retry-delay 2 \
  -A "maybraid-language-udpipe/0.1" \
  -o "$DEST.partial" "$URL"

if head -c 15 "$DEST.partial" | grep -q '<!DOCTYPE\|<html'; then
  rm -f "$DEST.partial"
  echo "Download did not return a UDPipe model (got HTML)." >&2
  exit 1
fi
if [[ "$(wc -c < "$DEST.partial")" -lt 1000000 ]]; then
  rm -f "$DEST.partial"
  echo "Download is too small to be the English EWT model." >&2
  exit 1
fi

mv "$DEST.partial" "$DEST"
echo "Wrote $DEST" >&2
