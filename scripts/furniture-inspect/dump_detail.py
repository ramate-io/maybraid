"""Dump a few vertex samples from the active furniture mesh."""

import bpy


def main():
    for obj in bpy.data.objects:
        if obj.type != "MESH":
            continue
        me = obj.data
        print(f"MESH {obj.name} verts={len(me.vertices)} faces={len(me.polygons)}")
        zs = [v.co.z for v in me.vertices]
        print(f"  z min/max {min(zs):.4f} {max(zs):.4f}")
        for i, v in enumerate(me.vertices[:8]):
            print(f"  v{i} {v.co.x:.3f} {v.co.y:.3f} {v.co.z:.3f}")
        areas = [p.area for p in me.polygons]
        print(f"  face area min/max {min(areas):.4f} {max(areas):.4f}")


if __name__ == "__main__":
    main()
