"""Offline package invariants; KTX-enabled tests also decode the real exports."""

import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

from export_volumes import check_header, decode_volume, encode_volume, volume_manifest
from fetch_reference import DEFAULT_LOCK, ROOT, checked_path, fetch, selected_entries, verify

ASSETS = ROOT / "maybraid/assets/vfx/dust"
KTX = os.environ.get("KTX") or shutil.which("ktx")


class VolumePackageTests(unittest.TestCase):
    def setUp(self):
        self.lock = json.loads(DEFAULT_LOCK.read_text())
        self.provenance = json.loads((ASSETS / "volumes.provenance.json").read_text())

    def test_actual_package_shapes_formats_hashes_and_transforms(self):
        entries = {e["path"]: e for e in self.lock["reusable"]}
        volumes = self.provenance["volumes"]
        self.assertEqual(len(volumes), 7)
        self.assertEqual(sum(v["gpu_bytes"] for v in volumes), 11730944)
        for volume in volumes:
            path = checked_path(ASSETS, volume["file"])
            manifest = json.loads(path.with_suffix(".volume.json").read_text())
            raw = path.read_bytes()
            dimensions = manifest["dimensions"]
            self.assertEqual(dimensions, [64, 64, 64] if path.parent.name == "clouds"
                             else [128, 112, 112])
            check_header(raw, dimensions)
            self.assertEqual(hashlib.sha256(raw).hexdigest(), volume["sha256"])
            self.assertEqual(volume["source_sha256"], entries[volume["source"]]["sha256"])
            self.assertEqual(volume["decoded_sha256"], volume["source_sha256"])
            self.assertEqual(manifest["format"], "r16_float")
            self.assertEqual(checked_path(path.parent, manifest["image"]), path)
            matrix = manifest["index_to_local"]
            self.assertEqual(len(matrix), 16)
            self.assertEqual(matrix[3::4], [0.0, 0.0, 0.0, 1.0])
            for axis in range(3):
                spacing = matrix[axis * 4 + axis]
                self.assertTrue(math.isfinite(spacing) and spacing > 0)
                self.assertAlmostEqual(matrix[12 + axis] - spacing * 0.5,
                                       manifest["bounds"]["min"][axis])
                self.assertAlmostEqual(matrix[12 + axis] + spacing * (dimensions[axis] - 0.5),
                                       manifest["bounds"]["max"][axis])

    @unittest.skipUnless(KTX, "set KTX to KTX-Software ktx executable for full decode checks")
    def test_actual_exports_decode_to_pinned_original_bytes(self):
        for entry in self.provenance["volumes"]:
            path = ASSETS / entry["file"]
            dimensions = json.loads(path.with_suffix(".volume.json").read_text())["dimensions"]
            decoded = decode_volume(KTX, path, dimensions)
            self.assertEqual(len(decoded), entry["gpu_bytes"])
            self.assertEqual(hashlib.sha256(decoded).hexdigest(), entry["source_sha256"])

    def test_raw_length_validation_happens_before_tool_or_output(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "invalid.ktx2"
            for raw in (b"", b"\0" * 15, b"\0" * 17):
                with self.assertRaisesRegex(ValueError, "byte length"):
                    encode_volume("nonexistent-tool", raw, [2, 2, 2], path)
            self.assertFalse(path.exists())

    def test_truncated_and_wrong_format_ktx_rejected(self):
        path = ASSETS / self.provenance["volumes"][0]["file"]
        raw = path.read_bytes()
        with self.assertRaises(ValueError):
            check_header(raw[:-1], [64, 64, 64])
        with self.assertRaises(ValueError):
            check_header(raw[:80], [64, 64, 64])
        corrupted = bytearray(raw)
        corrupted[12] = 9  # Vulkan R8_UNORM, not the supplied f16 scalar.
        with self.assertRaises(ValueError):
            check_header(corrupted, [64, 64, 64])

    def test_reusable_selection_and_dedup_exclude_recordings(self):
        reusable = selected_entries(self.lock, "reusable")
        self.assertFalse(any("/Reference/" in e["path"] for e in reusable))
        self.assertEqual(len(self.lock["deduplicated_sources"]), 9)
        self.assertEqual(self.lock["optional_groups"]["recordings"]["bytes"], 474914261)
        self.assertLess(sum(e["bytes"] for e in self.provenance["volumes"]), 2 * 1024 * 1024)
        for entry in self.provenance["retained_files"]:
            data = checked_path(ASSETS, entry["file"]).read_bytes()
            self.assertEqual(hashlib.sha256(data).hexdigest(), entry["sha256"])

    def test_hash_and_size_failures(self):
        entry = {"path": "Body/grid.f16", "bytes": 2,
                 "sha256": hashlib.sha256(b"\0\0").hexdigest()}
        self.assertEqual(verify(b"\0\0", entry), b"\0\0")
        with self.assertRaisesRegex(ValueError, "length"):
            verify(b"\0", entry)
        with self.assertRaisesRegex(ValueError, "checksum"):
            verify(b"\0\1", entry)

    def test_optional_selection_cannot_include_recording_in_authoring_group(self):
        files = [{"path": "Combined/Assets/Body/Shoulder.glb", "bytes": 12},
                 {"path": "Combined/Assets/Reference/Pyro/Frame_000_density.f32.gz", "bytes": 99}]
        lock = {"optional_groups": {"authoring": {
            "prefixes": ["Combined/"], "exclude_prefixes": ["Combined/Assets/Reference/Pyro/"],
            "file_count": 1, "bytes": 12}}}
        self.assertEqual(selected_entries(lock, "authoring", {"files": files}), files[:1])
        with self.assertRaisesRegex(ValueError, "totals"):
            selected_entries(lock, "authoring", {"files": files[1:]})

    def test_downloads_cannot_target_runtime_assets_even_before_network(self):
        with patch("fetch_reference.download") as download:
            with self.assertRaisesRegex(ValueError, "outside the runtime assets"):
                fetch(self.lock, "recordings", ASSETS / "reference")
            download.assert_not_called()

    def test_matrix_preserves_source_translation_and_axis_order(self):
        manifest = volume_manifest("test.ktx2", [2, 3, 4], [-1, -2, -3], [3, 7, 13],
                                   [[2, 0, 0, 0], [0, 3, 0, -0.5], [0, 0, 4, -1]])
        self.assertEqual(manifest["index_to_local"],
                         [2, 0, 0, 0, 0, 3, 0, 0, 0, 0, 4, 0, 0, -0.5, -1, 1])

    def test_rejects_paths_outside_selected_root(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "safe"
            root.mkdir()
            for path in ("../secret", "/etc/passwd", "a/../../secret", "a\\..\\secret", ""):
                with self.assertRaises(ValueError):
                    checked_path(root, path)
            (root / "escape").symlink_to(Path(directory))
            with self.assertRaises(ValueError):
                checked_path(root, "escape/secret")
            self.assertEqual(checked_path(root, "Body/test.f16"), root / "Body/test.f16")


if __name__ == "__main__":
    unittest.main()
