#!/usr/bin/env python3
"""Convert `epiphant_default_preview.obj` to a Bevy-loadable GLB.

This is a format conversion of the [#1589](https://github.com/ramate-io/maybraid/pull/1589)
export, not new authoring. It keeps:

- object names (`o body`, `o head`, …)
- material slot names (`body_mat`, …)
- vertex positions, normals, and per-corner UVs

glTF TEXCOORD origin is top-left; Wavefront `vt` origin is bottom-left, so V is
flipped (`v' = 1 - v`). Positions stay Y-up metres (no axis swap, no rescale).

Each independently paintable component is its own glTF scene so
`SceneRef::glb("…glb#SceneN")` can load one part. Scene index order (stable,
not OBJ file order):

0 body, 1 head, 2 ear_left, 3 ear_right, 4 eye_left, 5 eye_right, 6 nose, 7 tail

Materials are untextured white PBR named `{part}_mat`. Liam's Color / packed
metallic-roughness maps bind at runtime.

Usage (from the repo root):

    python3 maybraid/art/characters/exports/epiphant/obj_to_glb.py
"""

from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

SCENE_ORDER = (
	"body",
	"head",
	"ear_left",
	"ear_right",
	"eye_left",
	"eye_right",
	"nose",
	"tail",
)

HERE = Path(__file__).resolve().parent
DEFAULT_OBJ = HERE / "epiphant_default_preview.obj"
DEFAULT_GLB = (
	HERE.parents[3] / "assets" / "characters" / "epiphant" / "default" / "epiphant_default_preview.glb"
)


def pad4(data: bytes, pad: bytes) -> bytes:
	missing = (4 - (len(data) % 4)) % 4
	return data + pad * missing


def parse_obj(text: str) -> dict[str, dict]:
	objects: dict[str, dict] = {}
	current: str | None = None
	positions: list[tuple[float, float, float]] = []
	texcoords: list[tuple[float, float]] = []
	normals: list[tuple[float, float, float]] = []

	def ensure(name: str) -> dict:
		return objects.setdefault(
			name,
			{"material": None, "tris": []},
		)

	for raw in text.splitlines():
		if not raw or raw.startswith("#"):
			continue
		kind, *rest = raw.split()
		if kind == "o":
			current = rest[0]
			ensure(current)
		elif kind == "usemtl":
			if current is None:
				raise SystemExit("usemtl before o")
			ensure(current)["material"] = rest[0]
		elif kind == "v":
			positions.append((float(rest[0]), float(rest[1]), float(rest[2])))
		elif kind == "vt":
			texcoords.append((float(rest[0]), float(rest[1])))
		elif kind == "vn":
			normals.append((float(rest[0]), float(rest[1]), float(rest[2])))
		elif kind == "f":
			if current is None:
				raise SystemExit("face before o")
			if len(rest) != 3:
				raise SystemExit(f"expected triangles, got {len(rest)}-gon in {current}")
			tri = []
			for corner in rest:
				bits = corner.split("/")
				vi = int(bits[0]) - 1
				ti = int(bits[1]) - 1 if len(bits) > 1 and bits[1] else None
				ni = int(bits[2]) - 1 if len(bits) > 2 and bits[2] else None
				if ti is None:
					raise SystemExit(f"{current} face missing vt")
				tri.append((vi, ti, ni))
			ensure(current)["tris"].append(tri)

	for name, obj in objects.items():
		obj["positions"] = positions
		obj["texcoords"] = texcoords
		obj["normals"] = normals
		if not obj["tris"]:
			raise SystemExit(f"{name} has no faces")
		expected = f"{name}_mat"
		if obj["material"] != expected:
			raise SystemExit(f"{name} material {obj['material']!r} != {expected!r}")
	missing = [name for name in SCENE_ORDER if name not in objects]
	if missing:
		raise SystemExit(f"OBJ missing objects: {missing}")
	return objects


def mesh_buffers(obj: dict) -> tuple[bytes, list[float], list[float], int]:
	"""Weld unique (v,vt,vn) corners. Returns (blob, pos_min, pos_max, vertex_count)."""
	positions: list[float] = []
	normals: list[float] = []
	uvs: list[float] = []
	indices: list[int] = []
	weld: dict[tuple[int, int, int | None], int] = {}
	src_p = obj["positions"]
	src_t = obj["texcoords"]
	src_n = obj["normals"]
	for tri in obj["tris"]:
		for vi, ti, ni in tri:
			key = (vi, ti, ni)
			if key not in weld:
				weld[key] = len(weld)
				px, py, pz = src_p[vi]
				positions.extend((px, py, pz))
				u, v = src_t[ti]
				uvs.extend((u, 1.0 - v))
				if ni is None:
					normals.extend((0.0, 1.0, 0.0))
				else:
					nx, ny, nz = src_n[ni]
					normals.extend((nx, ny, nz))
			indices.append(weld[key])
	nvert = len(weld)
	index_fmt = "<" + "H" * len(indices) if nvert < 65536 else "<" + "I" * len(indices)
	index_bytes = struct.pack(index_fmt, *indices)
	if len(index_bytes) % 4:
		index_bytes += b"\x00" * (4 - (len(index_bytes) % 4))
	blob = (
		struct.pack("<" + "f" * len(positions), *positions)
		+ struct.pack("<" + "f" * len(normals), *normals)
		+ struct.pack("<" + "f" * len(uvs), *uvs)
		+ index_bytes
	)
	xs = positions[0::3]
	ys = positions[1::3]
	zs = positions[2::3]
	return blob, [min(xs), min(ys), min(zs)], [max(xs), max(ys), max(zs)], nvert


