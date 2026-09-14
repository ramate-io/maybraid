"""Author low-poly furniture fill kits under ``maybraid/art/furniture``.

Invoked headlessly:

    blender --background --python scripts/furniture-author/main.py -- \\
        [--only substring] [--preview-dir DIR]

Each kit is one mesh, origin at world 0, default Camera + Light kept (same
as the existing bed / chair / chest / counter files). Authored space is
Blender Z-up:

- box carcass: X,Y ∈ [-1, 1], Z ∈ [0, 1] (floor origin, +Y is the wall / back)
- hinge door: origin at hinge-front-bottom; X ∈ [0, 1], Y thin about 0, Z ∈ [0, 1]
- basin: sit-on-top, contact at Z = 0, bowl in +Z
- faucet: deck mount at origin, spout toward −Y
- props (fruit / bread / display): sit on Z = 0
- shelf row: poles Z ∈ [0, 1], slanted deck starts at Z = 0 (stack on Z)
"""

from __future__ import annotations

import math
import os
import sys
from typing import Callable

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
ART = os.path.join(REPO, "maybraid", "art", "furniture")

KitFn = Callable[[], None]
KITS: list[tuple[str, KitFn]] = []


def kit(relpath: str) -> Callable[[KitFn], KitFn]:
    def deco(fn: KitFn) -> KitFn:
        KITS.append((relpath, fn))
        return fn

    return deco


def _bpy():
    import bpy

    return bpy


def _bmesh():
    import bmesh

    return bmesh


def _vec(*xyz):
    from mathutils import Vector

    return Vector(xyz)


def reset_scene() -> None:
    bpy = _bpy()
    bpy.ops.wm.read_factory_settings(use_empty=False)
    for obj in list(bpy.data.objects):
        if obj.type == "MESH":
            bpy.data.objects.remove(obj, do_unlink=True)
    bpy.context.scene.cursor.location = (0.0, 0.0, 0.0)


def link(obj):
    bpy = _bpy()
    if obj.name not in bpy.context.scene.collection.objects:
        bpy.context.scene.collection.objects.link(obj)
    return obj


def shade_flat(obj) -> None:
    bpy = _bpy()
    for poly in obj.data.polygons:
        poly.use_smooth = False
    obj.data.update()
    bpy.context.view_layer.objects.active = obj


def apply_object(obj) -> None:
    bpy = _bpy()
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)


def finish(name: str):
    bpy = _bpy()
    meshes = [o for o in bpy.data.objects if o.type == "MESH"]
    if not meshes:
        raise RuntimeError(f"{name}: no mesh objects")
    bpy.ops.object.select_all(action="DESELECT")
    for obj in meshes:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = meshes[0]
    if len(meshes) > 1:
        bpy.ops.object.join()
    obj = bpy.context.view_layer.objects.active
    apply_object(obj)
    bpy.context.scene.cursor.location = (0.0, 0.0, 0.0)
    bpy.ops.object.origin_set(type="ORIGIN_CURSOR")
    obj.name = name
    obj.data.name = name
    shade_flat(obj)
    return obj


def box(name: str, xmin, xmax, ymin, ymax, zmin, zmax):
    bpy = _bpy()
    bmesh = _bmesh()
    mesh = bpy.data.meshes.new(name)
    obj = bpy.data.objects.new(name, mesh)
    link(obj)
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=2.0)
    bmesh.ops.scale(
        bm,
        verts=list(bm.verts),
        vec=_vec((xmax - xmin) * 0.5, (ymax - ymin) * 0.5, (zmax - zmin) * 0.5),
    )
    bmesh.ops.translate(
        bm,
        verts=list(bm.verts),
        vec=_vec((xmin + xmax) * 0.5, (ymin + ymax) * 0.5, (zmin + zmax) * 0.5),
    )
    bm.to_mesh(mesh)
    bm.free()
    mesh.update()
    return obj


