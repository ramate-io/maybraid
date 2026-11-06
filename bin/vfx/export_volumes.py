#!/usr/bin/env python3
"""Losslessly package the pinned reusable dust grids with KTX-Software 4.4.2.

No Houdini, reference recordings, network, resampling or numeric conversion.
The official KTX writer creates linear R16_SFLOAT 3D textures; the official
reader then extracts every depth slice and verifies exact source bytes.
"""

import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile

from fetch_reference import DEFAULT_LOCK, ROOT, checked_path, verify

KTX_VERSION = "ktx version: v4.4.2"
SOURCE_PREFIX = "Combined/Assets/"


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def check_header(data, dimensions):
    if len(data) < 104 or data[:12] != b"\xabKTX 20\xbb\r\n\x1a\n":
        raise ValueError("invalid or truncated KTX2 header")
    header = struct.unpack_from("<9I", data, 12)
    if header != (76, 2, *dimensions, 0, 1, 1, 2):
        raise ValueError("expected one-level Zstd R16_SFLOAT 3D KTX2")
    offset, length, decoded_length = struct.unpack_from("<3Q", data, 80)
    if offset < 104 or length == 0 or offset + length > len(data):
        raise ValueError("invalid or truncated KTX2 level")
    if decoded_length != math.prod(dimensions) * 2:
        raise ValueError("wrong KTX2 decoded volume size")
    dfd_offset, dfd_length = struct.unpack_from("<2I", data, 48)
    if dfd_length != 44 or dfd_offset + dfd_length > len(data) or data[dfd_offset + 14] != 1:
        raise ValueError("expected a linear scalar data format descriptor")


def decode_volume(ktx, path, dimensions):
    check_header(path.read_bytes(), dimensions)
    subprocess.run([ktx, "validate", str(path)], check=True, capture_output=True)
    with tempfile.TemporaryDirectory(prefix="dust-decode-") as temporary:
        decoded = Path(temporary)
        subprocess.run([ktx, "extract", "--raw", "--depth", "all", str(path), str(decoded)],
                       check=True, capture_output=True)
        return b"".join((decoded / f"output_depth{z}.raw").read_bytes()
                        for z in range(dimensions[2]))


def encode_volume(ktx, raw, dimensions, output):
    if len(dimensions) != 3 or any(type(d) is not int or d < 2 for d in dimensions):
        raise ValueError("expected three positive 3D volume dimensions")
    if len(raw) != math.prod(dimensions) * 2:
        raise ValueError("raw grid byte length does not match dimensions")
    width, height, depth = dimensions
    stride = width * height * 2
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="dust-encode-") as temporary:
        slices = []
        for z in range(depth):
            path = Path(temporary) / f"{z:04d}.raw"
            path.write_bytes(raw[z * stride:(z + 1) * stride])
            slices.append(str(path))
        candidate = Path(temporary) / "volume.ktx2"
        subprocess.run([ktx, "create", "--raw", "--format", "R16_SFLOAT",
                        "--width", str(width), "--height", str(height), "--depth", str(depth),
                        "--assign-tf", "linear", "--assign-primaries", "none",
                        "--assign-texcoord-origin", "top-left-front", "--zstd", "9",
                        *slices, str(candidate)], check=True, capture_output=True)
        if decode_volume(ktx, candidate, dimensions) != raw:
            raise ValueError("KTX2 extraction differs from original f16 bytes")
        shutil.copyfile(candidate, output)


def volume_manifest(image, dimensions, bounds_min, bounds_max, transform):
    # Source matrices are row-major 3x4; Bevy Mat4 is column-major 4x4.
    matrix = [transform[row][column] if row < 3 else float(column == 3)
              for column in range(4) for row in range(4)]
    return {"schema_version": 1, "image": image, "dimensions": dimensions,
            "bounds": {"min": bounds_min, "max": bounds_max}, "format": "r16_float",
            "density_scale": 1.0, "index_to_local": matrix}


