#!/usr/bin/env python3
"""Generate or check PNG thumbnails compatible with existing Durop controllers."""

import argparse
import json
from pathlib import Path
import struct
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent
# Existing Durop controllers enforce these limits independently of the host.
MAX_ICON_BYTES = 160 * 1024
MAX_DIMENSION = 512
# Retry from the original media, never repeatedly resample a previous attempt.
PREVIEW_WIDTHS = (320, 256, 192, 128, 64)


def validate_icon(path):
    """Check the same compressed-size and PNG-header bounds as Durop's transport."""
    size = path.stat().st_size
    with path.open("rb") as source:
        header = source.read(33)
    if (len(header) != 33
            or header[:16] != b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR"):
        raise ValueError(f"Expected a PNG preview: {path}")
    width, height = struct.unpack(">II", header[16:24])
    if size > MAX_ICON_BYTES or not (0 < width <= MAX_DIMENSION and 0 < height <= MAX_DIMENSION):
        raise ValueError(f"{path}: {size} bytes, {width}x{height}; limit is {MAX_ICON_BYTES} bytes and {MAX_DIMENSION}px per side")
    return size, width, height


def generate_icon(path):
    """Publish only a validated preview; retain the previous icon if generation fails."""
    metadata = json.loads(path.read_text())
    media = path.parent / metadata["asset"]
    destination = path.with_suffix(".png")
    with tempfile.TemporaryDirectory(prefix=".durop-preview-", dir=path.parent) as temporary:
        candidate = Path(temporary) / "preview.png"
        for width in PREVIEW_WIDTHS:
            command = ["ffmpeg", "-v", "error", "-y"]
            # Show the title card before the swallow rather than its opening background.
            if metadata["id"] == "title-swallow":
                command += ["-ss", "3.9"]
            command += ["-i", str(media)]
            if path.parent.name == "audio":
                command += ["-filter_complex", f"showwavespic=s={width}x{width * 180 // 320}:colors=0x45d6c8:draw=full"]
            else:
                command += ["-vf", f"scale=w='min({width},iw)':h='min({MAX_DIMENSION},ih)':force_original_aspect_ratio=decrease"]
            subprocess.run(command + [
                "-frames:v", "1", "-pix_fmt", "rgba", "-compression_level", "9",
                "-pred", "mixed", "-update", "1", str(candidate),
            ], check=True)
            try:
                result = validate_icon(candidate)
            except ValueError:
                continue
            candidate.replace(destination)
            return result
    raise ValueError(f"Cannot generate a remote-compatible preview for {path}")


def main():
    """Regenerate all or selected clip previews, or check the current catalog without writing."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("clip_ids", nargs="*", help="Optional clip IDs, for example banana")
    parser.add_argument("--check", action="store_true", help="Validate existing PNG bounds and decoding without modifying files")
    args = parser.parse_args()
    clips = {json.loads(path.read_text())["id"]: path
             for path in sorted((ROOT / "pack").rglob("*.json")) if path.name != "durop.json"}
    unknown = set(args.clip_ids) - clips.keys()
    if unknown:
        parser.error(f"Unknown clip IDs: {', '.join(sorted(unknown))}")
    for clip_id in args.clip_ids or clips:
        path = clips[clip_id]
        if args.check:
            size, width, height = validate_icon(path.with_suffix(".png"))
            subprocess.run(["ffmpeg", "-v", "error", "-xerror", "-i", str(path.with_suffix(".png")), "-f", "null", "-"], check=True)
        else:
            size, width, height = generate_icon(path)
        print(f"{clip_id}: {size} bytes, {width}x{height}")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Thumbnail processing failed: {error}") from error
