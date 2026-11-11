//! Export-time checks: finite values, UV coverage, collapsed UVs, overlaps, materials, textures.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use bevy::math::{Vec2, Vec3};

use crate::export::MeshGroup;
use crate::obj_text::{ObjFile, ObjValidationError};
use crate::texture::{SolidPng, TextureConfig};
use crate::uv::{aabb_extent, Triangle2, Triangle3, UvIsland};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportCheck {
	NonFinite { kind: &'static str, group: String },
	MissingUv { group: String, face: usize },
	CollapsedUv { group: String, face: usize },
	UvOverlap { group: String, a: usize, b: usize },
	IslandOutOfBounds { group: String },
	MissingMtllib,
	MissingUsemtl { group: String },
	DuplicateMaterial { name: String },
	DuplicateColorImage { path: String },
	MissingCompanionMtl { path: String },
	MissingTexture { material: String, path: String },
	WrongTextureSize { path: String, width: u32, height: u32 },
	WrongTextureColor { path: String },
	NonWhiteKd { material: String },
	Obj(ObjValidationError),
}

impl std::fmt::Display for ExportCheck {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NonFinite { kind, group } => write!(f, "{group}: non-finite {kind}"),
			Self::MissingUv { group, face } => write!(f, "{group}: face {face} missing UV"),
			Self::CollapsedUv { group, face } => {
				write!(f, "{group}: face {face} has collapsed UVs on nondegenerate geometry")
			}
			Self::UvOverlap { group, a, b } => {
				write!(f, "{group}: faces {a} and {b} overlap in UV")
			}
			Self::IslandOutOfBounds { group } => write!(f, "{group}: UVs leave 0..1"),
			Self::MissingMtllib => write!(f, "OBJ is missing mtllib"),
			Self::MissingUsemtl { group } => write!(f, "object {group} has faces but no usemtl"),
			Self::DuplicateMaterial { name } => {
				write!(f, "material {name} is assigned to more than one object")
			}
			Self::DuplicateColorImage { path } => {
				write!(f, "color image {path} is shared by more than one material")
			}
			Self::MissingCompanionMtl { path } => write!(f, "companion MTL is missing: {path}"),
			Self::MissingTexture { material, path } => {
				write!(f, "material {material} references missing texture {path}")
			}
			Self::WrongTextureSize { path, width, height } => {
				write!(f, "texture {path} is {width}x{height}, expected a square export size")
			}
			Self::WrongTextureColor { path } => {
				write!(f, "texture {path} is not the intended solid base color")
			}
			Self::NonWhiteKd { material } => {
				write!(f, "material {material} Kd must be white so the PNG supplies the color")
			}
			Self::Obj(error) => write!(f, "{error}"),
		}
	}
}

#[derive(Debug, Default)]
pub struct MeshAudit {
	pub errors: Vec<ExportCheck>,
}

impl MeshAudit {
	pub fn groups(groups: &[MeshGroup]) -> Self {
		let mut errors = Vec::new();
		for group in groups {
			errors.extend(Self::group(group));
		}
		Self { errors }
	}