def cyl(name: str, radius, depth, location, rotation=(0.0, 0.0, 0.0), vertices=8):
    bpy = _bpy()
    bpy.ops.mesh.primitive_cylinder_add(
        radius=radius,
        depth=depth,
        vertices=vertices,
        location=location,
        rotation=rotation,
    )
    obj = bpy.context.active_object
    obj.name = name
    apply_object(obj)
    return obj


def ico(name: str, radius, location, subdivisions=1):
    bpy = _bpy()
    bpy.ops.mesh.primitive_ico_sphere_add(
        radius=radius,
        subdivisions=subdivisions,
        location=location,
    )
    obj = bpy.context.active_object
    obj.name = name
    apply_object(obj)
    return obj


def uv_sphere(name: str, radius, location, segments=12, rings=8):
    bpy = _bpy()
    bpy.ops.mesh.primitive_uv_sphere_add(
        radius=radius,
        segments=segments,
        ring_count=rings,
        location=location,
    )
    obj = bpy.context.active_object
    obj.name = name
    apply_object(obj)
    return obj


def torus(name: str, major, minor, location, rotation=(0.0, 0.0, 0.0), major_seg=10, minor_seg=6):
    bpy = _bpy()
    bpy.ops.mesh.primitive_torus_add(
        major_radius=major,
        minor_radius=minor,
        major_segments=major_seg,
        minor_segments=minor_seg,
        location=location,
        rotation=rotation,
    )
    obj = bpy.context.active_object
    obj.name = name
    apply_object(obj)
    return obj


def boolean_difference(keep, cutter, name="Cut") -> None:
    bpy = _bpy()
    bpy.context.view_layer.objects.active = keep
    keep.select_set(True)
    cutter.select_set(False)
    mod = keep.modifiers.new(name, "BOOLEAN")
    mod.operation = "DIFFERENCE"
    mod.operand_type = "OBJECT"
    mod.object = cutter
    if hasattr(mod, "solver"):
        try:
            mod.solver = "EXACT"
        except TypeError:
            pass
    bpy.ops.object.modifier_apply(modifier=mod.name)
    bpy.data.objects.remove(cutter, do_unlink=True)


def scale_verts(obj, sx=1.0, sy=1.0, sz=1.0, origin=(0.0, 0.0, 0.0)) -> None:
    ox, oy, oz = origin
    for v in obj.data.vertices:
        v.co.x = ox + (v.co.x - ox) * sx
        v.co.y = oy + (v.co.y - oy) * sy
        v.co.z = oz + (v.co.z - oz) * sz
    obj.data.update()


def nudge_verts(obj, fn) -> None:
    for v in obj.data.vertices:
        v.co = fn(v.co)
    obj.data.update()


def rotate_obj(obj, euler) -> None:
    obj.rotation_euler = euler
    apply_object(obj)


def rotate_about(obj, center, euler) -> None:
    from mathutils import Euler, Vector

    pivot = Vector(center)
    rot = Euler(euler).to_matrix()
    for vert in obj.data.vertices:
        vert.co = rot @ (vert.co - pivot) + pivot
    obj.data.update()


def finish_named(name: str, keep):
    bpy = _bpy()
    for obj in list(bpy.data.objects):
        if obj.type == "MESH" and obj != keep:
            bpy.data.objects.remove(obj, do_unlink=True)
    keep.name = name
    apply_object(keep)
    bpy.context.scene.cursor.location = (0.0, 0.0, 0.0)
    bpy.ops.object.origin_set(type="ORIGIN_CURSOR")
    keep.data.name = name
    shade_flat(keep)
    return keep


