//! Quantize, weld, and UV-fill assembled mesh groups for a smaller Procreate-safe OBJ.

use std::collections::HashMap;

use bevy::math::{Vec2, Vec3};

use crate::export::MeshGroup;

pub const FLOAT_DECIMALS: i32 = 5;

pub fn prepare_group_for_obj(mut group: MeshGroup) -> MeshGroup {
	if group.uvs.len() != group.positions.len() {
		group.uvs = planar_uvs(&group.positions);
	}
	if group.normals.len() != group.positions.len() {
		group.normals = flat_normals(&group.positions, &group.faces);
	}
	weld_group(group)
}

/// Body first as `o root`; remaining parts keep their names as separate objects.
pub fn order_parts_for_obj(mut groups: Vec<MeshGroup>) -> Vec<MeshGroup> {
	groups.sort_by(|a, b| {
		part_rank(&a.name)
			.cmp(&part_rank(&b.name))
			.then_with(|| a.name.cmp(&b.name))
	});
	if let Some(body) = groups.iter_mut().find(|g| is_body_part(&g.name)) {
		body.name = "root".into();
	}
	groups
}

fn is_body_part(name: &str) -> bool {
	name == "root" || name == "body" || name.starts_with("body__")
}

fn part_rank(name: &str) -> u8 {
	if is_body_part(name) { 0 } else { 1 }
}

pub fn planar_uvs(positions: &[Vec3]) -> Vec<Vec2> {
	if positions.is_empty() {
		return Vec::new();
	}
	let mut min = Vec3::splat(f32::MAX);
	let mut max = Vec3::splat(f32::MIN);
	for p in positions {
		min = min.min(*p);
		max = max.max(*p);
	}
	let ext = (max - min).max(Vec3::splat(1e-6));
	let mut axes = [0usize, 1, 2];
	axes.sort_by(|a, b| ext[*b].partial_cmp(&ext[*a]).unwrap_or(std::cmp::Ordering::Equal));
	positions
		.iter()
		.map(|p| {
			Vec2::new(
				((p[axes[0]] - min[axes[0]]) / ext[axes[0]]).clamp(0.0, 1.0),
				((p[axes[1]] - min[axes[1]]) / ext[axes[1]]).clamp(0.0, 1.0),
			)
		})
		.collect()
}

fn flat_normals(positions: &[Vec3], faces: &[[u32; 3]]) -> Vec<Vec3> {
	let mut normals = vec![Vec3::ZERO; positions.len()];
	for [a, b, c] in faces {
		let (ia, ib, ic) = (*a as usize, *b as usize, *c as usize);
		let (Some(&pa), Some(&pb), Some(&pc)) =
			(positions.get(ia), positions.get(ib), positions.get(ic))
		else {
			continue;
		};
		let n = (pb - pa).cross(pc - pa);
		normals[ia] += n;
		normals[ib] += n;
		normals[ic] += n;
	}
	for n in &mut normals {
		*n = if n.length_squared() > 1e-12 { n.normalize() } else { Vec3::Y };
	}
	normals
}

fn weld_group(group: MeshGroup) -> MeshGroup {
	// Same quantized position+UV is a duplicate corner (skinned GLBs repeat verts).
	// Normals are averaged so float-noise splits do not block the weld.
	let mut map: HashMap<[i32; 5], u32> = HashMap::new();
	let mut positions = Vec::new();
	let mut normals = Vec::new();
	let mut uvs = Vec::new();
	let mut remap = vec![0u32; group.positions.len()];
	for i in 0..group.positions.len() {
		let p = group.positions[i];
		let n = group.normals.get(i).copied().unwrap_or(Vec3::Y);
		let uv = group.uvs.get(i).copied().unwrap_or(Vec2::ZERO);
		let key = [qkey(p.x), qkey(p.y), qkey(p.z), qkey(uv.x), qkey(uv.y)];
		if let Some(&idx) = map.get(&key) {
			normals[idx as usize] += n;
			remap[i] = idx;
		} else {
			let id = positions.len() as u32;
			map.insert(key, id);
			positions.push(quantize_vec3(p));
			normals.push(n);
			uvs.push(quantize_vec2(uv));
			remap[i] = id;
		}
	}
	for n in &mut normals {
		*n = if n.length_squared() > 1e-12 {
			quantize_vec3(n.normalize())
		} else {
			Vec3::Y
		};
	}
	let mut seen_faces = std::collections::HashSet::new();
	let faces = group
		.faces
		.iter()
		.filter_map(|[a, b, c]| {
			let (ia, ib, ic) = (
				*remap.get(*a as usize)?,
				*remap.get(*b as usize)?,
				*remap.get(*c as usize)?,
			);
			if ia == ib || ib == ic || ia == ic {
				return None;
			}
			let key = {
				let mut t = [ia, ib, ic];
				t.sort_unstable();
				t
			};
			if seen_faces.insert(key) {
				Some([ia, ib, ic])
			} else {
				None
			}
		})
		.collect();
	MeshGroup {
		name: group.name,
		material_name: group.material_name,
		diffuse: group.diffuse,
		positions,
		normals,
		uvs,
		faces,
	}
}