	pub fn group(group: &MeshGroup) -> Vec<ExportCheck> {
		let mut errors = Vec::new();
		for p in &group.positions {
			if !p.is_finite() {
				errors.push(ExportCheck::NonFinite { kind: "position", group: group.name.clone() });
				break;
			}
		}
		for n in &group.normals {
			if !n.is_finite() {
				errors.push(ExportCheck::NonFinite { kind: "normal", group: group.name.clone() });
				break;
			}
		}
		for uv in &group.uvs {
			if !uv.is_finite() {
				errors.push(ExportCheck::NonFinite { kind: "uv", group: group.name.clone() });
				break;
			}
		}

		if group.uvs.len() != group.positions.len() {
			for i in 0..group.faces.len() {
				errors.push(ExportCheck::MissingUv { group: group.name.clone(), face: i });
			}
			return errors;
		}

		if let Some(island) = UvIsland::from_uvs(&group.uvs) {
			if !island.in_unit_square() {
				errors.push(ExportCheck::IslandOutOfBounds { group: group.name.clone() });
			}
		}

		let pos_scale = aabb_extent(&group.positions);
		let mut uv_faces: Vec<(usize, Triangle2)> = Vec::new();
		let mut densities = Vec::new();
		let mut inspected: Vec<(usize, Triangle3, Triangle2)> = Vec::new();
		for (i, face) in group.faces.iter().enumerate() {
			let Some(geom) = Triangle3::from_indices(&group.positions, *face) else {
				errors.push(ExportCheck::MissingUv { group: group.name.clone(), face: i });
				continue;
			};
			let Some(uv) = Triangle2::from_indices(&group.uvs, *face) else {
				errors.push(ExportCheck::MissingUv { group: group.name.clone(), face: i });
				continue;
			};
			if geom.is_degenerate(pos_scale) {
				continue;
			}
			let geom_area = geom.area().max(1e-20);
			densities.push(uv.area() / geom_area);
			inspected.push((i, geom, uv));
			uv_faces.push((i, uv));
		}
		densities.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
		let median_density = densities.get(densities.len() / 2).copied().unwrap_or(1.0);
		for (i, geom, uv) in inspected {
			if uv.is_projection_collapsed(geom, median_density) {
				errors.push(ExportCheck::CollapsedUv { group: group.name.clone(), face: i });
			}
		}

		for (a_i, (ia, a)) in uv_faces.iter().enumerate() {
			for (ib, b) in uv_faces.iter().skip(a_i + 1) {
				if share_mesh_edge(group.faces[*ia], group.faces[*ib]) {
					continue;
				}
				if a.overlaps_interior(*b) {
					errors.push(ExportCheck::UvOverlap {
						group: group.name.clone(),
						a: *ia,
						b: *ib,
					});
				}
			}
		}
		errors
	}

	pub fn unique_assignments(groups: &[MeshGroup]) -> Self {
		let mut errors = Vec::new();
		let mut materials = HashSet::new();
		let mut images = HashSet::new();
		for group in groups {
			let mat = group.material_name();
			if !materials.insert(mat.clone()) {
				errors.push(ExportCheck::DuplicateMaterial { name: mat });
			}
			let image = group.color_file();
			if !images.insert(image.clone()) {
				errors.push(ExportCheck::DuplicateColorImage { path: image });
			}
		}
		Self { errors }
	}

	pub fn obj_text(text: &str) -> Self {
		let mut errors = Vec::new();
		for (kind, token) in scan_numeric_tokens(text) {
			if let Ok(value) = token.parse::<f32>() {
				if !value.is_finite() {
					errors.push(ExportCheck::NonFinite { kind, group: "obj".into() });
					break;
				}
			}
		}
		let file = ObjFile::parse(text);
		if file.mtllib.is_none() {
			errors.push(ExportCheck::MissingMtllib);
		}
		let mut seen_materials = HashSet::new();
		for object in &file.objects {
			if object.faces > 0 && object.material.is_none() {
				errors.push(ExportCheck::MissingUsemtl { group: object.name.clone() });
			}
			if let Some(name) = &object.material {
				if !seen_materials.insert(name.clone()) {
					errors.push(ExportCheck::DuplicateMaterial { name: name.clone() });
				}
			}
		}
		errors.extend(file.validation_errors().into_iter().map(ExportCheck::Obj));
		Self { errors }
	}

	pub fn companion_files(obj_path: &Path) -> Self {
		let mut errors = Vec::new();
		let mtl = obj_path.with_extension("mtl");
		if !mtl.exists() {
			errors.push(ExportCheck::MissingCompanionMtl { path: mtl.display().to_string() });
		}
		Self { errors }
	}