def chamfer_slab(name: str, xmin, xmax, ymin, ymax, zmin, zmax, chamfer=0.22):
    """Axis-aligned slab with the four plan corners cut."""
    bpy = _bpy()
    c = min(chamfer, (xmax - xmin) * 0.35, (ymax - ymin) * 0.35)
    plan = [
        (xmin + c, ymin),
        (xmax - c, ymin),
        (xmax, ymin + c),
        (xmax, ymax - c),
        (xmax - c, ymax),
        (xmin + c, ymax),
        (xmin, ymax - c),
        (xmin, ymin + c),
    ]
    verts = [(x, y, zmin) for x, y in plan] + [(x, y, zmax) for x, y in plan]
    faces = [
        [0, 1, 2, 3, 4, 5, 6, 7],
        [15, 14, 13, 12, 11, 10, 9, 8],
    ]
    for i in range(8):
        j = (i + 1) % 8
        faces.append([i, j, j + 8, i + 8])
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(verts, [], faces)
    bm = _bmesh().new()
    bm.from_mesh(mesh)
    _bmesh().ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    link(obj)
    return obj


# ---------------------------------------------------------------------------
# Shelves
# ---------------------------------------------------------------------------


@kit("shelf/row/shelf_row_001.blend")
def shelf_row() -> None:
    """Pair of poles + slanted deck with a front lip. Deck starts at Z = 0."""
    cyl("pole_l", 0.07, 1.00, (-0.90, 0.55, 0.50), vertices=8)
    cyl("pole_r", 0.07, 1.00, (0.90, 0.55, 0.50), vertices=8)
    deck = box("deck", -0.86, 0.86, -0.95, 0.92, 0.00, 0.08)

    def slant(co):
        t = (co.y + 0.95) / 1.87
        co.z += 0.78 * max(0.0, min(1.0, t))
        return co

    nudge_verts(deck, slant)
    box("lip", -0.86, 0.86, -0.95, -0.80, 0.00, 0.20)
    finish("shelf_row_001")


# ---------------------------------------------------------------------------
# Refrigerator
# ---------------------------------------------------------------------------


@kit("refrigerator/body/refrigerator_body_001.blend")
def refrigerator_body() -> None:
    """Carcass with a front cavity and a freezer split. Floor origin, +Y wall."""
    body = box("body", -1.00, 1.00, -1.00, 1.00, 0.00, 1.00)
    cutter = box("cavity", -0.86, 0.86, -1.08, -0.48, 0.08, 0.94)
    boolean_difference(body, cutter, "Cavity")
    box("mullion", -0.86, 0.86, -0.58, -0.48, 0.66, 0.74)
    box("shelf_a", -0.82, 0.82, -0.58, 0.88, 0.28, 0.32)
    box("shelf_b", -0.82, 0.82, -0.58, 0.88, 0.48, 0.52)
    box("toe", -1.00, 1.00, -1.00, -0.72, 0.00, 0.06)
    finish("refrigerator_body_001")


@kit("refrigerator/door/refrigerator_door_001.blend")
def refrigerator_door() -> None:
    """Hinge at X=0, Y=0 (front plane), Z=0. Panel in +X, proud toward −Y."""
    box("panel", 0.02, 0.98, -0.10, 0.02, 0.02, 0.98)
    box("inset", 0.14, 0.86, -0.14, -0.10, 0.14, 0.86)
    box("handle", 0.78, 0.94, -0.28, -0.10, 0.44, 0.56)
    finish("refrigerator_door_001")


# ---------------------------------------------------------------------------
# Wardrobe / locker
# ---------------------------------------------------------------------------


@kit("wardrobe/body/wardrobe_body_001.blend")
def wardrobe_body() -> None:
    """Tapered carcass, bun feet, gable cornice. Floor origin, +Y wall."""
    for i, (x, y) in enumerate(((-0.82, -0.78), (0.82, -0.78), (-0.82, 0.78), (0.82, 0.78))):
        cyl(f"foot_{i}", 0.10, 0.12, (x, y, 0.06), vertices=8)
    body = box("body", -1.00, 1.00, -0.92, 0.92, 0.12, 0.82)

    def taper(co):
        t = (co.z - 0.12) / 0.70
        s = 1.0 - 0.16 * max(0.0, min(1.0, t))
        co.x *= s
        return co

    nudge_verts(body, taper)
    cutter = box("cavity", -0.78, 0.78, -1.05, 0.55, 0.20, 0.76)
    boolean_difference(body, cutter, "Cavity")
    gable = box("gable", -0.86, 0.86, -0.78, 0.78, 0.78, 1.00)

    def ridge(co):
        if co.z > 0.86:
            co.y *= 1.0 - 0.92 * ((co.z - 0.86) / 0.14)
        return co

    nudge_verts(gable, ridge)
    box("shelf", -0.72, 0.72, -0.50, 0.55, 0.68, 0.74)
    cyl("rail", 0.04, 1.40, (0.0, 0.05, 0.70), rotation=(0.0, math.pi * 0.5, 0.0), vertices=8)
    finish("wardrobe_body_001")


