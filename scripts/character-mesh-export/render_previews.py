#!/usr/bin/env python3
"""Render front/side/three-quarter PNGs from an assembled character OBJ."""

from __future__ import annotations

import sys
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
import trimesh


def load_vertices(obj_path: Path) -> np.ndarray:
    scene = trimesh.load(str(obj_path), process=False, maintain_order=True)
    if isinstance(scene, trimesh.Scene):
        meshes = list(scene.geometry.values())
        if not meshes:
            raise RuntimeError("empty OBJ scene")
        return trimesh.util.concatenate(meshes).vertices
    return scene.vertices


def render_view(verts: np.ndarray, elev: float, azim: float, out: Path) -> None:
    fig = plt.figure(figsize=(8, 8), dpi=120)
    ax = fig.add_subplot(111, projection="3d")
    ax.scatter(verts[:, 0], verts[:, 1], verts[:, 2], s=0.2, c=verts[:, 2], cmap="viridis")
    ax.view_init(elev=elev, azim=azim)
    mins = verts.min(axis=0)
    maxs = verts.max(axis=0)
    center = (mins + maxs) * 0.5
    radius = float(np.max(maxs - mins)) * 0.55
    ax.set_xlim(center[0] - radius, center[0] + radius)
    ax.set_ylim(center[1] - radius, center[1] + radius)
    ax.set_zlim(center[2] - radius, center[2] + radius)
    ax.set_box_aspect((1, 1, 1))
    ax.set_axis_off()
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, bbox_inches="tight", pad_inches=0.05)
    plt.close(fig)


def main() -> int:
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} <export.obj> <preview_dir>", file=sys.stderr)
        return 2
    obj_path = Path(sys.argv[1])
    preview_dir = Path(sys.argv[2])
    verts = load_vertices(obj_path)
    render_view(verts, elev=12, azim=-90, out=preview_dir / "epiphant_default_preview_front.png")
    render_view(verts, elev=12, azim=0, out=preview_dir / "epiphant_default_preview_side.png")
    render_view(verts, elev=18, azim=-135, out=preview_dir / "epiphant_default_preview_three_quarter.png")
    print(f"wrote previews under {preview_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
