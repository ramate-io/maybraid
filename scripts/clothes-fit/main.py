"""Fit an open (or thin) clothing mesh to a bind-pose body and export.

The clothing .blend must already be open (garment skinned to the unit Humanoid
armature). This script does not save the clothing file.

    blender --background clothes.blend --python scripts/clothes-fit/main.py -- \\
        --body body.blend --output fitted.glb

Pipeline: shrinkwrap Outside onto an inflated body (keep-out), push verts a
little farther along body normals (slack), Cloth-simulate against the render
body, apply Cloth, keep existing vertex groups.

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

PIN_GROUP = "FitPins"
PIN_NAME_TOKENS = ("shoulder", "neck")
PIN_NAME_EXCLUDE = ("thickness", "humerus", "forearm")


def _argv_after_dashdash() -> list[str]:
    argv = sys.argv
    if "--" not in argv:
        print("main.py: expected arguments after '--'", file=sys.stderr)
        sys.exit(1)
    return argv[argv.index("--") + 1 :]


def _parse_args(args: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Keep-out wrap a clothing shell, add ease, cloth-drape, export"
    )
    parser.add_argument("--body", required=True, help="Bind-pose body .blend to wrap onto")
    parser.add_argument("--output", required=True, help="Destination .glb path")
    parser.add_argument(
        "--inflate",
        type=float,
        default=0.04,
        help="Meters to inflate the Outside-wrap target body (default: 0.04)",
    )
    parser.add_argument(
        "--ease",
        type=float,
        default=0.02,
        help="Meters to push garment verts along body normals after wrap (default: 0.02)",
    )
    parser.add_argument(
        "--collision-gap",
        type=float,
        default=0.015,
        help="Cloth collision thickness against the body, meters (default: 0.015)",
    )
    parser.add_argument(
        "--cloth-frames",
        type=int,
        default=24,
        help="Frames to simulate cloth drape (default: 24)",
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


def _copy_mesh(obj, name: str):
    import bpy

    dup = obj.copy()
    dup.data = obj.data.copy()
    dup.name = name
    bpy.context.scene.collection.objects.link(dup)
    for mod in list(dup.modifiers):
        dup.modifiers.remove(mod)
    return dup


def _delete(obj) -> None:
    import bpy

    mesh = obj.data if obj.type == "MESH" else None
    bpy.data.objects.remove(obj, do_unlink=True)
    if mesh is not None and mesh.users == 0:
        bpy.data.meshes.remove(mesh)


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


def _body_bvh_in_garment_space(body, garment):
    import bmesh
    from mathutils.bvhtree import BVHTree

    g_inv = garment.matrix_world.inverted()
    bm = bmesh.new()
    bm.from_mesh(body.data)
    bm.transform(g_inv @ body.matrix_world)
    bm.normal_update()
    return bm, BVHTree.FromBMesh(bm, epsilon=1e-6)


def _inflate_along_body_normals(garment, body, distance: float) -> None:
    import bmesh

    if distance == 0.0:
        return
    body_bm, bvh = _body_bvh_in_garment_space(body, garment)
    g_bm = bmesh.new()
    g_bm.from_mesh(garment.data)
    g_bm.verts.ensure_lookup_table()
    moved = 0
    for vert in g_bm.verts:
        loc, hit_n, _i, _d = bvh.find_nearest(vert.co, 1.5)
        if loc is None or hit_n is None or hit_n.length_squared < 1e-12:
            continue
        vert.co += hit_n.normalized() * distance
        moved += 1
    body_bm.free()
    g_bm.to_mesh(garment.data)
    g_bm.free()
    garment.data.update()
    print(f"ease {distance}m along body normals ({moved} verts)")


def _coords(obj) -> list[float]:
    n = len(obj.data.vertices)
    buf = [0.0] * (n * 3)
    obj.data.vertices.foreach_get("co", buf)
    return buf


def _moved_vert_count(before: list[float], after: list[float], eps: float = 1e-5) -> int:
    moved = 0
    for i in range(0, min(len(before), len(after)), 3):
        dx = after[i] - before[i]
        dy = after[i + 1] - before[i + 1]
        dz = after[i + 2] - before[i + 2]
        if dx * dx + dy * dy + dz * dz > eps * eps:
            moved += 1
    return moved


def _shrinkwrap_outside(garment, target) -> None:
    import bpy

    before = _coords(garment)
    name = "ClothesFitWrap"
    existing = garment.modifiers.get(name)
    if existing is not None:
        garment.modifiers.remove(existing)
    wrap = garment.modifiers.new(name, "SHRINKWRAP")
    wrap.target = target
    wrap.wrap_method = "NEAREST_SURFACEPOINT"
    wrap.wrap_mode = "OUTSIDE"
    wrap.offset = 0.0
    garment.modifiers.move(garment.modifiers.find(name), 0)
    _apply_modifier(garment, name)
    after = _coords(garment)
    print(
        f"shrinkwrap OUTSIDE verts={len(garment.data.vertices)} moved={_moved_vert_count(before, after)}"
    )


def _make_pin_group(obj) -> str:
    existing = obj.vertex_groups.get(PIN_GROUP)
    if existing is not None:
        obj.vertex_groups.remove(existing)
    pin_groups = []
    for group in obj.vertex_groups:
        name = group.name.lower()
        if any(tok in name for tok in PIN_NAME_EXCLUDE):
            continue
        if any(tok in name for tok in PIN_NAME_TOKENS):
            pin_groups.append(group)
    indices = set()
    group_ids = {g.index for g in pin_groups}
    for vert in obj.data.vertices:
        for membership in vert.groups:
            if membership.group in group_ids and membership.weight >= 0.25:
                indices.add(vert.index)
                break
    if not indices:
        zs = [v.co.z for v in obj.data.vertices]
        cutoff = min(zs) + 0.82 * (max(zs) - min(zs))
        indices = {v.index for v in obj.data.vertices if v.co.z >= cutoff}
    group = obj.vertex_groups.new(name=PIN_GROUP)
    if indices:
        group.add(list(indices), 1.0, "REPLACE")
    print(f"pin group {PIN_GROUP}: {len(indices)} verts from {[g.name for g in pin_groups]}")
    return PIN_GROUP


def _enable_collision(obj, thickness: float) -> None:
    import bpy

    existing = obj.modifiers.get("FitCollision")
    if existing is not None:
        obj.modifiers.remove(existing)
    obj.modifiers.new("FitCollision", "COLLISION")
    obj.collision.use = True
    obj.collision.thickness_outer = thickness
    obj.collision.thickness_inner = thickness


def _freeze_evaluated_coords(obj) -> None:
    import bpy

    depsgraph = bpy.context.evaluated_depsgraph_get()
    eval_obj = obj.evaluated_get(depsgraph)
    eval_mesh = eval_obj.to_mesh()
    n = len(eval_mesh.vertices)
    if n != len(obj.data.vertices):
        eval_obj.to_mesh_clear()
        print(
            f"main.py: evaluated vert count {n} != {len(obj.data.vertices)}",
            file=sys.stderr,
        )
        sys.exit(1)
    coords = [0.0] * (n * 3)
    eval_mesh.vertices.foreach_get("co", coords)
    obj.data.vertices.foreach_set("co", coords)
    obj.data.update()
    eval_obj.to_mesh_clear()


def _drape_cloth(garment, collider, *, frames: int, gap: float, pin_group: str) -> None:
    import bpy

    if frames <= 0:
        return
    scene = bpy.context.scene
    scene.frame_start = 1
    scene.frame_end = frames
    scene.gravity = (0.0, 0.0, -9.81)
    scene.frame_set(1)

    collider.hide_viewport = False
    collider.hide_render = False
    garment.hide_viewport = False
    garment.hide_render = False
    _enable_collision(collider, gap)

    existing = garment.modifiers.get("FitCloth")
    if existing is not None:
        garment.modifiers.remove(existing)
    cloth = garment.modifiers.new("FitCloth", "CLOTH")
    settings = cloth.settings
    settings.quality = 7
    settings.mass = 0.2
    settings.tension_stiffness = 15.0
    settings.compression_stiffness = 15.0
    settings.shear_stiffness = 5.0
    settings.bending_stiffness = 0.5
    settings.vertex_group_mass = pin_group
    collide = cloth.collision_settings
    collide.use_collision = True
    collide.distance_min = gap
    collide.collision_quality = 5
    collide.use_self_collision = False
    cache = cloth.point_cache
    cache.frame_start = 1
    cache.frame_end = frames

    before = _coords(garment)
    print(f"cloth drape {frames} frames gap={gap} pin={pin_group}")
    baked = False
    try:
        with bpy.context.temp_override(
            scene=scene, active_object=garment, point_cache=cache
        ):
            result = bpy.ops.ptcache.bake(bake=True)
        baked = "FINISHED" in result
        print(f"ptcache.bake {result}")
    except Exception as exc:
        print(f"ptcache.bake skipped ({exc})")
    if not baked:
        for frame in range(1, frames + 1):
            scene.frame_set(frame)
            bpy.context.view_layer.update()
    scene.frame_set(frames)
    bpy.context.view_layer.update()
    _freeze_evaluated_coords(garment)
    garment.modifiers.remove(cloth)
    after = _coords(garment)
    print(
        f"cloth frozen verts={len(garment.data.vertices)} moved={_moved_vert_count(before, after)}"
    )


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

    _default, enabled = check("io_scene_gltf2")
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
    wrap_body = _copy_mesh(body, "FitWrapBody")
    _recalc_normals(wrap_body)
    _inflate_along_normals(wrap_body, args.inflate)
    _recalc_normals(wrap_body)

    garments = _garment_objects(args.garment, exclude=[body, wrap_body])
    armatures = [obj for obj in bpy.data.objects if obj.type == "ARMATURE"]
    if not armatures:
        print("main.py: no armature in the clothing scene", file=sys.stderr)
        sys.exit(1)

    for garment in garments:
        disabled = _disable_armature_modifiers(garment)
        print(f"fit {garment.name} verts={len(garment.data.vertices)}")
        _shrinkwrap_outside(garment, wrap_body)
        _inflate_along_body_normals(garment, body, args.ease)
        pin = _make_pin_group(garment)
        _drape_cloth(
            garment,
            body,
            frames=args.cloth_frames,
            gap=args.collision_gap,
            pin_group=pin,
        )
        pin_vg = garment.vertex_groups.get(pin)
        if pin_vg is not None:
            garment.vertex_groups.remove(pin_vg)
        _restore_armature_modifiers(disabled)
        _assert_reasonable_bounds(garment)

    _delete(wrap_body)
    _delete(body)
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