@kit("wardrobe/door/wardrobe_door_001.blend")
def wardrobe_door() -> None:
    """Hinge at X=0. Trapezoid panel, diamond inset, peaked head."""
    panel = box("panel", 0.04, 0.96, -0.10, 0.02, 0.02, 0.78)

    def trap(co):
        t = (co.z - 0.02) / 0.76
        # Wider at the floor, narrower under the peak.
        mid = 0.50
        half = 0.46 - 0.10 * max(0.0, min(1.0, t))
        if co.x > mid:
            co.x = mid + (co.x - mid) * (half / 0.46)
        else:
            co.x = mid - (mid - co.x) * (half / 0.46)
        return co

    nudge_verts(panel, trap)
    peak = box("peak", 0.18, 0.82, -0.10, 0.02, 0.76, 0.98)

    def peak_ridge(co):
        if co.z > 0.80:
            t = (co.z - 0.80) / 0.18
            mid = 0.50
            co.x = mid + (co.x - mid) * (1.0 - 0.92 * t)
        return co

    nudge_verts(peak, peak_ridge)
    brace = box("brace", 0.22, 0.78, -0.14, -0.08, 0.42, 0.50)
    rotate_about(brace, (0.50, -0.11, 0.46), (0.0, math.radians(38.0), 0.0))
    brace_b = box("brace_b", 0.22, 0.78, -0.14, -0.08, 0.42, 0.50)
    rotate_about(brace_b, (0.50, -0.11, 0.46), (0.0, math.radians(-38.0), 0.0))
    box("handle", 0.78, 0.92, -0.22, -0.10, 0.40, 0.54)
    finish("wardrobe_door_001")


# ---------------------------------------------------------------------------
# Fruit — sit on Z = 0, roughly unit height
# ---------------------------------------------------------------------------


@kit("fruit/apple/apple_001.blend")
def apple() -> None:
    fruit = uv_sphere("body", 0.46, (0.0, 0.0, 0.46), segments=12, rings=8)
    scale_verts(fruit, sx=1.08, sy=1.04, sz=1.00, origin=(0.0, 0.0, 0.46))

    def pinch(co):
        if co.z > 0.78:
            t = (co.z - 0.78) / 0.22
            co.x *= 1.0 - 0.45 * t
            co.y *= 1.0 - 0.45 * t
            co.z -= 0.10 * t
        if co.z < 0.06:
            co.z = 0.0
        return co

    nudge_verts(fruit, pinch)
    cyl("stem", 0.03, 0.14, (0.0, 0.0, 0.90), vertices=6)
    leaf = uv_sphere("leaf", 0.10, (0.14, 0.02, 0.94), segments=8, rings=6)
    scale_verts(leaf, sx=1.60, sy=0.45, sz=0.35, origin=(0.14, 0.02, 0.94))
    finish("apple_001")


@kit("fruit/orange/orange_001.blend")
def orange() -> None:
    fruit = uv_sphere("body", 0.46, (0.0, 0.0, 0.42), segments=12, rings=8)
    scale_verts(fruit, sx=1.12, sy=1.12, sz=0.88, origin=(0.0, 0.0, 0.42))

    def sit(co):
        if co.z < 0.04:
            co.z = 0.0
        return co

    nudge_verts(fruit, sit)
    cyl("navel", 0.045, 0.04, (0.0, 0.0, 0.82), vertices=8)
    finish("orange_001")


