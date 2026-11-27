#!/usr/bin/env python3
"""Build the pinned reference, link an offline reader and export poses/captures.
All source/tool outputs are written here; Cargo alone writes its build directory.
Uses Cargo artifact paths, including CARGO_TARGET_DIR / configured target-dir.
"""
from pathlib import Path
import argparse
import subprocess
import os
import json
from cartoon_reference import reference_checkout

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference-checkout", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--target-dir", type=Path, help="Optional Cargo build directory override")
    parser.add_argument("--validate-only", action="store_true", help="Check path/commit without building")
    parser.add_argument("--samples-only", action="store_true", help="Export poses without opening a window")
    args = parser.parse_args()
    try:
        ref = reference_checkout(args.reference_checkout)
    except ValueError as error:
        parser.error(str(error))
    if args.validate_only:
        print(f"validated reference: {ref}")
        return
    command = ["cargo", "build", "--locked", "--profile", "perf", "-p",
               "vfx-playground", "--message-format=json"]
    if args.target_dir:
        command += ["--target-dir", str(args.target_dir.expanduser().resolve())]
    build = subprocess.run(command, cwd=ref, check=True, capture_output=True, text=True)
    artifacts = {}
    search_dirs = set()
    for line in build.stdout.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if event.get("reason") == "compiler-artifact":
            for file in event.get("filenames", []):
                if file.endswith(".rlib"):
                    artifacts[event["target"]["name"]] = file
                    search_dirs.add(str(Path(file).parent))
    source = ROOT / "bin/vfx/read_cartoon_reference.rs"
    binary = ROOT / "target/cartoon-export/read_cartoon_reference"
    binary.parent.mkdir(parents=True, exist_ok=True)
    command = ["rustc", "--edition=2024", "-C", "lto=thin", "-C", "opt-level=2",
               str(source), "-o", str(binary)]
    for directory in sorted(search_dirs):
        command += ["-L", "dependency=" + directory]
    for name in ["bevy", "game_commands", "maybraid_vfx", "vfx_playground", "serde_json"]:
        if name not in artifacts:
            parser.error(f"Cargo did not report the required {name} library artifact")
        command += ["--extern", name + "=" + artifacts[name]]
    env = {**os.environ, "CARGO_MANIFEST_DIR": str(ref / "maybraid/vfx/playground"),
           "CARGO_PKG_NAME": "offline-cartoon-reader"}
    subprocess.run(command, env=env, cwd=ref, check=True)
    command = [str(binary), "--output", str(args.output.expanduser().resolve()),
               "--reference-assets", str(ref / "maybraid/assets")]
    if args.samples_only:
        command += ["--samples-only"]
    subprocess.run(command, cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
