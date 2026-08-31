"""Rewrite linked .blend library paths in the open Blender file.

Invoked headlessly:

    blender --background input.blend --python scripts/blend-relink/main.py -- \\
        --map-json '{"humanoid_rig.blend": "/abs/path/to/humanoid_rig.blend"}'

Libraries whose basename appears in the map are pointed at the mapped file
(relative to the open .blend) and reloaded. The file is saved in place.

Requires Blender's bundled Python (``bpy``).
"""

from __future__ import annotations

import json
import os
import sys


def _argv_after_dashdash() -> list[str]:
    argv = sys.argv
    if "--" not in argv:
        print("main.py: expected arguments after '--'", file=sys.stderr)
        sys.exit(1)
    return argv[argv.index("--") + 1 :]


def _parse_map(args: list[str]) -> dict[str, str]:
    import argparse

    parser = argparse.ArgumentParser(description="Relink Blender libraries by basename")
    parser.add_argument(
        "--map-json",
        required=True,
        help="JSON object mapping library basename -> absolute .blend path",
    )
    parsed = parser.parse_args(args)
    raw = parsed.map_json
    if os.path.isfile(raw):
        with open(raw, encoding="utf-8") as handle:
            mapping = json.load(handle)
    else:
        mapping = json.loads(raw)
    if not isinstance(mapping, dict):
        print("main.py: --map-json must be a JSON object", file=sys.stderr)
        sys.exit(1)
    out: dict[str, str] = {}
    for key, value in mapping.items():
        if not isinstance(key, str) or not isinstance(value, str):
            print("main.py: map keys and values must be strings", file=sys.stderr)
            sys.exit(1)
        out[os.path.basename(key)] = os.path.abspath(value)
    return out


def _library_basename(filepath: str) -> str:
    return os.path.basename(filepath.replace("\\", "/"))


def main() -> None:
    mapping = _parse_map(_argv_after_dashdash())

    import bpy

    blend_path = bpy.data.filepath
    if not blend_path:
        print("main.py: no open .blend file", file=sys.stderr)
        sys.exit(1)

    changed = 0
    for lib in list(bpy.data.libraries):
        basename = _library_basename(lib.filepath)
        target = mapping.get(basename)
        if target is None:
            continue
        if not os.path.isfile(target):
            print(f"main.py: mapped library missing: {target}", file=sys.stderr)
            sys.exit(1)
        relative = bpy.path.relpath(target)
        if lib.filepath == relative:
            continue
        print(f"Relink {basename}: {lib.filepath} -> {relative}")
        lib.filepath = relative
        try:
            lib.reload()
        except Exception as exc:  # noqa: BLE001 — Blender library reload is best-effort
            print(f"warning: reload failed for {basename}: {exc}", file=sys.stderr)
        changed += 1

    bpy.ops.file.make_paths_relative()
    bpy.ops.wm.save_mainfile(filepath=blend_path)
    print(f"Relinked {changed} librar{'y' if changed == 1 else 'ies'} in {blend_path}")


if __name__ == "__main__":
    main()
