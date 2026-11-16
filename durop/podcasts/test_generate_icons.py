"""Exercise remote preview bounds and atomic publication with real FFmpeg output."""

import json
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import generate_icons


class RemotePreviewTests(unittest.TestCase):
    """Synthetic source frames expose byte-budget and portrait-dimension failures."""

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def source(self, width, height):
        """Create a deterministic noisy RGB frame and the corresponding clip metadata."""
        rng = random.Random(42)
        media = self.root / "noise.ppm"
        media.write_bytes(f"P6\n{width} {height}\n255\n".encode() + rng.randbytes(width * height * 3))
        metadata = self.root / "noise.json"
        metadata.write_text(json.dumps({"id": "noise", "asset": media.name}))
        return metadata

    def test_noisy_frame_retries_until_compressed_size_fits(self):
        metadata = self.source(512, 512)
        size, width, height = generate_icons.generate_icon(metadata)
        self.assertLess(width, 320, "Noisy source must exercise the byte-budget retry")
        self.assertEqual(width, height)
        self.assertLessEqual(size, generate_icons.MAX_ICON_BYTES)
        subprocess.run(["ffmpeg", "-v", "error", "-xerror", "-i", str(metadata.with_suffix(".png")), "-f", "null", "-"], check=True)

    def test_portrait_preserves_aspect_ratio_with_bounded_height(self):
        metadata = self.source(200, 1000)
        _, width, height = generate_icons.generate_icon(metadata)
        self.assertLessEqual(height, generate_icons.MAX_DIMENSION)
        self.assertAlmostEqual(width / height, 0.2, delta=0.005)

    def test_failed_generation_keeps_previous_preview(self):
        metadata = self.source(10, 10)
        icon = metadata.with_suffix(".png")
        icon.write_bytes(b"previous preview")
        with patch.object(generate_icons.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "ffmpeg")):
            with self.assertRaises(subprocess.CalledProcessError):
                generate_icons.generate_icon(metadata)
        self.assertEqual(icon.read_bytes(), b"previous preview")
        self.assertFalse(list(self.root.glob(".durop-preview-*")))


if __name__ == "__main__":
    unittest.main()
