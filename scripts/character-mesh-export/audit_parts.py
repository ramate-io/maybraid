#!/usr/bin/env python3
"""Cross-check exported OBJ groups vs Epiphant default_preview recipe."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
import trimesh

def obj_object_names(path: Path) -> list[str]:
    names: list[str] = []
    for line in path.read_text().splitlines():
        if line.startswith("o "):
            names.append(line[2:].strip())
    return names


EXPECTED = [
    "body__epiphant",
    "head__meerkat",
    "tail__cat-tail",
    "eye_left__standard",
    "eye_right__standard",
    "nose__trunkish",
    "ear_left__epiphant",
    "ear_right__epiphant",
]


def bbox(mesh: trimesh.Trimesh) -> tuple[np.ndarray, np.ndarray]:
    return mesh.bounds[0], mesh.bounds[1]


def main() -> int:
    if len(sys.argv) != 4:
        print(
            f"usage: {sys.argv[0]} <export.obj> <diagnose.json> <out.json>",
            file=sys.stderr,
        )
        return 2
    obj_path = Path(sys.argv[1])
    diagnose_path = Path(sys.argv[2])
    out_path = Path(sys.argv[3])
    diagnose = json.loads(diagnose_path.read_text())
    group_keys = sorted(obj_object_names(obj_path))
    scene = trimesh.load(str(obj_path), process=False, maintain_order=True)
    if isinstance(scene, trimesh.Trimesh):
        meshes = {"mesh": scene}
    else:
        meshes = dict(scene.geometry.items())
    if group_keys and all(k.startswith(obj_path.name) for k in meshes):
        # trimesh lost `o` names; load per-group by scanning OBJ.
        meshes = {}
        current = None
        verts: list[list[float]] = []
        faces: list[list[int]] = []
        for line in obj_path.read_text().splitlines():
            if line.startswith("o "):
                if current and verts:
                    meshes[current] = trimesh.Trimesh(
                        vertices=verts, faces=faces, process=False
                    )
                current = line[2:].strip()
                verts, faces = [], []
            elif line.startswith("v "):
                verts.append([float(x) for x in line.split()[1:4]])
            elif line.startswith("f "):
                idx = []
                for tok in line.split()[1:]:
                    idx.append(int(tok.split("/")[0]) - 1)
                if len(idx) >= 3:
                    for i in range(1, len(idx) - 1):
                        faces.append([idx[0], idx[i], idx[i + 1]])
        if current and verts:
            meshes[current] = trimesh.Trimesh(vertices=verts, faces=faces, process=False)
    missing = [e for e in EXPECTED if e not in group_keys]
    extra = [g for g in group_keys if g not in EXPECTED]

    parts = {}
    for name, g in meshes.items():
        mn, mx = bbox(g)
        parts[name] = {
            "min": mn.tolist(),
            "max": mx.tolist(),
            "extent": (mx - mn).tolist(),
        }

    def part_mesh(name: str):
        return meshes.get(name) or meshes.get(f"{name}_mat")

    body = part_mesh("body__epiphant")
    head = part_mesh("head__meerkat")
    tail = part_mesh("tail__cat-tail")
    proportions = {}
    if body is not None and head is not None:
        bmn, bmx = bbox(body)
        hmn, hmx = bbox(head)
        body_len = float(np.linalg.norm(bmx - bmn))
        head_len = float(np.linalg.norm(hmx - hmn))
        proportions["head_to_body_extent_ratio"] = head_len / body_len if body_len else None
        proportions["head_center_y"] = float((hmn[1] + hmx[1]) * 0.5)
        proportions["body_center_y"] = float((bmn[1] + bmx[1]) * 0.5)
    if body is not None and tail is not None:
        tmn, tmx = bbox(tail)
        proportions["tail_center"] = ((tmn + tmx) * 0.5).tolist()
        proportions["body_rear_z"] = float(bmn[2])
        proportions["tail_behind_body"] = float((tmn[2] + tmx[2]) * 0.5) < float(
            (bmn[2] + bmx[2]) * 0.5
        )
    ear_l = part_mesh("ear_left__epiphant")
    ear_r = part_mesh("ear_right__epiphant")
    eye_l = part_mesh("eye_left__standard")
    eye_r = part_mesh("eye_right__standard")
    if ear_l is not None and ear_r is not None:
        lmn, lmx = bbox(ear_l)
        rmn, rmx = bbox(ear_r)
        proportions["ear_left_center_x"] = float((lmn[0] + lmx[0]) * 0.5)
        proportions["ear_right_center_x"] = float((rmn[0] + rmx[0]) * 0.5)
        proportions["ears_left_right_order"] = proportions["ear_left_center_x"] > 0.0 and (
            proportions["ear_right_center_x"] < 0.0
        )
    if head is not None and eye_l is not None and eye_r is not None:
        hmn, hmx = bbox(head)
        lmn, lmx = bbox(eye_l)
        rmn, rmx = bbox(eye_r)
        proportions["eye_left_center"] = ((lmn + lmx) * 0.5).tolist()
        proportions["eye_right_center"] = ((rmn + rmx) * 0.5).tolist()
        proportions["eyes_on_head"] = (
            float(lmn[2]) >= float(hmn[2]) - 0.05
            and float(rmn[2]) >= float(hmn[2]) - 0.05
            and float(lmx[2]) <= float(hmx[2]) + 0.05
            and float(rmx[2]) <= float(hmx[2]) + 0.05
        )

    report = {
        "expected_groups": EXPECTED,
        "obj_groups": group_keys,
        "missing_groups": missing,
        "extra_groups": extra,
        "diagnose_part_count": len(diagnose.get("parts", [])),
        "proportions": proportions,
        "parts_bbox": parts,
    }
    out_path.write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0 if not missing and not extra else 1


if __name__ == "__main__":
    raise SystemExit(main())
