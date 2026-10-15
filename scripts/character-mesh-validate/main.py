#!/usr/bin/env python3
"""Compare OBJ group bounds against ECS validation.json from character-mesh-export."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
import trimesh


def group_bounds(mesh: trimesh.Trimesh) -> tuple[np.ndarray, np.ndarray]:
    return mesh.bounds[0], mesh.bounds[1]


def max_corner_delta(a_min, a_max, b_min, b_max) -> float:
    return max(float(np.linalg.norm(a_min - b_min)), float(np.linalg.norm(a_max - b_max)))


def main() -> int:
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} <export.obj> <export.validation.json>", file=sys.stderr)
        return 2
    obj_path = Path(sys.argv[1])
    validation_path = Path(sys.argv[2])
    report = json.loads(validation_path.read_text())
    scene = trimesh.load(str(obj_path), process=False, maintain_order=True)
    if not isinstance(scene, trimesh.Scene):
        print("expected scene OBJ with groups", file=sys.stderr)
        return 1

    ecs_bounds = report["ecs_group_bounds"]
    print(f"groups_ecs={len(ecs_bounds)} groups_obj={len(scene.geometry)}")
    max_delta = 0.0
    for obj_name, mesh in scene.geometry.items():
        key = obj_name[: -len("_mat")] if obj_name.endswith("_mat") else obj_name
        if key not in ecs_bounds:
            print(f"warning: OBJ group {obj_name} missing from ECS validation")
            continue
        o_min, o_max = group_bounds(mesh)
        e = ecs_bounds[key]
        e_min = np.array(e["min"], dtype=float)
        e_max = np.array(e["max"], dtype=float)
        delta = max_corner_delta(e_min, e_max, o_min, o_max)
        max_delta = max(max_delta, delta)
        print(
            f"{key}: delta_m={delta:.6f} ecs_min={e_min.round(3).tolist()} "
            f"obj_min={o_min.round(3).tolist()}"
        )

    combined = trimesh.util.concatenate(tuple(scene.geometry.values()))
    print(
        "combined_obj_bbox_min",
        combined.bounds[0].round(3).tolist(),
        "max",
        combined.bounds[1].round(3).tolist(),
    )
    print(
        "combined_ecs_bbox_min",
        report["combined_ecs_bbox"]["min"],
        "max",
        report["combined_ecs_bbox"]["max"],
    )
    print(f"max_group_bbox_corner_delta_m={max_delta:.6f}")
    return 0 if max_delta < 1e-4 else 1


if __name__ == "__main__":
    raise SystemExit(main())