def export(source, output, lock, ktx):
    version = subprocess.run([ktx, "--version"], check=True, capture_output=True,
                             text=True).stdout.strip()
    if version != KTX_VERSION:
        raise ValueError(f"expected {KTX_VERSION}, found {version}")
    # All inputs are checked before writing outputs, including each tiny manifest.
    entries = {e["path"].removeprefix(SOURCE_PREFIX): e for e in lock["reusable"]
               if e["path"].startswith(SOURCE_PREFIX)}
    for relative, entry in entries.items():
        verify(checked_path(source, relative).read_bytes(), entry)
    clouds = json.loads((source / "DynamicDust/SmallClouds/CloudLibrary.json").read_text())
    bodies = json.loads((source / "Body/BodyAssets.json").read_text())
    provenance = {"schema_version": 1, "repository": lock["repository"],
                  "commit": lock["commit"], "source_root": lock["source_root"],
                  "tool": version, "encoding": "R16_SFLOAT, linear, Zstd level 9, one mip",
                  "voxel_order": "little endian float16, X fastest, then Y, then Z",
                  "verification": "ktx validate; ktx extract --raw --depth all; exact byte equality",
                  "volumes": [], "retained_files": []}

    def emit(asset, category, relative, dimensions, bounds_min, bounds_max, transform):
        name = asset["name"].lower()
        raw = checked_path(source, relative).read_bytes()
        entry = entries[relative]
        target = output / category / f"{name}.ktx2"
        encode_volume(ktx, raw, dimensions, target)
        write_json(target.with_suffix(".volume.json"), volume_manifest(
            target.name, dimensions, bounds_min, bounds_max, transform))
        provenance["volumes"].append({"file": str(target.relative_to(output)),
                                      "source": entry["path"], "source_sha256": entry["sha256"],
                                      "decoded_sha256": sha256(raw), "gpu_bytes": len(raw),
                                      "bytes": target.stat().st_size,
                                      "sha256": sha256(target.read_bytes())})
        return target.name

    packaged_clouds = copy.deepcopy(clouds)
    for asset in packaged_clouds["assets"]:
        transform = asset["index_to_world_m"]
        dimensions = asset["dimensions_xyz"]
        bounds_min = [transform[i][3] - transform[i][i] * 0.5 for i in range(3)]
        bounds_max = [bounds_min[i] + transform[i][i] * dimensions[i] for i in range(3)]
        asset["source_file"] = asset["file"]
        asset["file"] = emit(asset, "clouds", "DynamicDust/SmallClouds/" + asset["file"],
                             dimensions, bounds_min, bounds_max, transform)
        asset["source_sha256"] = asset.pop("sha256")
        asset["source_bytes"] = asset.pop("bytes")
        asset["container"] = "KTX2 R16_SFLOAT, linear, Zstd; decode before indexing"
    write_json(output / "clouds/cloud_library.json", packaged_clouds)

    packaged_bodies = copy.deepcopy(bodies)
    for asset in packaged_bodies["assets"]:
        grid = asset["grid"]
        spacing, minimum = grid["spacing_xyz_m"], grid["bounds_min_m"]
        transform = [[spacing[row] if col == row else 0.0 for col in range(3)] +
                     [minimum[row] + spacing[row] * 0.5] for row in range(3)]
        asset["source_density"] = asset["density"]
        asset["density"] = emit(asset, "bodies", "Body/" + asset["density"],
                                grid["dimensions_xyz"], minimum, grid["bounds_max_m"], transform)
        asset["source_density_sha256"] = asset.pop("density_sha256")
        asset["encoding"] = "KTX2 R16_SFLOAT, linear, Zstd; decoded float16 LE, X then Y then Z"
    write_json(output / "bodies/body_assets.json", packaged_bodies)

    retained = [("Body/" + a["mesh"], "bodies/" + a["mesh"]) for a in bodies["assets"]]
    retained += [("Body/LocalDeformation.f32", "bodies/LocalDeformation.f32"),
                 ("MotionRules.json", "bodies/motion_rules.json"),
                 ("DynamicDust/DynamicRules.json", "clouds/dynamic_rules.json")]
    if "Provenance.json" in entries:
        retained.append(("Provenance.json", "volumes.source.json"))
    for original, target in retained:
        destination = output / target
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(checked_path(source, original), destination)
        provenance["retained_files"].append({"file": target, "source": entries[original]["path"],
                                              "bytes": destination.stat().st_size,
                                              "sha256": sha256(destination.read_bytes())})
    provenance["deduplicated_sources"] = lock.get("deduplicated_sources", [])
    write_json(output / "volumes.provenance.json", provenance)
    return provenance


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path, required=True,
                        help="verified source Combined/Assets directory")
    parser.add_argument("--output", type=Path, default=ROOT / "maybraid/assets/vfx/dust")
    parser.add_argument("--lock", type=Path, default=DEFAULT_LOCK)
    parser.add_argument("--ktx", default="ktx", help="KTX-Software 4.4.2 ktx executable")
    args = parser.parse_args()
    result = export(args.source_dir, args.output, json.loads(args.lock.read_text()), args.ktx)
    print(f"Verified {len(result['volumes'])} volumes: "
          f"{sum(e['bytes'] for e in result['volumes'])} transport bytes, "
          f"{sum(e['gpu_bytes'] for e in result['volumes'])} R16Float GPU bytes")


if __name__ == "__main__":
    main()
