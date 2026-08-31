"""Fit a clothing mesh to a bind-pose body and export garment + armature.

The clothing .blend must already be open (garment skinned to the unit Humanoid
armature). This script does not save the clothing file.

    blender --background clothes.blend --python scripts/clothes-fit/main.py -- \\
        --body body.blend --output fitted.glb

Steps: append the body's rest-pose mesh, inflate the garment along normals,
shrinkwrap onto the body with a keep-above-surface offset, apply those
deforms, drop the body, export the garment and armature.

Requires Blender's bundled Python (``bpy``).
"""

from __future__ import annotations

import argparse
import os
import sys

HELPER_MESH_PREFIXES = (
    "cube",
    "cone",
    "icosphere",
    "sphere",
    "plane",
    "cylinder",
    "nurbscircle",
    "nurbspath",
)


def _argv_after_dashdash() -> list[str]:
    argv = sys.argv
    if "--" not in argv:
        print("main.py: expected arguments after '--'", file=sys.stderr)
        sys.exit(1)
    return argv[argv.index("--") + 1 :]


def _parse_args(args: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Inflate and shrinkwrap an open clothing blend onto a body"
    )
    parser.add_argument("--body", required=True, help="Bind-pose body .blend to wrap onto")
    parser.add_argument("--output", required=True, help="Destination .glb path")
    parser.add_argument(
        "--inflate",
        type=float,
        default=0.2,
        help="Meters to push garment vertices along normals before wrap (default: 0.2)",
    )
    parser.add_argument(
        "--offset",
        type=float,
        default=0.02,
        help="Shrinkwrap keep-above-surface offset in meters (default: 0.02)",
    )
    parser.add_argument(
        "--garment",
        action="append",
        default=[],
        help="Optional garment object name (repeatable). Default: non-helper mesh objects",
    )
    return parser.parse_args(args)


def _is_helper_mesh(name: str) -> bool:
    stem = name.split(".", 1)[0].lower()
    return stem in HELPER_MESH_PREFIXES or any(stem.startswith(p) for p in HELPER_MESH_PREFIXES)


def _object_mode() -> None:
    import bpy

    if bpy.context.mode != "OBJECT":
        bpy.ops.object.mode_set(mode="OBJECT")


def _select_only(objects) -> None:
    import bpy

    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.select_set(True)
    if objects:
        bpy.context.view_layer.objects.active = objects[0]


def _append_body_meshes(body_path: str):
    import bpy

    with bpy.data.libraries.load(body_path, link=False) as (data_from, data_to):
        data_to.objects = list(data_from.objects)

    imported = []
    for obj in data_to.objects:
        if obj is None:
            continue
        if obj.type != "MESH":
            bpy.data.objects.remove(obj, do_unlink=True)
            continue
        if obj.name not in bpy.context.scene.collection.objects:
            bpy.context.scene.collection.objects.link(obj)
        imported.append(obj)

    if not imported:
        print(f"main.py: no mesh objects appended from {body_path}", file=sys.stderr)
        sys.exit(1)

    for obj in imported:
        obj.name = f"FitTarget_{obj.name}"
        for mod in list(obj.modifiers):
            obj.modifiers.remove(mod)

    return imported


def _join_targets(targets):
    import bpy

    if len(targets) == 1:
        return targets[0]
    _select_only(targets)
    ctx = bpy.context.copy()
    ctx["object"] = targets[0]
    ctx["active_object"] = targets[0]
    ctx["selected_objects"] = list(targets)
    ctx["selected_editable_objects"] = list(targets)
    with bpy.context.temp_override(**ctx):
        bpy.ops.object.join()
    return bpy.context.view_layer.objects.active