fn qkey(x: f32) -> i32 {
	let scale = 10f32.powi(FLOAT_DECIMALS);
	(x * scale).round() as i32
}

fn quantize_vec3(v: Vec3) -> Vec3 {
	Vec3::new(quantize(v.x), quantize(v.y), quantize(v.z))
}

fn quantize_vec2(v: Vec2) -> Vec2 {
	Vec2::new(quantize(v.x), quantize(v.y))
}

fn quantize(x: f32) -> f32 {
	let scale = 10f32.powi(FLOAT_DECIMALS);
	(x * scale).round() / scale
}

pub fn fmt_f32(x: f32) -> String {
	format!("{:.prec$}", quantize(x), prec = FLOAT_DECIMALS as usize)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn body_is_first_object_named_root() {
		let ordered = order_parts_for_obj(vec![
			MeshGroup {
				name: "head__meerkat".into(),
				material_name: "h".into(),
				diffuse: [1.0; 3],
				positions: vec![Vec3::ZERO],
				normals: vec![Vec3::Y],
				uvs: vec![Vec2::ZERO],
				faces: vec![],
			},
			MeshGroup {
				name: "body__epiphant".into(),
				material_name: "b".into(),
				diffuse: [1.0; 3],
				positions: vec![Vec3::X],
				normals: vec![Vec3::Y],
				uvs: vec![Vec2::ZERO],
				faces: vec![],
			},
		]);
		assert_eq!(ordered[0].name, "root");
		assert_eq!(ordered[1].name, "head__meerkat");
	}

	#[test]
	fn weld_collapses_identical_corners() {
		let group = prepare_group_for_obj(MeshGroup {
			name: "dup".into(),
			material_name: "dup_mat".into(),
			diffuse: [1.0, 1.0, 1.0],
			positions: vec![
				Vec3::ZERO,
				Vec3::ZERO,
				Vec3::X,
				Vec3::X,
				Vec3::Y,
				Vec3::Y,
			],
			normals: vec![Vec3::Z; 6],
			uvs: vec![Vec2::ZERO, Vec2::ZERO, Vec2::X, Vec2::X, Vec2::Y, Vec2::Y],
			faces: vec![[0, 2, 4], [1, 3, 5]],
		});
		assert_eq!(group.positions.len(), 3);
		assert_eq!(group.faces.len(), 1);
	}
}

/// Minimal 1×1 RGB PNG (solid color) so `map_Kd` has a relative file.
pub fn solid_png_rgb(r: u8, g: u8, b: u8) -> Vec<u8> {
	// Prebuilt 1×1 8-bit RGB PNG; IDAT stores one filtered gray-white row we overwrite
	// via a generated uncompressed store block when colors vary.
	fn crc32(data: &[u8]) -> u32 {
		let mut crc = 0xFFFF_FFFFu32;
		for &byte in data {
			crc ^= u32::from(byte);
			for _ in 0..8 {
				crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
			}
		}
		!crc
	}
	fn chunk(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
		let mut out = Vec::new();
		out.extend_from_slice(&(data.len() as u32).to_be_bytes());
		out.extend_from_slice(tag);
		out.extend_from_slice(data);
		let mut crc_src = Vec::new();
		crc_src.extend_from_slice(tag);
		crc_src.extend_from_slice(data);
		out.extend_from_slice(&crc32(&crc_src).to_be_bytes());
		out
	}
	let raw = vec![0u8, r, g, b]; // filter 0 + RGB
	let mut deflate = vec![0x78, 0x01, 0x01, 0x04, 0x00, 0xFB, 0xFF];
	deflate.extend_from_slice(&raw);
	let adler = {
		let mut s1 = 1u32;
		let mut s2 = 0u32;
		for &byte in &raw {
			s1 = (s1 + u32::from(byte)) % 65521;
			s2 = (s2 + s1) % 65521;
		}
		(s2 << 16) | s1
	};
	deflate.extend_from_slice(&adler.to_be_bytes());
	let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
	png.extend(chunk(
		b"IHDR",
		&[0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0],
	));
	png.extend(chunk(b"IDAT", &deflate));
	png.extend(chunk(b"IEND", &[]));
	let _ = raw;
	png
}
