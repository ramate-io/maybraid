"""Fit a canonical garment to a bind-pose body and export.

The clothing .blend must already be skinned to the unit Humanoid armature. This
script does not save the clothing file.

    blender --background clothes.blend --python scripts/clothes-fit/main.py -- \\
        --body body.blend --output fitted.glb

Pipeline: shrinkwrap the garment once (Target Normal Project, Outside) onto
the (optional FitTo-clipped) body, then bind it to the Humanoid armature.
Existing modifiers are stripped, not applied, so a viewport Shrinkwrap is not
baked before the fitter wraps again. Existing complete weights are preserved;
otherwise weights transfer from the fitted body's nearest surface.

An empty named ``FitOffset_0.04`` (also ``FitOffset_4cm``) or a ``fit_offset``
custom property sets the wrap distance. Body-file empties win over the clothes
file; ``--offset`` wins over both.

``FitTo_Torso`` / ``UpperBody`` / ``LowerBody`` / ``LegsAndTorso`` / ``FullBody``
(or ``fit_to``) clips wrap to an armature-derived AABB so a tank does not snap
onto the arms. ``LegsAndTorso`` is the full body minus the arms (togas and
similar). Weight transfer still uses the full mesh. ``--fit-to`` wins over the
blends.

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

DEFAULT_OFFSET = 0.04
DEFAULT_FIT_TO = "FullBody"
FIT_TO_REGIONS = ("FullBody", "Torso", "UpperBody", "LowerBody", "LegsAndTorso")
INSIDE_EPS = 1e-5
HOST_BODY_PREFIXES = ("humanoidfullbody", "fittarget_", "fitregion")

# Bind-pose Humanoid fractions: "tiny bit" of the adjacent limb, flesh beyond bones.
TORSO_ARM_FRAC = 0.20
TORSO_LEG_FRAC = 0.25
LOWER_HIP_FRAC = 0.60
HAND_PAD_FRAC = 0.15
LEG_FLESH_PAD = 0.12


def _object_stem(name: str) -> str:
    import re

    return re.sub(r"\.\d{3}$", "", name)


def _argv_after_dashdash() -> list[str]:
    argv = sys.argv
    if "--" not in argv:
        print("main.py: expected arguments after '--'", file=sys.stderr)
        sys.exit(1)
    return argv[argv.index("--") + 1 :]


def _parse_args(args: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Fit a garment with one Outside target-normal shrinkwrap, then rebind"
    )
    parser.add_argument("--body", required=True, help="Bind-pose body .blend to wrap onto")
    parser.add_argument("--output", required=True, help="Destination .glb path")
    parser.add_argument(
        "--offset",
        type=float,
        default=None,
        help="OUTSIDE target-normal wrap distance, meters (default: 0.04, or FitOffset_* empty in the blend)",
    )
    parser.add_argument(
        "--garment",
        action="append",
        default=[],
        help="Optional garment object name (repeatable). Default: non-helper mesh objects",
    )
    parser.add_argument(
        "--fit-to",
        choices=FIT_TO_REGIONS,
        default=None,
        help="Body region for wrap (default: FullBody, or FitTo_* empty in the blend)",
    )
    return parser.parse_args(args)


def _is_helper_mesh(name: str) -> bool:
    stem = _object_stem(name).lower()
    return stem in HELPER_MESH_PREFIXES or any(stem.startswith(p) for p in HELPER_MESH_PREFIXES)


def _is_host_body_mesh(name: str) -> bool:
    stem = _object_stem(name).lower()
    return any(stem.startswith(p) for p in HOST_BODY_PREFIXES)


def _parse_length(token: str) -> float:
    text = token.strip().lower().replace(",", ".")
    if text.endswith("cm"):
        return float(text[:-2]) * 0.01
    if text.endswith("mm"):
        return float(text[:-2]) * 0.001
    if text.endswith("m") and len(text) > 1 and not text[-2].isalpha():
        return float(text[:-1])
    return float(text)


def _named_fit_offset(name: str) -> float | None:
    stem = _object_stem(name)
    lower = stem.lower()
    for prefix in ("FitOffset", "FitOutside"):
        needle = prefix.lower() + "_"
        if lower.startswith(needle):
            try:
                return _parse_length(stem[len(prefix) + 1 :])
            except ValueError:
                return None
    return None


def _is_fit_offset_object(obj) -> bool:
    if _named_fit_offset(obj.name) is not None:
        return True
    return "fit_offset" in obj


def _named_fit_to(name: str) -> str | None:
    stem = _object_stem(name)
    lower = stem.lower()
    if not lower.startswith("fitto_"):
        return None
    token = stem.split("_", 1)[1].replace("_", "").lower()
    for region in FIT_TO_REGIONS:
        if token == region.lower():
            return region
    return None


def _is_fit_to_object(obj) -> bool:
    if _named_fit_to(obj.name) is not None:
        return True
    return "fit_to" in obj


def _is_fit_meta_object(obj) -> bool:
    return _is_fit_offset_object(obj) or _is_fit_to_object(obj)


def _read_fit_offset(objects) -> float | None:
    found = None
    for obj in objects:
        if "fit_offset" in obj:
            found = float(obj["fit_offset"])
        parsed = _named_fit_offset(obj.name)
        if parsed is not None:
            found = parsed
    return found


def _read_fit_to(objects) -> str | None:
    found = None
    for obj in objects:
        if "fit_to" in obj:
            token = str(obj["fit_to"]).replace("_", "").lower()
            match = next((r for r in FIT_TO_REGIONS if r.lower() == token), None)
            if match is None:
                print(f"main.py: unknown fit_to={obj['fit_to']!r} on {obj.name}", file=sys.stderr)
                sys.exit(1)
            found = match
        parsed = _named_fit_to(obj.name)
        if parsed is not None:
            found = parsed
        elif _is_fit_to_prefix(obj.name):
            print(f"main.py: unknown FitTo region on {obj.name}", file=sys.stderr)
            sys.exit(1)
    return found


def _is_fit_to_prefix(name: str) -> bool:
    return _object_stem(name).lower().startswith("fitto_")


def _bone_ends(armature, name: str):
    bone = armature.data.bones.get(name)
    if bone is None:
        print(f"main.py: armature {armature.name!r} missing bone {name}", file=sys.stderr)
        sys.exit(1)
    mw = armature.matrix_world
    return mw @ bone.head_local, mw @ bone.tail_local, bone.length


def _mesh_axis_bounds(obj, axis: int) -> tuple[float, float]:
    coords = [vert.co[axis] for vert in obj.data.vertices]
    return min(coords), max(coords)


def _region_aabb(armature, region: str, body) -> tuple[tuple[float, float], tuple[float, float], tuple[float, float]] | None:
    if region == "FullBody":
        return None

    y_lo, y_hi = _mesh_axis_bounds(body, 1)
    _sh_l_h, sh_l_t, _sh_l_len = _bone_ends(armature, "shoulder.L")
    _sh_r_h, sh_r_t, _sh_r_len = _bone_ends(armature, "shoulder.R")
    _hum_l_h, _hum_l_t, hum_len = _bone_ends(armature, "humerus.L")
    _fa_l_h, fa_l_t, fa_len = _bone_ends(armature, "forearm.L")
    _fa_r_h, fa_r_t, _fa_r_len = _bone_ends(armature, "forearm.R")
    femur_h, _femur_t, femur_len = _bone_ends(armature, "femur.L")
    _shin_h, shin_t, _shin_len = _bone_ends(armature, "shin.L")
    _root_h, _root_t, root_len = _bone_ends(armature, "root")
    _neck_h, neck_t, _neck_len = _bone_ends(armature, "upper_neck")
    pelvis_h, pelvis_t, _pelvis_len = _bone_ends(armature, "pelvis.L")
    _th_h, _th_t, thigh_thick = _bone_ends(armature, "thigh_thickness.L")

    hip_z = femur_h.z
    neck_z = neck_t.z
    torso_z_lo = hip_z - TORSO_LEG_FRAC * femur_len
    shoulder_x = max(abs(sh_l_t.x), abs(sh_r_t.x))
    torso_x = shoulder_x + TORSO_ARM_FRAC * hum_len
    arm_x = max(abs(fa_l_t.x), abs(fa_r_t.x)) + HAND_PAD_FRAC * fa_len
    leg_x = max(abs(pelvis_h.x), abs(pelvis_t.x)) + thigh_thick + LEG_FLESH_PAD
    foot_z = min(shin_t.z, _mesh_axis_bounds(body, 2)[0]) - 0.04
    lower_z_hi = hip_z + LOWER_HIP_FRAC * root_len

    if region == "Torso":
        x_hi = torso_x
        z_lo, z_hi = torso_z_lo, neck_z
    elif region == "UpperBody":
        x_hi = arm_x
        z_lo, z_hi = torso_z_lo, neck_z
    elif region == "LowerBody":
        x_hi = leg_x
        z_lo, z_hi = foot_z, lower_z_hi
    elif region == "LegsAndTorso":
        x_hi = max(torso_x, leg_x)
        z_lo, z_hi = foot_z, _mesh_axis_bounds(body, 2)[1]
    else:
        print(f"main.py: unknown fit region {region}", file=sys.stderr)
        sys.exit(1)

    return (-x_hi, x_hi), (y_lo, y_hi), (z_lo, z_hi)


def _make_aabb_cube(aabb, name: str):
    import bmesh
    import bpy
    from mathutils import Vector

    (x_lo, x_hi), (y_lo, y_hi), (z_lo, z_hi) = aabb
    mesh = bpy.data.meshes.new(name)
    cube = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(cube)
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=2.0)
    center = Vector(((x_lo + x_hi) * 0.5, (y_lo + y_hi) * 0.5, (z_lo + z_hi) * 0.5))
    scale = Vector(((x_hi - x_lo) * 0.5, (y_hi - y_lo) * 0.5, (z_hi - z_lo) * 0.5))
    bmesh.ops.scale(bm, verts=list(bm.verts), vec=scale)
    bmesh.ops.translate(bm, verts=list(bm.verts), vec=center)
    bm.to_mesh(mesh)
    bm.free()
    mesh.update()
    return cube


def _clip_body_to_aabb(body, aabb):
    clipped = _copy_mesh(body, "FitRegion")
    cube = _make_aabb_cube(aabb, "FitRegionCube")
    _recalc_normals(clipped)
    backup = _copy_mesh(clipped, "FitRegionBackup")
    for solver in ("FLOAT", "EXACT", "MANIFOLD"):
        existing = clipped.modifiers.get("FitClip")
        if existing is not None:
            clipped.modifiers.remove(existing)
        _restore_mesh(clipped, backup)
        clip = clipped.modifiers.new("FitClip", "BOOLEAN")
        clip.operation = "INTERSECT"
        clip.operand_type = "OBJECT"
        clip.object = cube
        if hasattr(clip, "solver"):
            try:
                clip.solver = solver
            except TypeError:
                print(f"boolean solver {solver} unavailable")
                clipped.modifiers.remove(clip)
                continue
        try:
            _apply_modifier(clipped, "FitClip")
        except Exception as exc:
            print(f"region intersect {solver} apply failed ({exc})")
            continue
        faces = len(clipped.data.polygons)
        print(
            f"fit region INTERSECT {solver} verts={len(backup.data.vertices)}->"
            f"{len(clipped.data.vertices)} faces={len(backup.data.polygons)}->{faces}"
        )
        if faces > 0:
            _delete(backup)
            _delete(cube)
            _merge_by_distance(clipped)
            _recalc_normals(clipped)
            return clipped
    _delete(backup)
    _delete(cube)
    _delete(clipped)
    print("main.py: FitTo cube missed the body mesh", file=sys.stderr)
    sys.exit(1)


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
    meta = []
    for obj in data_to.objects:
        if obj is None:
            continue
        if _is_fit_meta_object(obj) and obj.type != "MESH":
            if obj.name not in bpy.context.scene.collection.objects:
                bpy.context.scene.collection.objects.link(obj)
            meta.append(obj)
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

    return imported, meta


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
    if not meshes:
        print("main.py: no garment mesh objects found", file=sys.stderr)
        sys.exit(1)
    garments = [
        obj
        for obj in meshes
        if not _is_helper_mesh(obj.name) and not _is_host_body_mesh(obj.name)
    ]
    if garments:
        return garments
    meshes.sort(key=lambda obj: len(obj.data.vertices), reverse=True)
    print(
        f"no named garment mesh; using largest solid {meshes[0].name} "
        f"({len(meshes[0].data.vertices)} verts)"
    )
    return [meshes[0]]


def _scene_armature():
    import bpy

    named = bpy.data.objects.get("Humanoid")
    if named is not None and named.type == "ARMATURE":
        return named
    armatures = [obj for obj in bpy.data.objects if obj.type == "ARMATURE"]
    if not armatures:
        print("main.py: no armature in the clothing scene", file=sys.stderr)
        sys.exit(1)
    return armatures[0]


def _recalc_normals(obj) -> None:
    import bmesh

    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.normal_update()
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()


def _detach_helpers(keep) -> None:
    import bpy

    keep_names = {obj.name for obj in keep}
    for obj in list(bpy.data.objects):
        if obj.name in keep_names:
            continue
        if obj.type != "MESH" or not _is_helper_mesh(obj.name):
            continue
        print(f"dropping helper mesh {obj.name}")
        _delete(obj)


def _delete(obj) -> None:
    import bpy

    mesh = obj.data if obj.type == "MESH" else None
    bpy.data.objects.remove(obj, do_unlink=True)
    if mesh is not None and mesh.users == 0:
        bpy.data.meshes.remove(mesh)


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


def _apply_transform(obj) -> None:
    import bpy

    _select_only([obj])
    ctx = bpy.context.copy()
    ctx["object"] = obj
    ctx["active_object"] = obj
    ctx["selected_objects"] = [obj]
    ctx["selected_editable_objects"] = [obj]
    with bpy.context.temp_override(**ctx):
        bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)


def _merge_by_distance(obj, dist: float = 1e-5) -> None:
    import bmesh

    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.remove_doubles(bm, verts=list(bm.verts), dist=dist)
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()


def _strip_modifiers(obj) -> None:
    obj.parent = None
    for mod in list(obj.modifiers):
        print(f"stripping live {mod.type} {mod.name} (not applying)")
        obj.modifiers.remove(mod)


def _clear_vertex_groups(obj) -> None:
    for group in list(obj.vertex_groups):
        obj.vertex_groups.remove(group)


def _surface_bvh(obj):
    import bmesh
    from mathutils.bvhtree import BVHTree

    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.transform(obj.matrix_world)
    bm.normal_update()
    tree = BVHTree.FromBMesh(bm, epsilon=1e-6)
    bm.free()
    return tree


def _inside_vert_count(garment, bvh, eps: float = INSIDE_EPS) -> int:
    mw = garment.matrix_world
    inside = 0
    for vert in garment.data.vertices:
        point = mw @ vert.co
        loc, normal, _index, _dist = bvh.find_nearest(point)
        if loc is None:
            continue
        if (point - loc).dot(normal) < -eps:
            inside += 1
    return inside


def _shrinkwrap_outside(garment, target, offset: float) -> int:
    before = _coords(garment)
    name = "FitWrap"
    wrap = garment.modifiers.new(name, "SHRINKWRAP")
    wrap.target = target
    wrap.wrap_method = "TARGET_PROJECT"
    wrap.wrap_mode = "OUTSIDE"
    wrap.offset = offset
    garment.modifiers.move(garment.modifiers.find(name), 0)
    _apply_modifier(garment, name)
    after = _coords(garment)
    moved = _moved_vert_count(before, after)
    print(
        f"shrinkwrap OUTSIDE TARGET_PROJECT offset={offset}m "
        f"verts={len(garment.data.vertices)} moved={moved}"
    )
    return moved


def _wrap_once(garment, target, offset: float) -> None:
    bvh = _surface_bvh(target)
    total = len(garment.data.vertices)
    inside = _inside_vert_count(garment, bvh)
    print(f"wrap start inside={inside}/{total}")
    _shrinkwrap_outside(garment, target, offset)
    print(f"wrap done inside={_inside_vert_count(garment, bvh)}/{len(garment.data.vertices)}")


def _copy_mesh(obj, name: str):
    import bpy

    dup = obj.copy()
    dup.data = obj.data.copy()
    dup.name = name
    bpy.context.scene.collection.objects.link(dup)
    for mod in list(dup.modifiers):
        dup.modifiers.remove(mod)
    return dup


def _restore_mesh(dst, src) -> None:
    import bpy

    old = dst.data
    dst.data = src.data.copy()
    if old.users == 0:
        bpy.data.meshes.remove(old)
    dst.data.update()


def _weighted_vert_count(obj) -> int:
    return sum(1 for vert in obj.data.vertices if vert.groups)


def _transfer_weights_from_body(garment, body) -> None:
    if not body.vertex_groups:
        print("main.py: body has no vertex groups to transfer", file=sys.stderr)
        sys.exit(1)
    for group in body.vertex_groups:
        if garment.vertex_groups.get(group.name) is None:
            garment.vertex_groups.new(name=group.name)
    name = "FitWeights"
    transfer = garment.modifiers.new(name, "DATA_TRANSFER")
    transfer.object = body
    transfer.use_vert_data = True
    transfer.data_types_verts = {"VGROUP_WEIGHTS"}
    transfer.vert_mapping = "POLYINTERP_NEAREST"
    transfer.layers_vgroup_select_src = "ALL"
    transfer.layers_vgroup_select_dst = "NAME"
    transfer.mix_mode = "REPLACE"
    _apply_modifier(garment, name)
    weighted = _weighted_vert_count(garment)
    print(
        f"transferred {len(garment.vertex_groups)} vertex groups from body "
        f"({weighted}/{len(garment.data.vertices)} weighted verts)"
    )
    if weighted == 0:
        print("main.py: body weight transfer left every vertex unweighted", file=sys.stderr)
        sys.exit(1)


def _bind_armature(garment, armature) -> None:
    import bpy

    existing = garment.modifiers.get("Armature")
    if existing is not None:
        garment.modifiers.remove(existing)
    mod = garment.modifiers.new("Armature", "ARMATURE")
    mod.object = armature
    garment.parent = armature
    garment.parent_type = "OBJECT"


def _rerig(garment, armature, body) -> None:
    weighted = _weighted_vert_count(garment)
    if weighted == len(garment.data.vertices):
        print(f"preserving authored weights verts={weighted}")
    else:
        print(
            f"authored weights incomplete ({weighted}/{len(garment.data.vertices)}); "
            "transferring from body"
        )
        _clear_vertex_groups(garment)
        _transfer_weights_from_body(garment, body)
    _bind_armature(garment, armature)


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
    clothes_objects = list(bpy.data.objects)
    targets, body_meta = _append_body_meshes(body_path)
    body = _join_targets(targets)
    _apply_transform(body)
    offset = DEFAULT_OFFSET
    clothes_offset = _read_fit_offset(clothes_objects)
    if clothes_offset is not None:
        offset = clothes_offset
    body_offset = _read_fit_offset(body_meta)
    if body_offset is not None:
        offset = body_offset
    if args.offset is not None:
        offset = args.offset
    offset = max(0.0, offset)
    print(f"fit offset={offset}m")
    garments = _garment_objects(args.garment, exclude=[body])
    armature = _scene_armature()
    region = DEFAULT_FIT_TO
    clothes_region = _read_fit_to(clothes_objects)
    if clothes_region is not None:
        region = clothes_region
    body_region = _read_fit_to(body_meta)
    if body_region is not None:
        region = body_region
    if args.fit_to is not None:
        region = args.fit_to
    aabb = _region_aabb(armature, region, body)
    if aabb is None:
        print(f"fit_to={region}")
        fit_body = body
        owns_region = False
    else:
        (x_lo, x_hi), (y_lo, y_hi), (z_lo, z_hi) = aabb
        print(
            f"fit_to={region} "
            f"x=[{x_lo:.3f},{x_hi:.3f}] y=[{y_lo:.3f},{y_hi:.3f}] z=[{z_lo:.3f},{z_hi:.3f}]"
        )
        fit_body = _clip_body_to_aabb(body, aabb)
        owns_region = True

    for garment in garments:
        print(f"fit {garment.name} verts={len(garment.data.vertices)}")
        _strip_modifiers(garment)
        _apply_transform(garment)
        _wrap_once(garment, fit_body, offset)
        _rerig(garment, armature, body)
        _assert_reasonable_bounds(garment)

    for obj in body_meta:
        _delete(obj)
    if owns_region:
        _delete(fit_body)
    _delete(body)
    _detach_helpers(garments + [armature])
    _export_glb(output_path, garments + [armature])


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception:
        import traceback

        traceback.print_exc()
        sys.exit(1)
