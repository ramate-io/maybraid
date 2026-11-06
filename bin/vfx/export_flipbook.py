#!/usr/bin/env python3
"""Repackage the pinned eight-view dust delivery; requires KTX-Software 4.4.2."""

import argparse
import base64
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import tempfile


HERE = Path(__file__).resolve().parent
LOCK = HERE / "rolling_wave.sources.json"
DEFAULT_OUTPUT = HERE.parent.parent / "maybraid/assets/vfx/dust"
ENCODING = [
    "--t2", "--encode", "uastc", "--uastc_quality", "4", "--zcmp", "19",
    "--threads", "1", "--assign_oetf", "srgb", "--assign_primaries", "bt709",
    "--target_type", "RGBA", "--layers", "8",
]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def validate_manifest(manifest):
    """Reject incompatible delivery layouts instead of silently reinterpreting art."""
    b = manifest["B_plus"]
    expected = {
        "view_count": 8, "atlas_size_px": [952, 952], "tiles_xy": [7, 7],
        "frame_tile_interior_px": [128, 128], "padding_each_edge_px": 4,
        "tile_pitch_px": [136, 136], "frame_count": 49,
        "format": "RGBA8 PNG; RGB sRGB; A linear coverage",
        "projection": "orthographic", "capture_width_height_m": [7.8, 7.8],
        "quad_centre_m": [0, 0.55, 0], "pivot_m": [0, 0, 0],
        "frame_order": "row-major top-left; i -> column i%7,row floor(i/7)",
    }
    for key, value in expected.items():
        require(b[key] == value, f"incompatible manifest {key}: {b[key]!r}")
    require("straight/unassociated sRGB" in b["rgb"], "expected straight sRGB RGB")
    require(manifest["space"]["pivot_m"] == b["pivot_m"], "inconsistent pivot")
    timestamps = b["timestamps_s"]
    require(len(timestamps) == 49, "expected 49 timestamps")
    require(all(math.isclose(t, i / 15, abs_tol=1e-12) for i, t in enumerate(timestamps)),
            "expected uniform timestamps at 15 Hz starting at zero")
    require(manifest["shared_source"]["timestamps_s"] == timestamps, "inconsistent timestamps")
    views = b["views"]
    require(len(views) == 8, "expected eight views")
    for index, view in enumerate(views):
        require(view["index"] == index and view["file"] == f"atlases/view_{index:02}.png",
                f"view {index}: expected ascending authored view order")
        require(view["azimuth_degrees"] == index * 45 and view["elevation_degrees"] == 20,
                f"view {index}: incompatible camera angles")
        require(view["camera_target_m"] == b["quad_centre_m"], "inconsistent camera target")
    return b


def validate_png(data, name):
    require(len(data) >= 33 and data[:8] == b"\x89PNG\r\n\x1a\n" and data[12:16] == b"IHDR",
            f"{name}: invalid PNG header")
    header = struct.unpack_from(">IIBBBBB", data, 16)
    require(header == (952, 952, 8, 6, 0, 0, 0), f"{name}: expected 952x952 noninterlaced RGBA8 PNG")


def validate_inputs(source, lock):
    raw = (source / "manifest.json").read_bytes()
    manifest = json.loads(raw)
    b = validate_manifest(manifest)
    require(sha256(raw) == lock["files"]["manifest.json"], "source manifest SHA-256 mismatch")
    paths = []
    for view in b["views"]:
        name = view["file"]
        path = source / name
        data = path.read_bytes()
        validate_png(data, name)
        require(sha256(data) == lock["files"][name] == view["sha256"], f"{name}: SHA-256 mismatch")
        require(len(data) == view["disk_bytes"], f"{name}: byte count mismatch")
        paths.append(path.resolve())
    return manifest, paths


