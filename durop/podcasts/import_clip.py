#!/usr/bin/env python3
"""Publish a selected local media file and PNG icon into the podcast pack."""

import argparse
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent
EXTENSIONS = {".wav", ".mp3", ".ogg", ".flac", ".m4a", ".aac", ".mp4", ".mov", ".webm", ".gif"}


def probe(path):
    """Reject unreadable inputs before creating catalog metadata."""
    if not path.is_file() or not 0 < path.stat().st_size <= 256 * 1024 * 1024:
        raise ValueError(f"Expected a nonempty file of at most 256 MiB: {path}")
    result = subprocess.run(
        ["ffprobe", "-v", "error", "-show_streams", "-of", "json", str(path)],
        check=True, capture_output=True, text=True,
    )
    streams = json.loads(result.stdout).get("streams", [])
    if not streams:
        raise ValueError(f"No readable media streams: {path}")
    return streams


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("clip_id", help="ID from SOURCES.md")
    parser.add_argument("media", type=Path, help="Downloaded source media; copied unchanged")
    parser.add_argument("icon", type=Path, help="Author-supplied PNG card icon")
    args = parser.parse_args()
    templates = [
        path for path in (ROOT / "pending").rglob("*.json.in")
        if path.name == f"{args.clip_id}.json.in"
    ]
    if len(templates) != 1:
        parser.error("clip_id must identify exactly one selection in SOURCES.md")
    template = templates[0]
    metadata = json.loads(template.read_text())
    extension = args.media.suffix.lower()
    if extension not in EXTENSIONS:
        parser.error("Unsupported media extension for Durop v1")
    streams = probe(args.media)
    category = template.parent.name
    expected_stream = "audio" if category == "audio" else "video"
    if not any(stream.get("codec_type") == expected_stream for stream in streams):
        parser.error(f"This selection requires a {expected_stream} stream")
    probe(args.icon)
    with args.icon.open("rb") as source:
        header = source.read(33)
    if (len(header) != 33 or header[:16] != b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR"
            or 0 in struct.unpack(">II", header[16:24])):
        parser.error("Icon must be a PNG with nonzero dimensions")
    destination = ROOT / "pack" / category
    destination.mkdir(exist_ok=True)
    metadata["asset"] = f"{metadata['id']}{extension}"
    outputs = [destination / metadata["asset"], destination / f"{metadata['id']}.png",
               destination / f"{metadata['id']}.json"]
    if any(path.exists() for path in outputs):
        parser.error("Clip already exists; review and remove its files before reimporting")
    # Stage first; publish metadata last so it never references incomplete copies.
    with tempfile.TemporaryDirectory(prefix="durop-import-") as temporary:
        staged = Path(temporary)
        shutil.copyfile(args.media, staged / outputs[0].name)
        shutil.copyfile(args.icon, staged / outputs[1].name)
        (staged / outputs[2].name).write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n")
        created = []
        try:
            for output in outputs:
                with output.open("xb") as target, (staged / output.name).open("rb") as source:
                    created.append(output)
                    shutil.copyfileobj(source, target)
        except BaseException:
            for output in created:
                output.unlink()
            raise
    print(f"Imported maybraid-podcasts/{metadata['id']}; update SOURCES.md with provenance.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Import failed: {error}") from error
