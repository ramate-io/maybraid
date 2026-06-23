"""Dump armature bone hierarchy from the open Blender scene.

Invoked headlessly by the armature-dump pre-commit hook:

    blender --background input.blend --python scripts/armature-dump/main.py -- output.armature_dump

Requires Blender's bundled Python (``bpy``).
"""

import os
import sys


def find_armature_object():
    import bpy

    if "Armature" in bpy.data.objects:
        obj = bpy.data.objects["Armature"]
        if obj.type == "ARMATURE":
            return obj

    for obj in bpy.data.objects:
        if obj.type == "ARMATURE":
            return obj

    return None


def dump_bone(bone, lines: list[str], indent: int = 0) -> None:
    lines.append(f"{'  ' * indent}{bone.name}")
    for child in bone.children:
        dump_bone(child, lines, indent + 1)


def main() -> None:
    argv = sys.argv
    if "--" not in argv:
        print("main.py: expected output path after '--'", file=sys.stderr)
        sys.exit(1)

    output_path = os.path.abspath(argv[argv.index("--") + 1])
    output_dir = os.path.dirname(output_path)
    if output_dir:
        os.makedirs(output_dir, exist_ok=True)

    armature_obj = find_armature_object()
    if armature_obj is None:
        print("main.py: no armature found in scene", file=sys.stderr)
        sys.exit(2)

    lines: list[str] = [f"# armature object: {armature_obj.name}", ""]
    for bone in armature_obj.data.bones:
        if bone.parent is None:
            dump_bone(bone, lines)

    if len(lines) <= 2:
        print("main.py: armature has no root bones", file=sys.stderr)
        sys.exit(1)

    with open(output_path, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines))
        handle.write("\n")

    print(f"Dumped armature hierarchy to {output_path}")


if __name__ == "__main__":
    main()