def fetch_sources(source, lock):
    """Fetch exactly the nine pinned files through the configured GitHub CLI."""
    for name, digest in lock["files"].items():
        path = source / name
        if path.exists() and sha256(path.read_bytes()) == digest:
            continue
        endpoint = f"repos/{lock['repository']}/contents/{lock['directory']}/{name}?ref={lock['commit']}"
        reply = json.loads(subprocess.check_output(["gh", "api", endpoint]))
        require(reply.get("encoding") == "base64", f"{name}: GitHub did not return base64 contents")
        data = base64.b64decode(reply["content"])
        require(sha256(data) == digest, f"{name}: downloaded SHA-256 mismatch")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def validate_ktx(data):
    """Check shape, UASTC/sRGB/straight DFD and full indexed payload boundaries."""
    require(len(data) >= 104 and data[:12] == b"\xabKTX 20\xbb\r\n\x1a\n", "invalid KTX2 header")
    header = struct.unpack_from("<13I2Q", data, 12)
    require(header[:9] == (0, 1, 952, 952, 0, 8, 1, 1, 2),
            "expected Zstd UASTC 952x952 eight-layer 2D array with one mip level")
    dfd_offset, dfd_length = header[9:11]
    require(dfd_length >= 44 and dfd_offset + dfd_length <= len(data), "invalid DFD extent")
    # KHR_DF_MODEL_UASTC=166, BT709=1, SRGB=2, straight alpha flags=0.
    require(data[dfd_offset + 12:dfd_offset + 16] == bytes([166, 1, 2, 0]),
            "expected UASTC, BT709, sRGB, straight alpha DFD")
    require(data[dfd_offset + 16:dfd_offset + 20] == bytes([3, 3, 0, 0]), "expected 4x4 UASTC blocks")
    require(data[dfd_offset + 20] == 16 and data[dfd_offset + 31] == 3,
            "expected 16-byte UASTC RGBA blocks, including alpha")
    offset, compressed, uncompressed = struct.unpack_from("<3Q", data, 80)
    expected = 238 * 238 * 16 * 8
    require(uncompressed == expected, "unexpected UASTC decoded payload size")
    require(offset >= dfd_offset + dfd_length and compressed > 0 and offset + compressed == len(data),
            "truncated or trailing KTX2 payload")
    return {"disk_bytes": len(data), "zstd_payload_bytes": compressed,
            "uastc_payload_bytes": expected, "gpu_bc7_bytes": expected,
            "gpu_rgba8_fallback_bytes": 952 * 952 * 4 * 8}


def runtime_manifest(source):
    b = source["B_plus"]
    return {
        "schema_version": 1, "image": "rolling_wave.ktx2", "atlas_size": b["atlas_size_px"],
        "grid": b["tiles_xy"], "frame_size": b["frame_tile_interior_px"],
        "gutter_px": b["padding_each_edge_px"], "frame_count": b["frame_count"], "fps": 15,
        "bounds": {"min": source["space"]["bounds_min_m"], "max": source["space"]["bounds_max_m"]},
        "pivot": b["pivot_m"], "quad_center": b["quad_centre_m"],
        "capture_size": b["capture_width_height_m"], "alpha_mode": "straight",
        "end_behavior": {"mode": "hold"},
        "views": [{"layer": v["index"], "azimuth_degrees": v["azimuth_degrees"],
                   "elevation_degrees": v["elevation_degrees"]} for v in b["views"]],
    }


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def export(source, output, toktx, ktx, lock):
    manifest, paths = validate_inputs(source, lock)
    version = subprocess.check_output([toktx, "--version"], stderr=subprocess.STDOUT, text=True).strip()
    require(version == "toktx v4.4.2", f"expected toktx v4.4.2, got {version}")
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="rolling-wave-") as temp:
        artifact = Path(temp) / "rolling_wave.ktx2"
        subprocess.run([toktx, *ENCODING, str(artifact), *map(str, paths)], check=True)
        subprocess.run([ktx, "validate", str(artifact)], check=True)
        data = artifact.read_bytes()
        storage = validate_ktx(data)
        (output / artifact.name).write_bytes(data)
    write_json(output / "rolling_wave.flipbook.json", runtime_manifest(manifest))
    provenance = {
        "source": lock, "encoder": version, "encoder_arguments": ENCODING,
        "source_png_disk_bytes": sum(p.stat().st_size for p in paths),
        "artifact_sha256": sha256(data), "storage": storage,
        "format": "KTX2 UASTC 4x4, Zstandard, sRGB RGB / linear straight alpha; not native BC7",
        "mip_levels": 1, "source_pixels": "No resize, crop, flip, color conversion or premultiplication; UASTC is lossy",
    }
    write_json(output / "rolling_wave.provenance.json", provenance)
    print(json.dumps(storage, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="directory containing manifest.json and atlases/")
    parser.add_argument("--fetch", action="store_true", help="download only pinned inputs with gh api")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--toktx", default="toktx")
    parser.add_argument("--ktx", default="ktx")
    args = parser.parse_args()
    try:
        lock = json.loads(LOCK.read_text())
        if args.fetch:
            fetch_sources(args.source, lock)
        export(args.source, args.output, args.toktx, args.ktx, lock)
    except (KeyError, TypeError, ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"export failed: {error}\n")


if __name__ == "__main__":
    main()