@kit("fruit/pear/pear_001.blend")
def pear() -> None:
    fruit = uv_sphere("body", 0.42, (0.0, 0.0, 0.48), segments=12, rings=10)
    scale_verts(fruit, sx=1.00, sy=1.00, sz=1.20, origin=(0.0, 0.0, 0.48))

    def profile(co):
        t = max(0.0, min(1.0, co.z))
        if t < 0.42:
            w = 1.15
        else:
            w = 1.15 - 0.70 * ((t - 0.42) / 0.58)
        co.x *= w
        co.y *= w
        if co.z < 0.04:
            co.z = 0.0
        return co

    nudge_verts(fruit, profile)
    cyl("stem", 0.028, 0.12, (0.0, 0.0, 0.96), vertices=6)
    finish("pear_001")


@kit("fruit/banana/banana_001.blend")
def banana() -> None:
    """Crescent tube along X, resting on Z = 0."""
    bpy = _bpy()
    data = bpy.data.curves.new("banana_crv", "CURVE")
    data.dimensions = "3D"
    data.bevel_depth = 0.13
    data.bevel_resolution = 1
    data.resolution_u = 4
    spline = data.splines.new("POLY")
    pts = [
        (-0.95, 0.00, 0.52),
        (-0.50, 0.16, 0.18),
        (0.00, 0.26, 0.12),
        (0.50, 0.16, 0.20),
        (0.92, 0.02, 0.50),
    ]
    spline.points.add(len(pts) - 1)
    for i, (x, y, z) in enumerate(pts):
        spline.points[i].co = (x, y, z, 1.0)
    curve_obj = bpy.data.objects.new("banana_crv", data)
    link(curve_obj)
    bpy.ops.object.select_all(action="DESELECT")
    curve_obj.select_set(True)
    bpy.context.view_layer.objects.active = curve_obj
    bpy.ops.object.convert(target="MESH")
    cyl("stem", 0.04, 0.14, (0.90, 0.04, 0.58), rotation=(0.40, 0.65, 0.0), vertices=6)
    finish("banana_001")


# ---------------------------------------------------------------------------
# Bread
# ---------------------------------------------------------------------------


@kit("bread/loaf/loaf_001.blend")
def loaf() -> None:
    body = uv_sphere("body", 0.48, (0.0, 0.0, 0.36), segments=12, rings=8)
    scale_verts(body, sx=1.95, sy=0.78, sz=0.82, origin=(0.0, 0.0, 0.36))

    def sit(co):
        if co.z < 0.03:
            co.z = 0.0
        return co

    nudge_verts(body, sit)
    for i, x in enumerate((-0.40, 0.00, 0.40)):
        cutter = box(f"slash_{i}", x - 0.10, x + 0.10, -0.28, 0.28, 0.42, 0.95)
        rotate_about(cutter, (x, 0.0, 0.62), (0.0, math.radians(32.0), 0.0))
        boolean_difference(body, cutter, f"Slash{i}")
    finish_named("loaf_001", body)


@kit("bread/baguette/baguette_001.blend")
def baguette() -> None:
    body = uv_sphere("body", 0.22, (0.0, 0.0, 0.18), segments=10, rings=7)
    scale_verts(body, sx=4.40, sy=1.00, sz=0.90, origin=(0.0, 0.0, 0.18))

    def sit(co):
        if co.z < 0.02:
            co.z = 0.0
        return co

    nudge_verts(body, sit)
    for i, x in enumerate((-0.58, -0.20, 0.18, 0.56)):
        cutter = box(f"slash_{i}", x - 0.08, x + 0.08, -0.16, 0.16, 0.14, 0.48)
        rotate_about(cutter, (x, 0.0, 0.28), (0.0, math.radians(36.0), 0.0))
        boolean_difference(body, cutter, f"Slash{i}")
    finish_named("baguette_001", body)


@kit("bread/bun/bun_001.blend")
def bun() -> None:
    body = uv_sphere("body", 0.48, (0.0, 0.0, 0.28), segments=12, rings=8)
    scale_verts(body, sx=1.15, sy=1.15, sz=0.62, origin=(0.0, 0.0, 0.28))

    def sit(co):
        if co.z < 0.03:
            co.z = 0.0
        return co

    nudge_verts(body, sit)
    finish("bun_001")