def build_glb(objects: dict[str, dict]) -> bytes:
	bin_blob = bytearray()
	buffer_views = []
	accessors = []
	meshes = []
	nodes = []
	materials = []
	scenes = []

	def push_view(data: bytes, target: int | None) -> int:
		start = len(bin_blob)
		bin_blob.extend(data)
		view = {"buffer": 0, "byteOffset": start, "byteLength": len(data)}
		if target is not None:
			view["target"] = target
		buffer_views.append(view)
		return len(buffer_views) - 1

	for name in SCENE_ORDER:
		obj = objects[name]
		blob, pos_min, pos_max, nvert = mesh_buffers(obj)
		nidx = sum(len(tri) for tri in obj["tris"])
		pos_len = nvert * 12
		nrm_len = nvert * 12
		uv_len = nvert * 8
		idx_unpadded = nidx * (2 if nvert < 65536 else 4)
		pos_view = push_view(blob[0:pos_len], 34962)
		nrm_view = push_view(blob[pos_len : pos_len + nrm_len], 34962)
		uv_view = push_view(blob[pos_len + nrm_len : pos_len + nrm_len + uv_len], 34962)
		idx_view = push_view(blob[pos_len + nrm_len + uv_len :], 34963)
		base = len(accessors)
		accessors.append(
			{
				"bufferView": pos_view,
				"componentType": 5126,
				"count": nvert,
				"type": "VEC3",
				"min": pos_min,
				"max": pos_max,
			}
		)
		accessors.append(
			{"bufferView": nrm_view, "componentType": 5126, "count": nvert, "type": "VEC3"}
		)
		accessors.append(
			{"bufferView": uv_view, "componentType": 5126, "count": nvert, "type": "VEC2"}
		)
		index_type = 5123 if nvert < 65536 else 5125
		accessors.append(
			{
				"bufferView": idx_view,
				"byteOffset": 0,
				"componentType": index_type,
				"count": nidx,
				"type": "SCALAR",
			}
		)
		mat_index = len(materials)
		materials.append(
			{
				"name": f"{name}_mat",
				"pbrMetallicRoughness": {
					"baseColorFactor": [1.0, 1.0, 1.0, 1.0],
					"metallicFactor": 0.0,
					"roughnessFactor": 1.0,
				},
				"doubleSided": True,
			}
		)
		meshes.append(
			{
				"name": name,
				"primitives": [
					{
						"attributes": {
							"POSITION": base,
							"NORMAL": base + 1,
							"TEXCOORD_0": base + 2,
						},
						"indices": base + 3,
						"material": mat_index,
					}
				],
			}
		)
		node_index = len(nodes)
		nodes.append({"name": name, "mesh": len(meshes) - 1})
		scenes.append({"name": name, "nodes": [node_index]})

	# The BIN chunk writer already padded each view's payload; keep buffer length aligned.
	while len(bin_blob) % 4:
		bin_blob.append(0)

	gltf = {
		"asset": {"version": "2.0", "generator": "epiphant obj_to_glb.py (format conversion)"},
		"scene": 0,
		"scenes": scenes,
		"nodes": nodes,
		"meshes": meshes,
		"materials": materials,
		"accessors": accessors,
		"bufferViews": buffer_views,
		"buffers": [{"byteLength": len(bin_blob)}],
	}
	json_bytes = pad4(json.dumps(gltf, separators=(",", ":")).encode("utf-8"), b" ")
	bin_bytes = pad4(bytes(bin_blob), b"\x00")
	total = 12 + 8 + len(json_bytes) + 8 + len(bin_bytes)
	header = struct.pack("<4sII", b"glTF", 2, total)
	json_chunk = struct.pack("<I4s", len(json_bytes), b"JSON") + json_bytes
	bin_chunk = struct.pack("<I4s", len(bin_bytes), b"BIN\x00") + bin_bytes
	return header + json_chunk + bin_chunk


def main(argv: list[str]) -> int:
	obj_path = Path(argv[1]) if len(argv) > 1 else DEFAULT_OBJ
	glb_path = Path(argv[2]) if len(argv) > 2 else DEFAULT_GLB
	objects = parse_obj(obj_path.read_text())
	glb = build_glb(objects)
	glb_path.parent.mkdir(parents=True, exist_ok=True)
	glb_path.write_bytes(glb)
	print(f"wrote {glb_path} ({len(glb)} bytes, {len(SCENE_ORDER)} scenes)")
	return 0


if __name__ == "__main__":
	sys.exit(main(sys.argv))
