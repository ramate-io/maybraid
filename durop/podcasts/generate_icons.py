#!/usr/bin/env python3
"""Regenerate title-free thumbnails in the style of Durop's base reactions pack."""

import json
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parent


def main():
    """Render source frames and waveforms; Durop supplies the card titles."""
    for path in sorted((ROOT / "pack").rglob("*.json")):
        if path.name == "durop.json":
            continue
        metadata = json.loads(path.read_text())
        media = path.parent / metadata["asset"]
        command = ["ffmpeg", "-v", "error", "-y"]
        # Show the title card before the swallow rather than its opening background.
        if metadata["id"] == "title-swallow":
            command += ["-ss", "3.9"]
        command += ["-i", str(media)]
        if path.parent.name == "audio":
            command += [
                "-filter_complex",
                "showwavespic=s=320x180:colors=0x45d6c8:draw=full",
            ]
        else:
            command += ["-vf", "scale=320:-1"]
        subprocess.run(
            command + ["-frames:v", "1", "-update", "1", str(path.with_suffix(".png"))],
            check=True,
        )


if __name__ == "__main__":
    main()