@kit("bread/boule/boule_001.blend")
def boule() -> None:
    body = uv_sphere("body", 0.50, (0.0, 0.0, 0.38), segments=12, rings=8)
    scale_verts(body, sx=1.08, sy=1.04, sz=0.82, origin=(0.0, 0.0, 0.38))

    def sit(co):
        if co.z < 0.03:
            co.z = 0.0
        return co

    nudge_verts(body, sit)
    cutter = box("score", -0.03, 0.03, -0.35, 0.35, 0.58, 0.95)
    boolean_difference(body, cutter, "Score")
    finish("boule_001")


# ---------------------------------------------------------------------------
# Food display (sits on a counter)
# ---------------------------------------------------------------------------


@kit("food_display/case/food_display_001.blend")
def food_display() -> None:
    """50s diner pie stand: rounded tray, back, chrome hoop. Open front."""
    tray = cyl("tray", 1.00, 0.10, (0.0, 0.0, 0.05), vertices=12)
    scale_verts(tray, sx=1.00, sy=0.72, sz=1.00, origin=(0.0, 0.0, 0.05))
    rim = torus("rim", 0.90, 0.055, (0.0, 0.0, 0.12), major_seg=12, minor_seg=6)
    scale_verts(rim, sx=1.00, sy=0.72, sz=0.80, origin=(0.0, 0.0, 0.12))
    back = cyl("back", 0.96, 0.78, (0.0, 0.12, 0.52), vertices=12)
    scale_verts(back, sx=1.00, sy=0.62, sz=1.00, origin=(0.0, 0.12, 0.52))
    boolean_difference(back, box("front_cut", -1.20, 1.20, -1.20, 0.18, 0.10, 1.10), "OpenFront")
    hoop = torus(
        "hoop",
        0.78,
        0.045,
        (0.0, 0.10, 0.58),
        rotation=(math.pi * 0.5, 0.0, 0.0),
        major_seg=14,
        minor_seg=6,
    )
    boolean_difference(hoop, box("hoop_cut", -1.20, 1.20, -1.20, 1.20, -0.20, 0.32), "Arch")
    finish("food_display_001")


# ---------------------------------------------------------------------------
# Toilet
# ---------------------------------------------------------------------------


@kit("toilet/bowl/toilet_001.blend")
def toilet() -> None:
    """Abstract two-piece: tank at +Y, bowl toward −Y. Floor origin."""
    box("base", -0.42, 0.42, -0.55, 0.25, 0.00, 0.20)
    bowl = uv_sphere("bowl", 0.46, (0.0, -0.30, 0.36), segments=12, rings=8)
    scale_verts(bowl, sx=0.95, sy=1.12, sz=0.58, origin=(0.0, -0.30, 0.36))
    well = uv_sphere("well", 0.30, (0.0, -0.30, 0.44), segments=10, rings=6)
    scale_verts(well, sx=0.95, sy=1.05, sz=0.45, origin=(0.0, -0.30, 0.44))
    boolean_difference(bowl, well, "Well")
    cap = box("cap", -0.50, 0.50, -0.80, 0.20, 0.52, 0.90)
    boolean_difference(bowl, cap, "Open")
    torus("seat", 0.36, 0.06, (0.0, -0.30, 0.50), major_seg=12, minor_seg=6)
    box("trap", -0.16, 0.16, -0.05, 0.55, 0.16, 0.38)
    box("tank", -0.52, 0.52, 0.42, 0.95, 0.36, 0.90)
    box("lid", -0.56, 0.56, 0.40, 0.97, 0.90, 1.00)
    box("button", -0.10, 0.10, 0.58, 0.76, 0.98, 1.04)
    finish("toilet_001")


# ---------------------------------------------------------------------------
# Sinks — vanity carcass; basin sits on the deck; shared faucet
# ---------------------------------------------------------------------------


