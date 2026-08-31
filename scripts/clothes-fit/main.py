"""Fit a clothing mesh to a bind-pose body and export garment + armature.

The clothing .blend must already be open (garment skinned to the unit Humanoid
armature). This script does not save the clothing file.

    blender --background clothes.blend --python scripts/clothes-fit/main.py -- \\
        --body body.blend --output fitted.glb

The wrap target is a temporary body *cage*: Catmull-Clark subsurf, then inflate
along normals. The garment shrinkwraps onto that cage (nearest surface, keep-above
offset). The render body is not modified.

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
        description="Wrap an open clothing blend onto an inflated body cage"
    )
    parser.add_argument("--body", required=True, help="Bind-pose body .blend to wrap onto")
    parser.add_argument("--output", required=True, help="Destination .glb path")
    parser.add_argument(
        "--inflate",
        type=float,
        default=0.04,
        help="Meters to push cage vertices along normals before wrap (default: 0.04)",
    )
    parser.add_argument(
        "--offset",
        type=float,
        default=0.04,
        help="Keep-above-surface offset from the cage in meters (default: 0.04)",
    )
    parser.add_argument(
        "--cage-subsurf",
        type=int,
        default=1,
        help="Catmull-Clark levels on the wrap cage (default: 1; 0 skips)",
    )
    parser.add_argument(
        "--thickness",
        type=float,
        default=0.0,
        help="Solidify thickness after wrap, meters (default: 0 skips)",
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


def _apply_modifier(obj, name: str) -> None:
    import bpy

    _select_only([obj])
    ctx = bpy.context.copy()
    ctx["object"] = obj
    ctx["active_object"] = obj
    ctx["selected_objects"] = [obj]
    ctx["selected_editable_objects"] = [obj]
    with bpy.context.temp_override(**ctx):
        bpy.ops.object.modifier_apply(modifier=name)


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


def _recalc_normals(obj) -> None:
    import bmesh

    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.normal_update()
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()


def _inflate_along_normals(obj, distance: float) -> None:
    import bmesh

    if distance == 0.0:
        return
    mesh = obj.data
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bm.normal_update()
    bm.verts.ensure_lookup_table()
    for vert in bm.verts:
        vert.co += vert.normal * distance
    bm.to_mesh(mesh)
    bm.free()
    mesh.update()


def _apply_subsurf(obj, levels: int) -> None:
    import bpy

    if levels <= 0:
        return
    name = "FitCageSubsurf"
    existing = obj.modifiers.get(name)
    if existing is not None:
        obj.modifiers.remove(existing)
    mod = obj.modifiers.new(name, "SUBSURF")
    mod.subdivision_type = "CATMULL_CLARK"
    mod.levels = levels
    mod.render_levels = levels
    _apply_modifier(obj, name)


def _make_cage(body, *, subsurf_levels: int, inflate: float):
    import bpy

    cage = body.copy()
    cage.data = body.data.copy()
    cage.name = "FitCage"
    bpy.context.scene.collection.objects.link(cage)
    _apply_subsurf(cage, subsurf_levels)
    _recalc_normals(cage)
    _inflate_along_normals(cage, inflate)
    _recalc_normals(cage)
    return cage


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
    _apply_modifier(garment, name)


def _solidify(obj, thickness: float) -> None:
    import bpy

    if thickness <= 0.0:
        return
    name = "ClothesFitSolidify"
    existing = obj.modifiers.get(name)
    if existing is not None:
        obj.modifiers.remove(existing)
    mod = obj.modifiers.new(name, "SOLIDIFY")
    mod.thickness = thickness
    # Original (wrapped) surface stays inner; new verts go outward.
    mod.offset = 1.0
    mod.use_even_offset = True
    obj.modifiers.move(obj.modifiers.find(name), 0)
    _apply_modifier(obj, name)


def _assert_reasonable_bounds(obj, max_extent: float = 5.0) -> None:
    import bpy

    bpy.context.view_layer.update()
    extent = max(obj.dimensions)
    if extent > max_extent:
        print(
            f"main.py: garment {obj.name} extent {tuple(obj.dimensions)} exceeds {max_extent}m",
            file=sys.stderr,
        )
        sys.exit(1)


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
    body = _join_targets(targets)
    cage = _make_cage(
        body,
        subsurf_levels=args.cage_subsurf,
        inflate=args.inflate,
    )
    garments = _garment_objects(args.garment, exclude=[body, cage])
    armatures = [obj for obj in bpy.data.objects if obj.type == "ARMATURE"]
    if not armatures:
        print("main.py: no armature in the clothing scene", file=sys.stderr)
        sys.exit(1)

    for garment in garments:
        disabled = _disable_armature_modifiers(garment)
        _shrinkwrap(garment, cage, args.offset)
        _solidify(garment, args.thickness)
        _restore_armature_modifiers(disabled)
        _assert_reasonable_bounds(garment)

    bpy.data.objects.remove(cage, do_unlink=True)
    bpy.data.objects.remove(body, do_unlink=True)

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