	pub fn mtl_textures(mtl_text: &str, mtl_dir: &Path, config: TextureConfig) -> Self {
		let table = MtlTable::parse(mtl_text);
		table.audit(mtl_dir, config)
	}

	pub fn ok(&self) -> bool {
		self.errors.is_empty()
	}
}

#[derive(Debug, Clone)]
pub struct MtlRecord {
	pub name: String,
	pub kd: [f32; 3],
	pub map_kd: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MtlTable {
	pub materials: Vec<MtlRecord>,
}

impl MtlTable {
	pub fn parse(text: &str) -> Self {
		let mut materials = Vec::new();
		let mut current: Option<usize> = None;
		for line in text.lines() {
			let line = line.trim();
			if let Some(name) = line.strip_prefix("newmtl ") {
				materials.push(MtlRecord {
					name: name.trim().to_string(),
					kd: [1.0, 1.0, 1.0],
					map_kd: None,
				});
				current = Some(materials.len() - 1);
				continue;
			}
			let Some(i) = current else {
				continue;
			};
			if let Some(rest) = line.strip_prefix("Kd ") {
				let mut parts = rest.split_whitespace();
				let r = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
				let g = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
				let b = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
				materials[i].kd = [r, g, b];
			} else if let Some(path) = line.strip_prefix("map_Kd ") {
				let path = path.split_whitespace().next().unwrap_or("").trim();
				if !path.is_empty() {
					materials[i].map_kd = Some(path.to_string());
				}
			}
		}
		Self { materials }
	}

	pub fn audit(&self, mtl_dir: &Path, config: TextureConfig) -> MeshAudit {
		let mut errors = Vec::new();
		let mut names = HashSet::new();
		let mut images = HashSet::new();
		for record in &self.materials {
			if !names.insert(record.name.clone()) {
				errors.push(ExportCheck::DuplicateMaterial { name: record.name.clone() });
			}
			if record.kd.iter().any(|c| (*c - 1.0).abs() > 1e-4) {
				errors.push(ExportCheck::NonWhiteKd { material: record.name.clone() });
			}
			let Some(path) = &record.map_kd else {
				errors.push(ExportCheck::MissingTexture {
					material: record.name.clone(),
					path: "<missing map_Kd>".into(),
				});
				continue;
			};
			if !images.insert(path.clone()) {
				errors.push(ExportCheck::DuplicateColorImage { path: path.clone() });
			}
			let file = mtl_dir.join(path);
			if !file.is_file() {
				errors.push(ExportCheck::MissingTexture {
					material: record.name.clone(),
					path: path.clone(),
				});
				continue;
			}
			match SolidPng::load(&file) {
				Ok(png) => {
					if png.width != config.resolution || png.height != config.resolution {
						errors.push(ExportCheck::WrongTextureSize {
							path: path.clone(),
							width: png.width,
							height: png.height,
						});
					}
					if !png.uniform {
						errors.push(ExportCheck::WrongTextureColor { path: path.clone() });
					}
				}
				Err(_) => errors.push(ExportCheck::MissingTexture {
					material: record.name.clone(),
					path: path.clone(),
				}),
			}
		}
		MeshAudit { errors }
	}