@kit("sink/vanity/vanity_001.blend")
def vanity() -> None:
    """Bathroom vanity: toekick on −Y, solid deck. Basin sits on top."""
    box("kick", -1.00, 1.00, -0.70, 1.00, 0.00, 0.12)
    box("carcass", -1.00, 1.00, -1.00, 1.00, 0.12, 0.86)
    box("deck", -1.06, 1.06, -1.06, 1.00, 0.86, 1.00)
    box("reveal", -0.03, 0.03, -1.00, -0.90, 0.20, 0.78)
    finish("vanity_001")


@kit("sink/basin/basin_001.blend")
def basin() -> None:
    """Sit-on-top bowl. Contact at Z = 0, rim in +Z."""
    bowl = uv_sphere("bowl", 0.72, (0.0, 0.0, 0.38), segments=12, rings=8)
    scale_verts(bowl, sx=1.18, sy=0.92, sz=0.70, origin=(0.0, 0.0, 0.38))

    def sit(co):
        if co.z < 0.03:
            co.z = 0.0
        return co

    nudge_verts(bowl, sit)
    inner = uv_sphere("inner", 0.58, (0.0, 0.0, 0.42), segments=12, rings=8)
    scale_verts(inner, sx=1.18, sy=0.92, sz=0.70, origin=(0.0, 0.0, 0.42))
    boolean_difference(bowl, inner, "Hollow")
    cap = box("cap", -1.20, 1.20, -1.20, 1.20, 0.62, 1.20)
    boolean_difference(bowl, cap, "Open")
    rim = torus("rim", 0.74, 0.07, (0.0, 0.0, 0.58), major_seg=12, minor_seg=6)
    scale_verts(rim, sx=1.18, sy=0.92, sz=0.55, origin=(0.0, 0.0, 0.58))
    cyl("drain", 0.07, 0.05, (0.0, 0.0, 0.08), vertices=8)
    finish("basin_001")


@kit("sink/faucet/faucet_001.blend")
def faucet() -> None:
    """Deck-mount at origin. Column +Z, spout toward −Y over the basin."""
    cyl("base", 0.16, 0.08, (0.0, 0.0, 0.04), vertices=8)
    cyl("column", 0.08, 0.55, (0.0, 0.0, 0.36), vertices=8)
    box("neck", -0.08, 0.08, -0.42, 0.08, 0.56, 0.72)
    cyl("spout", 0.07, 0.46, (0.0, -0.52, 0.58), rotation=(math.pi * 0.5, 0.0, 0.0), vertices=8)
    cyl("aer", 0.08, 0.08, (0.0, -0.74, 0.50), vertices=8)
    cyl("tap_l", 0.06, 0.22, (-0.22, 0.00, 0.28), rotation=(0.0, math.pi * 0.5, 0.0), vertices=6)
    cyl("tap_r", 0.06, 0.22, (0.22, 0.00, 0.28), rotation=(0.0, math.pi * 0.5, 0.0), vertices=6)
    cyl("knob_l", 0.09, 0.08, (-0.34, 0.00, 0.28), rotation=(0.0, math.pi * 0.5, 0.0), vertices=6)
    cyl("knob_r", 0.09, 0.08, (0.34, 0.00, 0.28), rotation=(0.0, math.pi * 0.5, 0.0), vertices=6)
    finish("faucet_001")


# ---------------------------------------------------------------------------
# Tables
# ---------------------------------------------------------------------------


@kit("table/top/table_top_001.blend")
def table_top() -> None:
    """Rectangular top with chamfered corners. Floor of the slab at Z = 0."""
    top = chamfer_slab("top", -1.00, 1.00, -1.00, 1.00, 0.00, 1.00, chamfer=0.22)
    finish_named("table_top_001", top)


@kit("table/top/table_top_round_001.blend")
def table_top_round() -> None:
    """Disc top. Floor of the slab at Z = 0."""
    top = cyl("top", 1.00, 1.00, (0.0, 0.0, 0.50), vertices=12)
    finish_named("table_top_round_001", top)


