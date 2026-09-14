"""Dump furniture blend mesh bounds, names, and poly counts."""

import os
import sys

import bpy


def bounds(obj):
    xs, ys, zs = [], [], []
    for v in obj.bound_box:
        w = obj.matrix_world @ __import__("mathutils").Vector(v)
        xs.append(w.x)
        ys.append(w.y)
        zs.append(w.z)
    return (min(xs), max(xs), min(ys), max(ys), min(zs), max(zs))


def main():
    print("FILE", bpy.data.filepath)
    print("OBJECTS", len(bpy.data.objects))
    for obj in bpy.data.objects:
        loc = obj.location
        print(
            f"OBJ name={obj.name!r} type={obj.type} loc=({loc.x:.4f},{loc.y:.4f},{loc.z:.4f}) "
            f"rot=({obj.rotation_euler.x:.4f},{obj.rotation_euler.y:.4f},{obj.rotation_euler.z:.4f}) "
            f"scale=({obj.scale.x:.4f},{obj.scale.y:.4f},{obj.scale.z:.4f})"
        )
        if obj.type == "MESH":
            me = obj.data
            xmin, xmax, ymin, ymax, zmin, zmax = bounds(obj)
            print(
                f"  MESH verts={len(me.vertices)} faces={len(me.polygons)} "
                f"bounds X[{xmin:.3f},{xmax:.3f}] Y[{ymin:.3f},{ymax:.3f}] Z[{zmin:.3f},{zmax:.3f}]"
            )
            print(f"  materials={[s.name for s in obj.material_slots]}")
            print(f"  modifiers={[m.type for m in obj.modifiers]}")
    print("---")


if __name__ == "__main__":
    main()
