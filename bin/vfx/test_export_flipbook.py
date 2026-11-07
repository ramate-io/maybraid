"""Run with python3 -m unittest discover -s bin/vfx -p 'test_export_flipbook.py'."""

import copy
import json
from pathlib import Path
import struct
import tempfile
import unittest

import export_flipbook as exporter


def source_manifest():
    times = [i / 15 for i in range(49)]
    return {
        "shared_source": {"timestamps_s": times},
        "space": {"pivot_m": [0, 0, 0], "bounds_min_m": [-3.2, 0, -3.2],
                  "bounds_max_m": [3.2, 3.2, 3.2]},
        "B_plus": {
            "view_count": 8, "atlas_size_px": [952, 952], "tiles_xy": [7, 7],
            "frame_tile_interior_px": [128, 128], "padding_each_edge_px": 4,
            "tile_pitch_px": [136, 136], "frame_count": 49,
            "format": "RGBA8 PNG; RGB sRGB; A linear coverage",
            "rgb": "constant straight/unassociated sRGB", "projection": "orthographic",
            "capture_width_height_m": [7.8, 7.8], "quad_centre_m": [0, 0.55, 0],
            "pivot_m": [0, 0, 0], "timestamps_s": times,
            "frame_order": "row-major top-left; i -> column i%7,row floor(i/7)",
            "views": [{"index": i, "file": f"atlases/view_{i:02}.png",
                       "azimuth_degrees": i * 45, "elevation_degrees": 20,
                       "camera_target_m": [0, 0.55, 0]} for i in range(8)],
        },
    }


class FlipbookExportTests(unittest.TestCase):
    def test_preserves_order_and_metadata(self):
        source = source_manifest()
        exporter.validate_manifest(source)
        actual = exporter.runtime_manifest(source)
        self.assertEqual([v["layer"] for v in actual["views"]], list(range(8)))
        self.assertEqual([v["azimuth_degrees"] for v in actual["views"]], list(range(0, 360, 45)))
        self.assertEqual(actual["pivot"], [0, 0, 0])
        self.assertEqual(actual["quad_center"], [0, 0.55, 0])
        self.assertEqual(actual["capture_size"], [7.8, 7.8])
        self.assertEqual(actual["fps"], 15)
        self.assertEqual(actual["alpha_mode"], "straight")

    def test_rejects_reordered_missing_duplicate_and_unsafe_views(self):
        base = source_manifest()
        reversed_views = list(reversed(base["B_plus"]["views"]))
        duplicate = copy.deepcopy(base["B_plus"]["views"])
        duplicate[1] = duplicate[0]
        unsafe = copy.deepcopy(base["B_plus"]["views"])
        unsafe[0]["file"] = "../view_00.png"
        for views in (reversed_views, duplicate, unsafe, []):
            with self.subTest(views=views):
                source = copy.deepcopy(base)
                source["B_plus"]["views"] = views
                with self.assertRaises(ValueError):
                    exporter.validate_manifest(source)

    def test_rejects_incompatible_manifest(self):
        changes = {"atlas_size_px": [1024, 1024], "frame_count": 48, "tiles_xy": [8, 8],
                   "padding_each_edge_px": 0, "format": "RGBA8 linear", "rgb": "premultiplied",
                   "timestamps_s": [i / 30 for i in range(49)], "pivot_m": [0, 1, 0],
                   "frame_order": "column-major"}
        for key, value in changes.items():
            with self.subTest(key=key):
                source = source_manifest()
                source["B_plus"][key] = value
                with self.assertRaises(ValueError):
                    exporter.validate_manifest(source)

    def test_png_dimensions_channels_and_depth(self):
        def header(width=952, depth=8, channels=6):
            return b"\x89PNG\r\n\x1a\n" + struct.pack(">I4sIIBBBBBI", 13, b"IHDR", width, 952, depth, channels, 0, 0, 0, 0)
        exporter.validate_png(header(), "fixture")
        for data in (b"bad", header(width=1024), header(depth=16), header(channels=2)):
            with self.assertRaises(ValueError):
                exporter.validate_png(data, "fixture")

    def test_rejects_unpinned_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            (path / "manifest.json").write_text(json.dumps(source_manifest()))
            with self.assertRaisesRegex(ValueError, "manifest SHA-256"):
                exporter.validate_inputs(path, {"files": {"manifest.json": "0" * 64}})

    def test_rejects_modified_source_png(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            (path / "atlases").mkdir()
            manifest = source_manifest()
            data = b"\x89PNG\r\n\x1a\n" + struct.pack(">I4sIIBBBBBI", 13, b"IHDR", 952, 952, 8, 6, 0, 0, 0, 0)
            lock = {"files": {}}
            for view in manifest["B_plus"]["views"]:
                view["sha256"] = exporter.sha256(data)
                view["disk_bytes"] = len(data)
                lock["files"][view["file"]] = view["sha256"]
                (path / view["file"]).write_bytes(data)
            raw = json.dumps(manifest).encode()
            (path / "manifest.json").write_bytes(raw)
            lock["files"]["manifest.json"] = exporter.sha256(raw)
            _, paths = exporter.validate_inputs(path, lock)
            self.assertEqual([p.name for p in paths], [f"view_{i:02}.png" for i in range(8)])
            (path / "atlases/view_03.png").write_bytes(data + b"changed")
            with self.assertRaisesRegex(ValueError, "view_03.png: SHA-256"):
                exporter.validate_inputs(path, lock)

    def test_checked_in_artifact_and_provenance(self):
        data = (exporter.DEFAULT_OUTPUT / "rolling_wave.ktx2").read_bytes()
        stats = exporter.validate_ktx(data)
        provenance = json.loads((exporter.DEFAULT_OUTPUT / "rolling_wave.provenance.json").read_text())
        self.assertEqual(provenance["artifact_sha256"], exporter.sha256(data))
        self.assertEqual(provenance["storage"], stats)
        self.assertEqual(stats["gpu_bc7_bytes"], 7250432)
        self.assertEqual(stats["gpu_rgba8_fallback_bytes"], 29001728)
        for offset, value in ((32, 1), (40, 2), (44, 0)):
            corrupt = bytearray(data)
            struct.pack_into("<I", corrupt, offset, value)
            with self.assertRaises(ValueError):
                exporter.validate_ktx(corrupt)
        with self.assertRaises(ValueError):
            exporter.validate_ktx(data[:-1])


if __name__ == "__main__":
    unittest.main()
