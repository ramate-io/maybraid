#!/usr/bin/env python3
"""Package sampled poses; verify/reuse runtime meshes against the pinned reference."""
from pathlib import Path
import argparse
import json
import hashlib
from cartoon_reference import reference_checkout, EXPECTED_COMMIT

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference-checkout", type=Path, required=True)
    parser.add_argument("--input", type=Path, default=ROOT / "target/cartoon-export/landing.mesh_clip.json")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "maybraid/assets/vfx/dust")
    args = parser.parse_args()
    try:
        ref = reference_checkout(args.reference_checkout)
        data = json.loads(args.input.read_text())
        if data["fps"] != 240 or data["duration"] != 1.2 or len(data["tracks"]) != 16:
            raise ValueError("input does not match the sampled Landing clip contract")
        data["surface"] = {"light": [-3.8, 5.5, 3.0], "thresholds": [.04, .62],
                           "colors": [[.34, .19, .085], [.58, .37, .17], [.84, .59, .31]]}
        hashes = {}
        for name in ["Shoulder", "Column", "Swept"]:
            path = Path(f"vfx/dust/bodies/{name}.glb")
            source = (ref / "maybraid/assets" / path).read_bytes()
            existing = (ROOT / "maybraid/assets" / path).read_bytes()
            if source != existing:
                raise ValueError(f"existing runtime mesh differs from reference: {path}")
            hashes[name] = hashlib.sha256(source).hexdigest()
    except (ValueError, KeyError, OSError) as error:
        parser.error(str(error))
    # Stable key order retains the approved package bytes, including original
    # manifests sampled before the reader emitted the surface block itself.
    data = {key: data[key] for key in ("duration", "fps", "schema_version", "tracks", "surface")}
    args.output_dir.mkdir(parents=True, exist_ok=True)
    dest = args.output_dir / "cartoon_landing.mesh_clip.json"
    dest.write_text(json.dumps(data, separators=(",", ":")) + "\n")
    provenance = dict(reference_commit=EXPECTED_COMMIT, seed=13,
                      method="Native Puff::table/state samples at 240 Hz; existing byte-identical GLB meshes; rest-coordinate dissolution and palette",
                      fps=240, frames=289, duration_s=1.2, tracks=16,
                      source_mesh_sha256=hashes, clip_sha256=hashlib.sha256(dest.read_bytes()).hexdigest(),
                      commands=["python3 bin/vfx/export_cartoon_reference.py --reference-checkout \"$REFERENCE\" --output target/cartoon-export",
                                "python3 bin/vfx/package_cartoon.py --reference-checkout \"$REFERENCE\""])
    (args.output_dir / "cartoon_dust.provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    print(f"packaged {dest}: {dest.stat().st_size} bytes")


if __name__ == "__main__":
    main()