@kit("table/leg/table_leg_001.blend")
def table_leg() -> None:
    """Turned post. Plan about the origin, Z ∈ [0, 1] like the chair leg."""
    cyl("foot", 0.16, 0.10, (0.0, 0.0, 0.05), vertices=8)
    cyl("shaft", 0.10, 0.72, (0.0, 0.0, 0.46), vertices=8)
    cyl("ring", 0.14, 0.08, (0.0, 0.0, 0.78), vertices=8)
    cyl("neck", 0.08, 0.16, (0.0, 0.0, 0.92), vertices=8)
    finish("table_leg_001")


@kit("table/apron/table_apron_001.blend")
def table_apron() -> None:
    """One under-top rail on −Y. Instance around the top."""
    box("rail", -1.00, 1.00, -1.00, -0.72, 0.00, 1.00)
    finish("table_apron_001")


@kit("table/pedestal/table_pedestal_001.blend")
def table_pedestal() -> None:
    """Diner tulip: disc foot, column, cap. Floor origin."""
    cyl("foot", 0.95, 0.10, (0.0, 0.0, 0.05), vertices=12)
    cyl("flare", 0.42, 0.16, (0.0, 0.0, 0.16), vertices=10)
    cyl("column", 0.16, 0.62, (0.0, 0.0, 0.52), vertices=8)
    cyl("cap", 0.38, 0.10, (0.0, 0.0, 0.95), vertices=10)
    finish("table_pedestal_001")


# ---------------------------------------------------------------------------
# Save / preview
# ---------------------------------------------------------------------------


def save_blend(relpath: str) -> str:
    bpy = _bpy()
    path = os.path.join(ART, relpath)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=path)
    print(f"wrote {path}")
    return path


def render_preview(relpath: str, preview_dir: str) -> str:
    bpy = _bpy()
    scene = bpy.context.scene
    cam = bpy.data.objects.get("Camera")
    if cam is None:
        raise RuntimeError("missing Camera")
    cam.location = (3.2, -3.6, 1.8)
    cam.rotation_euler = (math.radians(68.0), 0.0, math.radians(42.0))
    scene.camera = cam
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.render.resolution_x = 512
    scene.render.resolution_y = 512
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    shading = scene.display.shading
    shading.light = "STUDIO"
    shading.color_type = "SINGLE"
    shading.single_color = (0.72, 0.74, 0.78)
    if hasattr(scene.display, "render_aa"):
        scene.display.render_aa = "8"
    out = os.path.join(preview_dir, relpath.replace("/", "_").replace(".blend", ".png"))
    os.makedirs(os.path.dirname(out), exist_ok=True)
    scene.render.filepath = out
    result = bpy.ops.render.render(write_still=True)
    if "FINISHED" not in result:
        raise RuntimeError(f"preview failed for {relpath}: {result}")
    print(f"preview {out}")
    return out


def parse_args(argv: list[str]) -> tuple[str | None, str | None]:
    only = None
    preview_dir = None
    args = argv[argv.index("--") + 1 :] if "--" in argv else argv[1:]
    i = 0
    while i < len(args):
        if args[i] == "--only" and i + 1 < len(args):
            only = args[i + 1]
            i += 2
            continue
        if args[i] == "--preview-dir" and i + 1 < len(args):
            preview_dir = args[i + 1]
            i += 2
            continue
        print(f"unknown arg {args[i]}", file=sys.stderr)
        sys.exit(2)
    return only, preview_dir


def main() -> None:
    only, preview_dir = parse_args(sys.argv)
    written = []
    for relpath, fn in KITS:
        if only and only not in relpath and only not in fn.__name__:
            continue
        reset_scene()
        fn()
        save_blend(relpath)
        if preview_dir:
            render_preview(relpath, preview_dir)
        written.append(relpath)
    if not written:
        print("no kits matched", file=sys.stderr)
        sys.exit(1)
    print(f"authored {len(written)} kits")


if __name__ == "__main__":
    main()
