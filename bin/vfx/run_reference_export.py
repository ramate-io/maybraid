#!/usr/bin/env python3
"""Build a pinned reference and link offline GPU readers without editing its sources.
Cargo may write build outputs; all reader sources/binaries/readbacks live here.
"""
from pathlib import Path
import argparse
import subprocess
import os
import json
from surrealist_reference import reference_checkout, EXPECTED_COMMIT

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference-checkout", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--compare", action="store_true", help="Use the native comparison reader")
    parser.add_argument("--target-dir", type=Path, help="Optional Cargo target-directory override")
    parser.add_argument("--validate-only", action="store_true", help="Check path/commit without a build or window")
    args = parser.parse_args()
    try:
        ref = reference_checkout(args.reference_checkout)
    except ValueError as error:
        parser.error(str(error))
    if args.validate_only:
        print(f"validated reference: {ref}")
        return
    build_command = ["cargo", "build", "--locked", "--profile", "perf", "-p",
                     "vfx-playground", "--message-format=json"]
    if args.target_dir:
        build_command += ["--target-dir", str(args.target_dir.expanduser().resolve())]
    result = subprocess.run(build_command, cwd=ref, capture_output=True, text=True)
    if result.returncode:
        parser.exit(result.returncode, result.stderr)
    artifacts = {}
    directories = set()
    for line in result.stdout.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if event.get("reason") == "compiler-artifact":
            for name in event.get("filenames", []):
                if name.endswith(".rlib"):
                    artifacts[event["target"]["name"]] = name
                    directories.add(str(Path(name).parent))
    stem = "compare_combined_reference" if args.compare else "read_combined_reference"
    binary = ROOT / "target/surrealist-export" / stem
    binary.parent.mkdir(parents=True, exist_ok=True)
    command = ["rustc", "--edition=2024", "-C", "lto=thin", "-C", "opt-level=2",
               str(ROOT / "bin/vfx" / (stem + ".rs")), "-o", str(binary)]
    for directory in sorted(directories):
        command += ["-L", "dependency=" + directory]
    for name in ["bevy", "game_commands", "maybraid_vfx", "vfx_playground", "serde_json"]:
        if name not in artifacts:
            parser.error(f"Cargo did not report the required {name} library artifact")
        command += ["--extern", name + "=" + artifacts[name]]
    env = {**os.environ, "CARGO_MANIFEST_DIR": str(ref / "maybraid/vfx/playground"),
           "CARGO_PKG_NAME": "offline-combined-reader"}
    subprocess.run(command, cwd=ref, env=env, check=True)
    output = args.output.expanduser().resolve()
    output.mkdir(parents=True, exist_ok=True)
    # Developer-neutral source identity/settings, separate from sampled field data.
    metadata = dict(reference_commit=EXPECTED_COMMIT, seed=13,
                    source_dimensions=[192, 96, 160], source_format="rgba16_float",
                    settled_frames=12, mode="comparison" if args.compare else "sequence",
                    times=[0.2, 1.0, 3.8] if args.compare else [i / 10 for i in range(1, 47)])
    (output / "export.json").write_text(json.dumps(metadata, indent=2) + "\n")
    subprocess.run([str(binary), "--output", str(output), "--reference-assets",
                    str(ref / "maybraid/assets")], cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