	pub fn color_files(&self) -> Vec<String> {
		self.materials.iter().filter_map(|m| m.map_kd.clone()).collect()
	}
}

fn share_mesh_edge(a: [u32; 3], b: [u32; 3]) -> bool {
	let mut shared = 0;
	for ia in a {
		if b.contains(&ia) {
			shared += 1;
		}
	}
	shared >= 2
}

fn scan_numeric_tokens(text: &str) -> Vec<(&'static str, &str)> {
	let mut out = Vec::new();
	for line in text.lines() {
		let line = line.trim();
		let kind = if line.starts_with("v ") {
			"position"
		} else if line.starts_with("vt ") {
			"uv"
		} else if line.starts_with("vn ") {
			"normal"
		} else {
			continue;
		};
		for token in line.split_whitespace().skip(1) {
			out.push((kind, token));
		}
	}
	out
}

/// Reconstruct per-object UV islands from OBJ text (after `vt` V-flip is already applied).
pub fn obj_uvs_by_object(text: &str) -> HashMap<String, Vec<Vec2>> {
	let mut current = None::<String>;
	let mut by_obj: HashMap<String, Vec<Vec2>> = HashMap::new();
	for line in text.lines() {
		let line = line.trim();
		if let Some(name) = line.strip_prefix("o ").or_else(|| line.strip_prefix("g ")) {
			current = Some(name.trim().to_string());
		} else if let Some(rest) = line.strip_prefix("vt ") {
			let mut parts = rest.split_whitespace();
			let Ok(u) = parts.next().unwrap_or("").parse::<f32>() else {
				continue;
			};
			let Ok(v) = parts.next().unwrap_or("").parse::<f32>() else {
				continue;
			};
			if let Some(name) = &current {
				by_obj.entry(name.clone()).or_default().push(Vec2::new(u, v));
			}
		}
	}
	by_obj
}

pub fn positions_from_obj_object(text: &str, object: &str) -> Vec<Vec3> {
	let mut current = None::<String>;
	let mut positions = Vec::new();
	for line in text.lines() {
		let line = line.trim();
		if let Some(name) = line.strip_prefix("o ").or_else(|| line.strip_prefix("g ")) {
			current = Some(name.trim().to_string());
		} else if line.starts_with("v ") && current.as_deref() == Some(object) {
			let mut parts = line.split_whitespace().skip(1);
			let Ok(x) = parts.next().unwrap_or("").parse::<f32>() else {
				continue;
			};
			let Ok(y) = parts.next().unwrap_or("").parse::<f32>() else {
				continue;
			};
			let Ok(z) = parts.next().unwrap_or("").parse::<f32>() else {
				continue;
			};
			positions.push(Vec3::new(x, y, z));
		}
	}
	positions
}

#[cfg(test)]
mod tests {
	use super::*;

	fn tri_group(name: &str, uvs: [Vec2; 3]) -> MeshGroup {
		MeshGroup {
			name: name.into(),
			diffuse: [0.5, 0.5, 0.5],
			positions: vec![Vec3::ZERO, Vec3::X, Vec3::Y],
			normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
			uvs: uvs.to_vec(),
			faces: vec![[0, 1, 2]],
		}
	}

	#[test]
	fn collapsed_uv_on_real_geometry_fails() -> anyhow::Result<()> {
		let group = tri_group("head", [Vec2::ZERO, Vec2::ZERO, Vec2::ZERO]);
		let errors = MeshAudit::group(&group);
		if !errors
			.iter()
			.any(|e| matches!(e, ExportCheck::CollapsedUv { group, .. } if group == "head"))
		{
			return Err(anyhow::anyhow!("expected collapsed UV, got {errors:?}"));
		}
		Ok(())
	}

	#[test]
	fn overlapping_uv_faces_fail() -> anyhow::Result<()> {
		let group = MeshGroup {
			name: "body".into(),
			diffuse: [0.5, 0.5, 0.5],
			positions: vec![
				Vec3::ZERO,
				Vec3::X,
				Vec3::Y,
				Vec3::new(2.0, 0.0, 0.0),
				Vec3::new(3.0, 0.0, 0.0),
				Vec3::new(2.0, 1.0, 0.0),
			],
			normals: vec![Vec3::Z; 6],
			uvs: vec![
				Vec2::new(0.1, 0.1),
				Vec2::new(0.9, 0.1),
				Vec2::new(0.1, 0.9),
				Vec2::new(0.2, 0.2),
				Vec2::new(0.8, 0.2),
				Vec2::new(0.2, 0.8),
			],
			faces: vec![[0, 1, 2], [3, 4, 5]],
		};
		let errors = MeshAudit::group(&group);
		if !errors
			.iter()
			.any(|e| matches!(e, ExportCheck::UvOverlap { group, .. } if group == "body"))
		{
			return Err(anyhow::anyhow!("expected UV overlap, got {errors:?}"));
		}
		Ok(())
	}