def _garment_objects(requested_names: list[str], exclude):
    import bpy

    exclude_names = {obj.name for obj in exclude}
    meshes = [
        obj
        for obj in bpy.data.objects
        if obj.type == "MESH" and obj.name not in exclude_names
    ]
    if requested_names:
        wanted = set(requested_names)
        found = [obj for obj in meshes if obj.name in wanted]
        missing = wanted.difference(obj.name for obj in found)
        if missing:
            print(f"main.py: garment object(s) not found: {sorted(missing)}", file=sys.stderr)
            sys.exit(1)
        return found
    garments = [obj for obj in meshes if not _is_helper_mesh(obj.name)]
    if not garments:
        print("main.py: no garment mesh objects found", file=sys.stderr)
        sys.exit(1)
    return garments


def _disable_armature_modifiers(obj) -> list:
    disabled = []
    for mod in obj.modifiers:
        if mod.type == "ARMATURE":
            disabled.append((mod, mod.show_viewport, mod.show_render))
            mod.show_viewport = False
            mod.show_render = False
    return disabled


def _restore_armature_modifiers(disabled) -> None:
    for mod, viewport, render in disabled:
        mod.show_viewport = viewport
        mod.show_render = render


def _inflate_along_normals(obj, distance: float) -> None:
    import bmesh

    if distance == 0.0:
        return
    mesh = obj.data
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bm.verts.ensure_lookup_table()
    for vert in bm.verts:
        vert.normal_update()
    for vert in bm.verts:
        vert.co += vert.normal * distance
    bm.to_mesh(mesh)
    bm.free()
    mesh.update()


def _shrinkwrap(garment, target, offset: float) -> None:
    import bpy

    name = "ClothesFitWrap"
    existing = garment.modifiers.get(name)
    if existing is not None:
        garment.modifiers.remove(existing)
    wrap = garment.modifiers.new(name, "SHRINKWRAP")
    wrap.target = target
    wrap.wrap_method = "NEAREST_SURFACEPOINT"
    wrap.wrap_mode = "ABOVE_SURFACE"
    wrap.offset = offset
    # Apply against rest-pose verts; armature stays on the stack but disabled.
    garment.modifiers.move(garment.modifiers.find(name), 0)

    _select_only([garment])
    ctx = bpy.context.copy()
    ctx["object"] = garment
    ctx["active_object"] = garment
    ctx["selected_objects"] = [garment]
    ctx["selected_editable_objects"] = [garment]
    with bpy.context.temp_override(**ctx):
        bpy.ops.object.modifier_apply(modifier=name)


def _export_glb(output_path: str, objects) -> None:
    import bpy
    from addon_utils import check, enable

    default, enabled = check("io_scene_gltf2")
    if not enabled:
        enable("io_scene_gltf2", default_set=True, persistent=True)

    output_dir = os.path.dirname(output_path)
    if output_dir:
        os.makedirs(output_dir, exist_ok=True)

    _select_only(objects)
    result = bpy.ops.export_scene.gltf(
        filepath=output_path,
        export_format="GLB",
        use_selection=True,
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


def main() -> None:
    args = _parse_args(_argv_after_dashdash())
    body_path = os.path.abspath(args.body)
    output_path = os.path.abspath(args.output)
    if not os.path.isfile(body_path):
        print(f"main.py: body blend not found: {body_path}", file=sys.stderr)
        sys.exit(1)

    import bpy

    if not bpy.data.filepath:
        print("main.py: open a clothing .blend as the Blender input file", file=sys.stderr)
        sys.exit(1)

    _object_mode()
    targets = _append_body_meshes(body_path)
    target = _join_targets(targets)
    garments = _garment_objects(args.garment, exclude=[target])
    armatures = [obj for obj in bpy.data.objects if obj.type == "ARMATURE"]
    if not armatures:
        print("main.py: no armature in the clothing scene", file=sys.stderr)
        sys.exit(1)

    for garment in garments:
        disabled = _disable_armature_modifiers(garment)
        _inflate_along_normals(garment, args.inflate)
        _shrinkwrap(garment, target, args.offset)
        _restore_armature_modifiers(disabled)

    bpy.data.objects.remove(target, do_unlink=True)

    _export_glb(output_path, garments + armatures)


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception:
        import traceback

        traceback.print_exc()
        sys.exit(1)
