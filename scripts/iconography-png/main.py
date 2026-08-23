"""Render the open icon .blend to a square PNG with a fixed ortho camera.

Invoked headlessly:

    blender --background input.blend --python scripts/iconography-png/main.py -- output.png

Batch wrapper: ``scripts/iconography-png/render.sh``.

Author icons in the XY plane inside a 2×2 square (X,Y ∈ [−1, +1]). This
script frames a 2.2×2.2 square so ±1 sits inside a 10% antialias margin:

    Camera: orthographic, scale 2.2, location (0, 0, 10), rotation (0, 0, 0)
    Output: 512 × 512 PNG with a transparent film

Requires Blender's bundled Python (``bpy``).
"""

import os
import sys

RESOLUTION = 512
ORTHO_SCALE = 2.2
CAMERA_NAME = "IconRenderCamera"
CAMERA_LOCATION = (0.0, 0.0, 10.0)
CAMERA_ROTATION = (0.0, 0.0, 0.0)


def _output_path() -> str:
    argv = sys.argv
    if "--" not in argv:
        print("main.py: expected output path after '--'", file=sys.stderr)
        sys.exit(1)
    return os.path.abspath(argv[argv.index("--") + 1])


def _ensure_ortho_camera(scene) -> None:
    import bpy

    existing = bpy.data.objects.get(CAMERA_NAME)
    if existing is not None:
        bpy.data.objects.remove(existing, do_unlink=True)

    camera = bpy.data.cameras.new(CAMERA_NAME)
    camera.type = "ORTHO"
    camera.ortho_scale = ORTHO_SCALE
    camera.clip_start = 0.1
    camera.clip_end = 100.0

    camera_obj = bpy.data.objects.new(CAMERA_NAME, camera)
    camera_obj.location = CAMERA_LOCATION
    camera_obj.rotation_euler = CAMERA_ROTATION
    scene.collection.objects.link(camera_obj)
    scene.camera = camera_obj


def _configure_render(scene, output_path: str) -> None:
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.render.resolution_x = RESOLUTION
    scene.render.resolution_y = RESOLUTION
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.filepath = output_path
    scene.render.use_file_extension = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.image_settings.color_depth = "8"
    scene.render.image_settings.compression = 15

    shading = scene.display.shading
    shading.light = "FLAT"
    shading.color_type = "MATERIAL"
    if hasattr(scene.display, "render_aa"):
        scene.display.render_aa = "32"


def main() -> None:
    output_path = _output_path()
    output_dir = os.path.dirname(output_path)
    if output_dir:
        os.makedirs(output_dir, exist_ok=True)

    import bpy

    scene = bpy.context.scene
    _ensure_ortho_camera(scene)
    _configure_render(scene, output_path)

    result = bpy.ops.render.render(write_still=True)
    if "FINISHED" not in result:
        print(f"main.py: render failed: {result}", file=sys.stderr)
        sys.exit(1)

    if not os.path.isfile(output_path):
        print(f"main.py: output file was not created: {output_path}", file=sys.stderr)
        sys.exit(1)

    print(f"Rendered {output_path}")


if __name__ == "__main__":
    main()