	#[test]
	fn identical_uvs_on_separate_components_are_allowed() -> anyhow::Result<()> {
		let uvs = [Vec2::new(0.1, 0.1), Vec2::new(0.9, 0.1), Vec2::new(0.1, 0.9)];
		let groups = [tri_group("head", uvs), tri_group("ear_left", uvs)];
		let audit = MeshAudit::groups(&groups);
		if audit.errors.iter().any(|e| matches!(e, ExportCheck::UvOverlap { .. })) {
			return Err(anyhow::anyhow!(
				"cross-component UV reuse must be allowed: {:?}",
				audit.errors
			));
		}
		Ok(())
	}

	#[test]
	fn missing_mtllib_and_usemtl_fail() -> anyhow::Result<()> {
		let text = "o body\nv 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvn 0 0 1\nvn 0 0 1\nvn 0 0 1\nf 1/1/1 2/2/2 3/3/3\n";
		let audit = MeshAudit::obj_text(text);
		if !audit.errors.iter().any(|e| matches!(e, ExportCheck::MissingMtllib)) {
			return Err(anyhow::anyhow!("expected missing mtllib, got {:?}", audit.errors));
		}
		if !audit
			.errors
			.iter()
			.any(|e| matches!(e, ExportCheck::MissingUsemtl { group } if group == "body"))
		{
			return Err(anyhow::anyhow!("expected missing usemtl, got {:?}", audit.errors));
		}
		Ok(())
	}

	#[test]
	fn shared_material_or_image_fails() -> anyhow::Result<()> {
		let groups = [
			tri_group("head", [Vec2::X, Vec2::Y, Vec2::ZERO]),
			tri_group("head", [Vec2::X, Vec2::Y, Vec2::ZERO]),
		];
		let audit = MeshAudit::unique_assignments(&groups);
		if !audit.errors.iter().any(|e| matches!(e, ExportCheck::DuplicateMaterial { .. })) {
			return Err(anyhow::anyhow!("expected duplicate material, got {:?}", audit.errors));
		}
		Ok(())
	}

	#[test]
	fn missing_companion_mtl_is_reported() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let obj = dir.path().join("mesh.obj");
		std::fs::write(&obj, "o body\n")?;
		let audit = MeshAudit::companion_files(&obj);
		if !audit
			.errors
			.iter()
			.any(|e| matches!(e, ExportCheck::MissingCompanionMtl { .. }))
		{
			return Err(anyhow::anyhow!("expected missing MTL, got {:?}", audit.errors));
		}
		Ok(())
	}

	#[test]
	fn missing_map_kd_file_is_reported() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let mtl = "newmtl body_mat\nKd 1 1 1\nd 1\nmap_Kd missing.png\n";
		let audit = MeshAudit::mtl_textures(mtl, dir.path(), TextureConfig::default());
		if !audit
			.errors
			.iter()
			.any(|e| matches!(e, ExportCheck::MissingTexture { path, .. } if path == "missing.png"))
		{
			return Err(anyhow::anyhow!("expected missing texture, got {:?}", audit.errors));
		}
		Ok(())
	}

	#[test]
	fn non_white_kd_is_reported() -> anyhow::Result<()> {
		let mtl = "newmtl body_mat\nKd 0.2 0.3 0.4\nd 1.0\nmap_Kd body_color.png\n";
		let dir = tempfile::tempdir()?;
		let audit = MeshAudit::mtl_textures(mtl, dir.path(), TextureConfig::default());
		if !audit
			.errors
			.iter()
			.any(|e| matches!(e, ExportCheck::NonWhiteKd { material } if material == "body_mat"))
		{
			return Err(anyhow::anyhow!("expected non-white Kd, got {:?}", audit.errors));
		}
		Ok(())
	}
}
