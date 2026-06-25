"""Export the open Blender scene to a binary glTF (.glb) file.

Invoked headlessly by the blend-export pre-commit hook:

    blender --background input.blend --python scripts/glb/main.py -- output.glb

Requires Blender's bundled Python (``bpy``); this module is not runnable with
system Python.
"""

import os
import sys


def main() -> None:
    argv = sys.argv
    if "--" not in argv:
        print("main.py: expected output path after '--'", file=sys.stderr)
        sys.exit(1)

    output_path = os.path.abspath(argv[argv.index("--") + 1])
    output_dir = os.path.dirname(output_path)
    if output_dir:
        os.makedirs(output_dir, exist_ok=True)

    import bpy
    from addon_utils import check, enable

    default, enabled = check("io_scene_gltf2")
    if not enabled:
        enable("io_scene_gltf2", default_set=True, persistent=True)

    result = bpy.ops.export_scene.gltf(
        filepath=output_path,
        export_format="GLB",
        export_apply=False,
        export_animations=True,
        export_skins=True,
        export_materials="EXPORT",
    )
    if "FINISHED" not in result:
        print(f"main.py: glTF export failed: {result}", file=sys.stderr)
        sys.exit(1)

    if not os.path.isfile(output_path):
        print(f"main.py: output file was not created: {output_path}", file=sys.stderr)
        sys.exit(1)

    print(f"Exported {output_path}")


if __name__ == "__main__":
    main()
