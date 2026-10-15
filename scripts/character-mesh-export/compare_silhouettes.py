#!/usr/bin/env python3
"""Compare GPU runtime PNGs vs OBJ pyrender silhouettes at shared cameras."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from mpl_toolkits.mplot3d.art3d import Poly3DCollection
from PIL import Image

import trimesh


def silhouette_mask(rgba: np.ndarray, bg_thresh: float = 0.08) -> np.ndarray:
    """Foreground mask from distance to capture clear-color (Bevy PNGs are opaque)."""
    rgb = rgba[..., :3].astype(np.float32) / 255.0
    bg = np.array([0.12, 0.13, 0.16], dtype=np.float32)
    dist = np.linalg.norm(rgb - bg, axis=-1)
    rgb_mask = dist > bg_thresh
    if rgba.shape[-1] == 4:
        alpha = rgba[..., 3].astype(np.float32) / 255.0
        # Only trust alpha when it is not a flat opaque screenshot.
        if float(alpha.std()) > 0.02 and float(alpha.min()) < 0.98:
            alpha_mask = alpha > 0.35
            if alpha_mask.mean() > 0.001:
                return np.logical_and(rgb_mask, alpha_mask)
    return rgb_mask


def mask_iou(a: np.ndarray, b: np.ndarray) -> float:
    a = a.astype(bool)
    b = b.astype(bool)
    inter = np.logical_and(a, b).sum()
    union = np.logical_or(a, b).sum()
    return float(inter / union) if union else 0.0


def mask_extent(mask: np.ndarray) -> tuple[float, float, float, float]:
    ys, xs = np.where(mask)
    if len(xs) == 0:
        return 0.0, 0.0, 0.0, 0.0
    h = float(ys.max() - ys.min() + 1)
    w = float(xs.max() - xs.min() + 1)
    return h, w, float(ys.mean()), float(xs.mean())


def spherical_angles(eye: np.ndarray, center: np.ndarray) -> tuple[float, float, float]:
    d = eye - center
    dist = float(np.linalg.norm(d))
    if dist < 1e-6:
        return 0.0, 0.0, 1.0
    elev = float(np.degrees(np.arcsin(np.clip(d[1] / dist, -1.0, 1.0))))
    azim = float(np.degrees(np.arctan2(d[0], d[2])))
    return elev, azim, dist


def render_obj_silhouette(obj_path: Path, manifest: dict, view: dict) -> np.ndarray:
    """Matplotlib mesh raster (headless; approximates manifest camera)."""
    loaded = trimesh.load(str(obj_path), process=False, maintain_order=True)
    if isinstance(loaded, trimesh.Scene):
        mesh = trimesh.util.concatenate(tuple(loaded.geometry.values()))
    else:
        mesh = loaded
    center = np.array(manifest["center"], dtype=np.float64)
    eye = np.array(view["eye"], dtype=np.float64)
    elev, azim, _dist = spherical_angles(eye, center)
    extent = float(manifest["extent_m"])
    w, h = int(manifest["width"]), int(manifest["height"])
    dpi = 100
    fig = plt.figure(figsize=(w / dpi, h / dpi), dpi=dpi)
    fig.patch.set_facecolor((0.12, 0.13, 0.16))
    ax = fig.add_subplot(111, projection="3d")
    ax.set_facecolor((0.12, 0.13, 0.16))
    faces = mesh.vertices[mesh.faces]
    coll = Poly3DCollection(
        faces, facecolors=(0.75, 0.75, 0.78, 1.0), edgecolors="none", linewidths=0
    )
    ax.add_collection3d(coll)
    ax.view_init(elev=elev, azim=azim)
    r = extent * 0.55
    ax.set_xlim(center[0] - r, center[0] + r)
    ax.set_ylim(center[1] - r, center[1] + r)
    ax.set_zlim(center[2] - r, center[2] + r)
    ax.set_box_aspect((1, 1, 1))
    ax.set_axis_off()
    fig.canvas.draw()
    buf = np.asarray(fig.canvas.buffer_rgba())[:, :, :4]
    plt.close(fig)
    return buf.copy()


def to_rgba(img: np.ndarray) -> np.ndarray:
    if img.ndim == 2:
        img = np.stack([img] * 3, axis=-1)
    if img.shape[2] == 3:
        alpha = np.full((*img.shape[:2], 1), 255, dtype=img.dtype)
        img = np.concatenate([img, alpha], axis=2)
    return img[:, :, :4]


def side_by_side(left: np.ndarray, right: np.ndarray) -> np.ndarray:
    left = to_rgba(left)
    right = to_rgba(right)
    if left.shape[:2] != right.shape[:2]:
        right_img = Image.fromarray(right).resize((left.shape[1], left.shape[0]), Image.Resampling.BILINEAR)
        right = np.array(right_img)
    return np.concatenate([left, right], axis=1)


def main() -> int:
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} <compare_dir> <export.obj>", file=sys.stderr)
        return 2
    compare_dir = Path(sys.argv[1])
    obj_path = Path(sys.argv[2])
    manifest = json.loads((compare_dir / "cameras.json").read_text())
    report = {"views": [], "recipe_parts_ok": True}

    for view in manifest["views"]:
        name = view["name"]
        runtime_path = compare_dir / f"runtime_{name}.png"
        obj_gpu_path = compare_dir / f"obj_reimport_{name}.png"
        if not runtime_path.is_file() or not obj_gpu_path.is_file():
            print(f"missing {runtime_path} or {obj_gpu_path}", file=sys.stderr)
            return 1
        runtime = np.array(Image.open(runtime_path).convert("RGBA"))
        obj_rgba = np.array(Image.open(obj_gpu_path).convert("RGBA"))
        obj_matplotlib = render_obj_silhouette(obj_path, manifest, view)
        m_run = silhouette_mask(runtime)
        m_obj = silhouette_mask(obj_rgba)
        iou = mask_iou(m_run, m_obj)
        h_r, w_r, _, _ = mask_extent(m_run)
        h_o, w_o, _, _ = mask_extent(m_obj)
        m_mpl = silhouette_mask(obj_matplotlib)
        iou_mpl = mask_iou(m_run, m_mpl)
        area_r = float(m_run.mean())
        area_o = float(m_obj.mean())
        entry = {
            "view": name,
            "gpu_mask_iou_runtime_vs_obj_reimport": iou,
            "matplotlib_mask_iou_runtime_vs_obj": iou_mpl,
            "runtime_mask_height_px": h_r,
            "runtime_mask_width_px": w_r,
            "obj_reimport_mask_height_px": h_o,
            "obj_reimport_mask_width_px": w_o,
            "height_ratio_obj_over_runtime": (h_o / h_r) if h_r else None,
            "width_ratio_obj_over_runtime": (w_o / w_r) if w_r else None,
            "runtime_mask_area_fraction": area_r,
            "obj_reimport_mask_area_fraction": area_o,
        }
        report["views"].append(entry)
        combo = side_by_side(runtime, obj_rgba)
        Image.fromarray(combo).save(compare_dir / f"compare_{name}.png")
        print(
            f"{name}: GPU IoU={iou:.4f} (runtime vs obj_reimport) "
            f"matplotlib IoU={iou_mpl:.4f} runtime_hw=({h_r:.0f},{w_r:.0f}) "
            f"obj_hw=({h_o:.0f},{w_o:.0f})"
        )

    out = compare_dir / "silhouette_compare.json"
    out.write_text(json.dumps(report, indent=2))
    print(f"wrote {out}")
    worst = min(v["gpu_mask_iou_runtime_vs_obj_reimport"] for v in report["views"])
    return 0 if worst >= 0.92 else 1


if __name__ == "__main__":
    raise SystemExit(main())
