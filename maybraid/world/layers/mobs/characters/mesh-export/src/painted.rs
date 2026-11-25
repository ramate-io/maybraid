//! Re-bind a painted export OBJ onto the live skinned part meshes.
//!
//! [`crate::export`] writes each component CPU-skinned into character root space,
//! then chart-unwrapped (vertices split at chart seams). Painting happens on that
//! static OBJ. To deform with the rig again, every painted triangle is matched back
//! to the triangle of the runtime part mesh it came from (baked corner positions plus
//! normals, so double-sided flaps keep front/back apart). The output keeps the
//! painted UVs and takes position, normal, `JOINTS_0` and `WEIGHTS_0` from the
//! runtime mesh in its own local (bind) space. Swapping that mesh onto the part's
//! [`bevy::mesh::skinning::SkinnedMesh`] entity therefore skins exactly like the
//! original part, with the painted UV layout. No weights are authored here: they
//! are the part GLB's own skin.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use bevy::ecs::entity::Entity;
use bevy::math::{Affine3A, Mat4, Vec2, Vec3};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::{GlobalTransform, Mesh, Mesh3d, Name, World};

use crate::export::{
	extract_mesh_local, mesh_faces, mesh_normals, mesh_positions, part_group_name,
	part_mesh_entities,
};

/// glTF mesh order in the re-bound GLB (`#Mesh{i}/Primitive0`).
pub const PAINTED_MESH_ORDER: [&str; 8] =
	["body", "head", "ear_left", "ear_right", "eye_left", "eye_right", "nose", "tail"];

/// Weight of the normal term against corner distance (metres) when matching faces.
const NORMAL_WEIGHT: f32 = 0.05;
/// A bind whose worst corner is farther than this (metres) is a pose mismatch, not
/// a re-bind: the painted OBJ was baked from a different pose or different meshes.
pub const MAX_CORNER_DISTANCE: f32 = 0.01;
/// Worst allowed painted vs source corner normal agreement.
pub const MIN_NORMAL_DOT: f32 = 0.9;

/// Joint names and inverse bind matrices of a part's [`SkinnedMesh`].
#[derive(Debug, Clone, PartialEq)]
pub struct SourceSkin {
	pub joint_names: Vec<String>,
	pub inverse_bindposes: Vec<Mat4>,
}

/// One runtime part mesh: local bind-space data plus its baked (root-space) surface.
#[derive(Debug, Clone)]
pub struct SourcePart {
	pub name: String,
	pub positions: Vec<Vec3>,
	pub normals: Vec<Vec3>,
	pub joints: Option<Vec<[u16; 4]>>,
	pub weights: Option<Vec<[f32; 4]>>,
	pub skin: Option<SourceSkin>,
	pub baked_positions: Vec<Vec3>,
	pub baked_normals: Vec<Vec3>,
	/// Triangles in the mesh's own winding (local-space indices).
	pub faces: Vec<[u32; 3]>,
}

/// One painted `o` object from the export OBJ.
#[derive(Debug, Clone, Default)]
pub struct PaintedObject {
	pub name: String,
	pub material: Option<String>,
	pub positions: Vec<Vec3>,
	pub normals: Vec<Vec3>,
	/// glTF / image-space UVs (`v` already flipped from OBJ).
	pub uvs: Vec<Vec2>,
	/// Per corner: (v, vt, vn) indices local to this object.
	pub faces: Vec<[[u32; 3]; 3]>,
}

/// Painted mesh in the source part's local space.
#[derive(Debug, Clone)]
pub struct BoundMesh {
	pub name: String,
	pub material: String,
	pub positions: Vec<Vec3>,
	pub normals: Vec<Vec3>,
	pub uvs: Vec<Vec2>,
	pub joints: Option<Vec<[u16; 4]>>,
	pub weights: Option<Vec<[f32; 4]>>,
	pub skin: Option<SourceSkin>,
	pub indices: Vec<u32>,
	pub stats: BindStats,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BindStats {
	pub painted_faces: usize,
	pub source_faces: usize,
	/// Largest painted→source corner distance in root space (metres).
	pub max_corner_distance: f32,
	pub mean_corner_distance: f32,
	/// Smallest corner normal agreement (`dot`) over all matched corners.
	pub min_normal_dot: f32,
	/// Faces that could not take their nearest source triangle (already claimed).
	pub displaced_faces: usize,
}

impl std::fmt::Display for BindStats {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"faces={}/{} max_corner={:.4}m mean_corner={:.4}m min_normal_dot={:.3} displaced={}",
			self.painted_faces,
			self.source_faces,
			self.max_corner_distance,
			self.mean_corner_distance,
			self.min_normal_dot,
			self.displaced_faces
		)
	}
}

/// Parse an export OBJ (`o` per component, `usemtl`, triangulated `f v/vt/vn`).
pub fn parse_painted_obj(text: &str) -> anyhow::Result<Vec<PaintedObject>> {
	let mut positions: Vec<Vec3> = Vec::new();
	let mut uvs: Vec<Vec2> = Vec::new();
	let mut normals: Vec<Vec3> = Vec::new();
	// Global → object-local index maps, per stream (reset at each `o`).
	let mut objects: Vec<PaintedObject> = Vec::new();
	let mut v_map: HashMap<usize, u32> = HashMap::new();
	let mut vt_map: HashMap<usize, u32> = HashMap::new();
	let mut vn_map: HashMap<usize, u32> = HashMap::new();

	fn floats<const N: usize>(rest: &[&str], line: usize) -> anyhow::Result<[f32; N]> {
		let mut out = [0.0; N];
		for (i, slot) in out.iter_mut().enumerate() {
			*slot = rest
				.get(i)
				.ok_or_else(|| anyhow::anyhow!("line {line}: expected {N} numbers"))?
				.parse()?;
		}
		Ok(out)
	}

	for (line_no, raw) in text.lines().enumerate() {
		let line_no = line_no + 1;
		let raw = raw.trim();
		if raw.is_empty() || raw.starts_with('#') {
			continue;
		}
		let mut parts = raw.split_whitespace();
		let Some(kind) = parts.next() else {
			continue;
		};
		let rest: Vec<&str> = parts.collect();
		match kind {
			"o" => {
				let name =
					rest.first().ok_or_else(|| anyhow::anyhow!("line {line_no}: unnamed o"))?;
				objects.push(PaintedObject { name: (*name).to_string(), ..Default::default() });
				v_map.clear();
				vt_map.clear();
				vn_map.clear();
			}
			"v" => positions.push(Vec3::from_array(floats::<3>(&rest, line_no)?)),
			"vt" => {
				let [u, v] = floats::<2>(&rest, line_no)?;
				uvs.push(Vec2::new(u, 1.0 - v));
			}
			"vn" => normals.push(Vec3::from_array(floats::<3>(&rest, line_no)?)),
			"usemtl" => {
				let object = objects
					.last_mut()
					.ok_or_else(|| anyhow::anyhow!("line {line_no}: usemtl before o"))?;
				object.material = rest.first().map(|s| (*s).to_string());
			}
			"f" => {
				let object = objects
					.last_mut()
					.ok_or_else(|| anyhow::anyhow!("line {line_no}: face before o"))?;
				if rest.len() != 3 {
					return Err(anyhow::anyhow!("line {line_no}: only triangles are supported"));
				}
				let mut face = [[0u32; 3]; 3];
				for (corner, token) in rest.iter().enumerate() {
					let mut ids = token.split('/');
					let mut next = |what: &str| -> anyhow::Result<usize> {
						let raw = ids
							.next()
							.filter(|s| !s.is_empty())
							.ok_or_else(|| anyhow::anyhow!("line {line_no}: missing {what}"))?;
						let index: usize = raw.parse()?;
						index
							.checked_sub(1)
							.ok_or_else(|| anyhow::anyhow!("line {line_no}: 0 index"))
					};
					let (v, vt, vn) = (next("v")?, next("vt")?, next("vn")?);
					face[corner] = [
						local_index(&mut v_map, v, positions.len(), line_no)?,
						local_index(&mut vt_map, vt, uvs.len(), line_no)?,
						local_index(&mut vn_map, vn, normals.len(), line_no)?,
					];
					if face[corner][0] as usize == object.positions.len() {
						object.positions.push(positions[v]);
					}
					if face[corner][1] as usize == object.uvs.len() {
						object.uvs.push(uvs[vt]);
					}
					if face[corner][2] as usize == object.normals.len() {
						object.normals.push(normals[vn].normalize_or(Vec3::Y));
					}
				}
				object.faces.push(face);
			}
			_ => {}
		}
	}
	Ok(objects)
}

fn local_index(
	map: &mut HashMap<usize, u32>,
	global: usize,
	len: usize,
	line_no: usize,
) -> anyhow::Result<u32> {
	if global >= len {
		return Err(anyhow::anyhow!("line {line_no}: index {} out of range", global + 1));
	}
	let next_id = map.len() as u32;
	Ok(*map.entry(global).or_insert(next_id))
}

/// Collect the primary runtime mesh of every part under `root`.
pub fn collect_source_parts(world: &mut World, root: Entity) -> anyhow::Result<Vec<SourcePart>> {
	let globals: Vec<(Entity, GlobalTransform)> = world
		.query::<(Entity, &GlobalTransform)>()
		.iter(world)
		.map(|(e, g)| (e, *g))
		.collect();
	let global_refs: Vec<(Entity, &GlobalTransform)> =
		globals.iter().map(|(e, g)| (*e, g)).collect();
	let root_inverse = world
		.get::<GlobalTransform>(root)
		.map(|t| t.affine().inverse())
		.unwrap_or(Affine3A::IDENTITY);

	let mut parts: BTreeMap<String, SourcePart> = BTreeMap::new();
	for entity in part_mesh_entities(world, root) {
		let Some(mesh_3d) = world.get::<Mesh3d>(entity) else {
			continue;
		};
		let Some(mesh) = world.resource::<bevy::asset::Assets<Mesh>>().get(&mesh_3d.0) else {
			continue;
		};
		let name = part_group_name(world, entity);
		let Some((baked_positions, baked_normals, _uvs, _aligned)) =
			extract_mesh_local(mesh, entity, world, &global_refs, root_inverse)
		else {
			continue;
		};
		let positions = mesh_positions(mesh);
		let mut normals = mesh_normals(mesh);
		if normals.len() != positions.len() {
			normals = vec![Vec3::Y; positions.len()];
		}
		let joints = match mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX) {
			Some(VertexAttributeValues::Uint16x4(j)) => Some(j.clone()),
			_ => None,
		};
		let weights = match mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT) {
			Some(VertexAttributeValues::Float32x4(w)) => Some(w.clone()),
			_ => None,
		};
		let skin = match world.get::<SkinnedMesh>(entity) {
			Some(skinned) => {
				let bindposes = world
					.resource::<bevy::asset::Assets<SkinnedMeshInverseBindposes>>()
					.get(&skinned.inverse_bindposes)
					.ok_or_else(|| anyhow::anyhow!("{name}: inverse bind poses not loaded"))?;
				let joint_names = skinned
					.joints
					.iter()
					.map(|joint| {
						world
							.get::<Name>(*joint)
							.map(|n| n.as_str().to_string())
							.unwrap_or_default()
					})
					.collect();
				Some(SourceSkin { joint_names, inverse_bindposes: bindposes.to_vec() })
			}
			None => None,
		};
		if skin.is_some() != joints.is_some() {
			return Err(anyhow::anyhow!("{name}: SkinnedMesh and JOINTS_0 disagree"));
		}
		let part = SourcePart {
			name: name.clone(),
			skin,
			positions,
			normals,
			joints,
			weights,
			baked_normals: if baked_normals.len() == baked_positions.len() {
				baked_normals
			} else {
				vec![Vec3::Y; baked_positions.len()]
			},
			baked_positions,
			faces: mesh_faces(mesh),
		};
		if parts.insert(name.clone(), part).is_some() {
			return Err(anyhow::anyhow!("component {name} has more than one primary mesh"));
		}
	}
	Ok(parts.into_values().collect())
}

/// Match every painted triangle to a distinct source triangle and rebuild the mesh.
pub fn bind_painted(source: &SourcePart, painted: &PaintedObject) -> anyhow::Result<BoundMesh> {
	if source.joints.is_some() != source.weights.is_some() {
		return Err(anyhow::anyhow!("{}: joints without weights", source.name));
	}
	let n_src = source.faces.len();
	if painted.faces.len() > n_src {
		return Err(anyhow::anyhow!(
			"{}: painted has {} faces, source only {}",
			source.name,
			painted.faces.len(),
			n_src
		));
	}

	// cost[p] = sorted candidate list (cost, source face, permutation)
	let perms: [[usize; 3]; 6] = [[0, 1, 2], [1, 2, 0], [2, 0, 1], [0, 2, 1], [2, 1, 0], [1, 0, 2]];
	let corner = |face: &[u32; 3], k: usize| -> (Vec3, Vec3) {
		let i = face[k] as usize;
		(source.baked_positions[i], source.baked_normals[i])
	};
	let painted_corners = |pface: &[[u32; 3]; 3]| -> [(Vec3, Vec3); 3] {
		[0, 1, 2].map(|k| {
			(painted.positions[pface[k][0] as usize], painted.normals[pface[k][2] as usize])
		})
	};
	// Cheapest corner permutation of source face `s_index` for painted corners `p`.
	let face_cost = |p: &[(Vec3, Vec3); 3], s_index: usize| -> (f32, usize) {
		let sface = &source.faces[s_index];
		let s = [corner(sface, 0), corner(sface, 1), corner(sface, 2)];
		let mut best = (f32::MAX, 0);
		for (perm_index, perm) in perms.iter().enumerate() {
			let mut cost = 0.0;
			for k in 0..3 {
				let (sp, sn) = s[perm[k]];
				cost += p[k].0.distance(sp) + NORMAL_WEIGHT * (1.0 - p[k].1.dot(sn));
			}
			if cost < best.0 {
				best = (cost, perm_index);
			}
		}
		best
	};
	const SHORTLIST: usize = 16;
	let mut candidates: Vec<Vec<(f32, usize, usize)>> = Vec::with_capacity(painted.faces.len());
	for pface in &painted.faces {
		let p = painted_corners(pface);
		let mut best: Vec<(f32, usize, usize)> = (0..n_src)
			.map(|s_index| {
				let (cost, perm) = face_cost(&p, s_index);
				(cost, s_index, perm)
			})
			.collect();
		best.sort_by(|a, b| a.0.total_cmp(&b.0));
		best.truncate(SHORTLIST);
		candidates.push(best);
	}

	// Greedy global assignment, cheapest first; each source triangle used once.
	let mut order: Vec<usize> = (0..painted.faces.len()).collect();
	order.sort_by(|&a, &b| candidates[a][0].0.total_cmp(&candidates[b][0].0));
	let mut claimed = vec![false; n_src];
	let mut assignment: Vec<Option<(usize, usize)>> = vec![None; painted.faces.len()];
	let mut displaced = 0;
	for &p in &order {
		let pick = candidates[p]
			.iter()
			.find(|(_, s, _)| !claimed[*s])
			.map(|&(_, s, perm)| (s, perm));
		let pick = pick.or_else(|| {
			// Shortlist exhausted: cheapest unclaimed source face.
			let corners = painted_corners(&painted.faces[p]);
			(0..n_src)
				.filter(|s| !claimed[*s])
				.map(|s| {
					let (cost, perm) = face_cost(&corners, s);
					(cost, s, perm)
				})
				.min_by(|a, b| a.0.total_cmp(&b.0))
				.map(|(_, s, perm)| (s, perm))
		});
		let Some((s, perm)) = pick else {
			return Err(anyhow::anyhow!(
				"{}: painted face {p} has no free source face",
				source.name
			));
		};
		if s != candidates[p][0].1 {
			displaced += 1;
		}
		claimed[s] = true;
		assignment[p] = Some((s, perm));
	}

	let mut out_index: HashMap<(u32, u32), u32> = HashMap::new();
	let mut bound = BoundMesh {
		name: source.name.clone(),
		material: painted.material.clone().unwrap_or_else(|| format!("{}_mat", source.name)),
		positions: Vec::new(),
		normals: Vec::new(),
		uvs: Vec::new(),
		joints: source.joints.as_ref().map(|_| Vec::new()),
		weights: source.weights.as_ref().map(|_| Vec::new()),
		skin: source.skin.clone(),
		indices: Vec::with_capacity(painted.faces.len() * 3),
		stats: BindStats {
			painted_faces: painted.faces.len(),
			source_faces: n_src,
			min_normal_dot: 1.0,
			displaced_faces: displaced,
			..Default::default()
		},
	};
	let mut distance_sum = 0.0;
	for (p, pface) in painted.faces.iter().enumerate() {
		let Some((s, perm_index)) = assignment[p] else {
			continue;
		};
		let sface = source.faces[s];
		let perm = perms[perm_index];
		// painted corner k ↔ source corner perm[k]; emit in source winding.
		let mut painted_for_source = [0usize; 3];
		for k in 0..3 {
			painted_for_source[perm[k]] = k;
		}
		for (sk, &src_vertex) in sface.iter().enumerate() {
			let k = painted_for_source[sk];
			let [_, vt, vn] = pface[k];
			let pv = pface[k][0];
			let src = src_vertex as usize;
			let dist = painted.positions[pv as usize].distance(source.baked_positions[src]);
			distance_sum += dist;
			bound.stats.max_corner_distance = bound.stats.max_corner_distance.max(dist);
			bound.stats.min_normal_dot = bound
				.stats
				.min_normal_dot
				.min(painted.normals[vn as usize].dot(source.baked_normals[src]));
			let key = (vt, src_vertex);
			let index = *out_index.entry(key).or_insert_with(|| {
				let id = bound.positions.len() as u32;
				bound.positions.push(source.positions[src]);
				bound.normals.push(source.normals[src]);
				bound.uvs.push(painted.uvs[vt as usize]);
				if let (Some(out), Some(src_joints)) = (bound.joints.as_mut(), &source.joints) {
					out.push(src_joints[src]);
				}
				if let (Some(out), Some(src_weights)) = (bound.weights.as_mut(), &source.weights) {
					out.push(src_weights[src]);
				}
				id
			});
			bound.indices.push(index);
		}
	}
	bound.stats.mean_corner_distance = distance_sum / (painted.faces.len().max(1) * 3) as f32;
	Ok(bound)
}

/// Bind every painted object to its source part, in [`PAINTED_MESH_ORDER`].
pub fn bind_all(
	sources: &[SourcePart],
	painted: &[PaintedObject],
) -> anyhow::Result<Vec<BoundMesh>> {
	let mut out = Vec::new();
	for name in PAINTED_MESH_ORDER {
		let source = sources
			.iter()
			.find(|s| s.name == name)
			.ok_or_else(|| anyhow::anyhow!("no runtime part mesh for {name}"))?;
		let object = painted
			.iter()
			.find(|o| o.name == name)
			.ok_or_else(|| anyhow::anyhow!("painted OBJ has no `o {name}`"))?;
		let bound = bind_painted(source, object)?;
		if bound.stats.max_corner_distance > MAX_CORNER_DISTANCE
			|| bound.stats.min_normal_dot < MIN_NORMAL_DOT
			|| bound.stats.displaced_faces > 0
		{
			return Err(anyhow::anyhow!(
				"{name}: painted OBJ does not fit the runtime mesh ({})",
				bound.stats
			));
		}
		out.push(bound);
	}
	Ok(out)
}

/// Write the GLB: one mesh (one primitive) per [`BoundMesh`], in order.
///
/// Each skinned mesh sits on a node with a skin that copies the part's joint names
/// and inverse bind matrices (joint nodes are flat, placed at their bind pose), so
/// the file is valid glTF and Bevy keeps `JOINTS_0` / `WEIGHTS_0`. The runtime only
/// takes `#Mesh{i}/Primitive0` and puts it on the part's existing skinned entity.
pub fn write_bound_glb(path: &Path, meshes: &[BoundMesh]) -> anyhow::Result<()> {
	use serde_json::json;
	let mut bin: Vec<u8> = Vec::new();
	let mut views = Vec::new();
	let mut accessors = Vec::new();
	let mut gltf_meshes = Vec::new();
	let mut materials = Vec::new();
	let mut nodes = Vec::new();
	let mut skins = Vec::new();
	let mut mesh_nodes = Vec::new();
	let mut joint_nodes: Vec<(String, Mat4)> = Vec::new();

	let mut push_view = |bin: &mut Vec<u8>, bytes: &[u8], target: u32| -> usize {
		while bin.len() % 4 != 0 {
			bin.push(0);
		}
		let offset = bin.len();
		bin.extend_from_slice(bytes);
		let mut view = json!({"buffer": 0, "byteOffset": offset, "byteLength": bytes.len()});
		if target != 0 {
			view["target"] = json!(target);
		}
		views.push(view);
		views.len() - 1
	};
	const ARRAY: u32 = 34962;
	const ELEMENT: u32 = 34963;

	for (mesh_index, mesh) in meshes.iter().enumerate() {
		let count = mesh.positions.len();
		let mut attributes = serde_json::Map::new();

		let mut min = Vec3::splat(f32::MAX);
		let mut max = Vec3::splat(f32::MIN);
		let mut bytes = Vec::with_capacity(count * 12);
		for p in &mesh.positions {
			min = min.min(*p);
			max = max.max(*p);
			for c in p.to_array() {
				bytes.extend_from_slice(&c.to_le_bytes());
			}
		}
		let view = push_view(&mut bin, &bytes, ARRAY);
		accessors.push(
			json!({"bufferView": view, "componentType": 5126, "count": count, "type": "VEC3",
			"min": min.to_array(), "max": max.to_array()}),
		);
		attributes.insert("POSITION".into(), json!(accessors.len() - 1));

		let mut bytes = Vec::with_capacity(count * 12);
		for n in &mesh.normals {
			for c in n.normalize_or(Vec3::Y).to_array() {
				bytes.extend_from_slice(&c.to_le_bytes());
			}
		}
		let view = push_view(&mut bin, &bytes, ARRAY);
		accessors.push(
			json!({"bufferView": view, "componentType": 5126, "count": count, "type": "VEC3"}),
		);
		attributes.insert("NORMAL".into(), json!(accessors.len() - 1));

		let mut bytes = Vec::with_capacity(count * 8);
		for uv in &mesh.uvs {
			for c in uv.to_array() {
				bytes.extend_from_slice(&c.to_le_bytes());
			}
		}
		let view = push_view(&mut bin, &bytes, ARRAY);
		accessors.push(
			json!({"bufferView": view, "componentType": 5126, "count": count, "type": "VEC2"}),
		);
		attributes.insert("TEXCOORD_0".into(), json!(accessors.len() - 1));

		if let (Some(joints), Some(weights)) = (&mesh.joints, &mesh.weights) {
			let mut bytes = Vec::with_capacity(count * 8);
			for j in joints {
				for c in j {
					bytes.extend_from_slice(&c.to_le_bytes());
				}
			}
			let view = push_view(&mut bin, &bytes, ARRAY);
			accessors.push(
				json!({"bufferView": view, "componentType": 5123, "count": count, "type": "VEC4"}),
			);
			attributes.insert("JOINTS_0".into(), json!(accessors.len() - 1));

			let mut bytes = Vec::with_capacity(count * 16);
			for w in weights {
				for c in w {
					bytes.extend_from_slice(&c.to_le_bytes());
				}
			}
			let view = push_view(&mut bin, &bytes, ARRAY);
			accessors.push(
				json!({"bufferView": view, "componentType": 5126, "count": count, "type": "VEC4"}),
			);
			attributes.insert("WEIGHTS_0".into(), json!(accessors.len() - 1));
		}

		let mut bytes = Vec::with_capacity(mesh.indices.len() * 4);
		for i in &mesh.indices {
			bytes.extend_from_slice(&i.to_le_bytes());
		}
		let view = push_view(&mut bin, &bytes, ELEMENT);
		accessors.push(json!({"bufferView": view, "componentType": 5125, "count": mesh.indices.len(), "type": "SCALAR"}));
		let indices = accessors.len() - 1;

		materials.push(json!({"name": mesh.material, "pbrMetallicRoughness": {"baseColorFactor": [1.0, 1.0, 1.0, 1.0], "metallicFactor": 1.0, "roughnessFactor": 1.0}, "doubleSided": true}));
		gltf_meshes.push(json!({
			"name": mesh.name,
			"primitives": [{"attributes": attributes, "indices": indices, "material": mesh_index, "mode": 4}],
		}));

		let mut node = json!({"name": mesh.name, "mesh": mesh_index});
		if let (Some(skin), Some(_)) = (&mesh.skin, &mesh.joints) {
			let mut bytes = Vec::with_capacity(skin.inverse_bindposes.len() * 64);
			for matrix in &skin.inverse_bindposes {
				for c in matrix.to_cols_array() {
					bytes.extend_from_slice(&c.to_le_bytes());
				}
			}
			let view = push_view(&mut bin, &bytes, 0);
			accessors.push(json!({"bufferView": view, "componentType": 5126, "count": skin.inverse_bindposes.len(), "type": "MAT4"}));
			let ibm = accessors.len() - 1;
			let first = joint_nodes.len();
			for (name, inverse) in skin.joint_names.iter().zip(&skin.inverse_bindposes) {
				joint_nodes.push((name.clone(), inverse.inverse()));
			}
			let joints: Vec<usize> = (first..joint_nodes.len()).collect();
			skins.push(
				json!({"name": format!("{}_skin", mesh.name), "inverseBindMatrices": ibm, "joints": joints}),
			);
			node["skin"] = json!(skins.len() - 1);
		}
		mesh_nodes.push(node);
	}
	// Skin joint indices count from 0, so joint nodes come first, then mesh nodes.
	for (name, bind) in &joint_nodes {
		nodes.push(json!({"name": name, "matrix": bind.to_cols_array()}));
	}
	nodes.extend(mesh_nodes);
	let scene_nodes: Vec<usize> = (0..nodes.len()).collect();
	while bin.len() % 4 != 0 {
		bin.push(0);
	}
	let mut doc = json!({
		"asset": {"version": "2.0", "generator": "maybraid character-mesh-export --rebind-painted"},
		"buffers": [{"byteLength": bin.len()}],
		"bufferViews": views,
		"accessors": accessors,
		"materials": materials,
		"meshes": gltf_meshes,
		"nodes": nodes,
		"scenes": [{"name": "painted", "nodes": scene_nodes}],
		"scene": 0,
	});
	if !skins.is_empty() {
		doc["skins"] = json!(skins);
	}
	let mut json_bytes = serde_json::to_vec(&doc)?;
	while json_bytes.len() % 4 != 0 {
		json_bytes.push(b' ');
	}
	let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
	let mut out = Vec::with_capacity(total);
	out.extend_from_slice(b"glTF");
	out.extend_from_slice(&2u32.to_le_bytes());
	out.extend_from_slice(&(total as u32).to_le_bytes());
	out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
	out.extend_from_slice(b"JSON");
	out.extend_from_slice(&json_bytes);
	out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
	out.extend_from_slice(b"BIN\0");
	out.extend_from_slice(&bin);
	if let Some(dir) = path.parent() {
		std::fs::create_dir_all(dir)?;
	}
	std::fs::write(path, out)?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Double-sided quad flap: front and back share corner positions but not normals,
	/// exactly the case that position-only matching would swap.
	fn flap_source() -> SourcePart {
		let corners = [Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::Y];
		let mut positions = Vec::new();
		let mut normals = Vec::new();
		let mut joints = Vec::new();
		let mut weights = Vec::new();
		for (side, normal) in [(0u16, Vec3::Z), (1u16, Vec3::NEG_Z)] {
			for c in corners {
				positions.push(c);
				normals.push(normal);
				joints.push([side, 2, 0, 0]);
				weights.push([0.75, 0.25, 0.0, 0.0]);
			}
		}
		// Baked = rest pose moved 2 m up with a 0.5 mm wobble per vertex.
		let baked_positions: Vec<Vec3> = positions
			.iter()
			.enumerate()
			.map(|(i, p)| *p + Vec3::new(0.0, 2.0, 0.0) + Vec3::splat(0.0005 * (i % 2) as f32))
			.collect();
		SourcePart {
			name: "ear_left".into(),
			baked_normals: normals.clone(),
			positions,
			normals,
			joints: Some(joints),
			weights: Some(weights),
			skin: Some(SourceSkin {
				joint_names: vec!["root".into(), "flap.base".into(), "flap.tip".into()],
				inverse_bindposes: vec![
					Mat4::IDENTITY,
					Mat4::from_translation(Vec3::NEG_Y),
					Mat4::from_translation(Vec3::new(0.0, -2.0, 0.0)),
				],
			}),
			baked_positions,
			faces: vec![[0, 1, 2], [0, 2, 3], [4, 6, 5], [4, 7, 6]],
		}
	}

	/// Export-style OBJ of the flap: back faces first, rotated corners, its own UVs.
	const FLAP_OBJ: &str = "\
o ear_left
v 0 2 0
v 1 2 0
v 1 3 0
v 0 3 0
v 0 2 0
v 1 2 0
v 1 3 0
v 0 3 0
vt 0.6 0.1
vt 0.9 0.1
vt 0.9 0.4
vt 0.6 0.4
vt 0.1 0.6
vt 0.4 0.6
vt 0.4 0.9
vt 0.1 0.9
vn 0 0 -1
vn 0 0 -1
vn 0 0 -1
vn 0 0 -1
vn 0 0 1
vn 0 0 1
vn 0 0 1
vn 0 0 1
s off
usemtl ear_left_mat
f 2/2/2 1/1/1 3/3/3
f 3/3/3 1/1/1 4/4/4
f 5/5/5 6/6/6 7/7/7
f 7/7/7 8/8/8 5/5/5
";

	#[test]
	fn parses_objects_and_flips_v_to_gltf() -> anyhow::Result<()> {
		let objects = parse_painted_obj(FLAP_OBJ)?;
		anyhow::ensure!(objects.len() == 1);
		let flap = &objects[0];
		anyhow::ensure!(flap.material.as_deref() == Some("ear_left_mat"));
		anyhow::ensure!(flap.faces.len() == 4 && flap.positions.len() == 8);
		// First corner is `vt 0.9 0.1` (OBJ, bottom-left origin) → glTF (0.9, 0.9).
		anyhow::ensure!((flap.uvs[0] - Vec2::new(0.9, 0.9)).length() < 1e-6, "{:?}", flap.uvs[0]);
		Ok(())
	}

	#[test]
	fn front_and_back_keep_their_own_uvs_and_source_skin() -> anyhow::Result<()> {
		let source = flap_source();
		let painted = &parse_painted_obj(FLAP_OBJ)?[0];
		let bound = bind_painted(&source, painted)?;
		anyhow::ensure!(bound.stats.displaced_faces == 0, "{}", bound.stats);
		anyhow::ensure!(bound.stats.max_corner_distance < 0.001, "{}", bound.stats);
		anyhow::ensure!(bound.indices.len() == 12);
		let joints = bound.joints.as_ref().ok_or_else(|| anyhow::anyhow!("joints"))?;
		for (i, uv) in bound.uvs.iter().enumerate() {
			// Back (-Z, joint 1) was painted in the u > 0.5 tile, front (+Z, joint 0) below.
			let back = bound.normals[i].z < 0.0;
			anyhow::ensure!(joints[i][0] == u16::from(back), "vertex {i} joints {:?}", joints[i]);
			anyhow::ensure!((uv.x > 0.5) == back, "vertex {i} uv {uv:?} back={back}");
			// Rest (local) positions, not baked ones.
			anyhow::ensure!(bound.positions[i].y <= 1.0 + 1e-6);
		}
		// Source winding is kept: the first emitted triangle is source face [0, 1, 2].
		let first: Vec<Vec3> =
			bound.indices[0..3].iter().map(|&i| bound.positions[i as usize]).collect();
		let back_tri = [Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0), Vec3::X];
		let front_tri = [Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0)];
		anyhow::ensure!(first == back_tri || first == front_tri, "{first:?}");
		Ok(())
	}

	#[test]
	fn far_pose_is_rejected_by_bind_all() -> anyhow::Result<()> {
		let mut source = flap_source();
		for p in &mut source.baked_positions {
			*p += Vec3::new(0.3, 0.0, 0.0);
		}
		let mut sources = vec![source];
		let painted = parse_painted_obj(FLAP_OBJ)?;
		// bind_all needs every component; check the gate on the one we have.
		let bound = bind_painted(&sources[0], &painted[0])?;
		anyhow::ensure!(bound.stats.max_corner_distance > MAX_CORNER_DISTANCE);
		sources.clear();
		anyhow::ensure!(bind_all(&sources, &painted).is_err());
		Ok(())
	}

	#[test]
	fn glb_round_trips_attributes() -> anyhow::Result<()> {
		let source = flap_source();
		let painted = &parse_painted_obj(FLAP_OBJ)?[0];
		let bound = bind_painted(&source, painted)?;
		let dir = tempfile::tempdir()?;
		let path = dir.path().join("painted.glb");
		write_bound_glb(&path, std::slice::from_ref(&bound))?;
		let bytes = std::fs::read(&path)?;
		anyhow::ensure!(&bytes[0..4] == b"glTF");
		anyhow::ensure!(u32::from_le_bytes(bytes[8..12].try_into()?) as usize == bytes.len());
		let json_len = u32::from_le_bytes(bytes[12..16].try_into()?) as usize;
		let doc: serde_json::Value = serde_json::from_slice(&bytes[20..20 + json_len])?;
		let primitive = &doc["meshes"][0]["primitives"][0];
		for key in ["POSITION", "NORMAL", "TEXCOORD_0", "JOINTS_0", "WEIGHTS_0"] {
			let accessor = primitive["attributes"][key]
				.as_u64()
				.ok_or_else(|| anyhow::anyhow!("missing {key}"))?;
			anyhow::ensure!(doc["accessors"][accessor as usize]["count"] == bound.positions.len());
		}
		anyhow::ensure!(doc["materials"][0]["name"] == "ear_left_mat");
		// Skinned mesh node carries a skin with the part's joints (so loaders keep JOINTS_0).
		let skin = &doc["skins"][0];
		anyhow::ensure!(skin["joints"].as_array().map(Vec::len) == Some(3), "{skin}");
		let mesh_node = doc["nodes"]
			.as_array()
			.and_then(|nodes| nodes.iter().find(|n| n["mesh"] == 0))
			.ok_or_else(|| anyhow::anyhow!("no node for mesh 0"))?;
		anyhow::ensure!(mesh_node["skin"] == 0);
		let joint0 = skin["joints"][1].as_u64().unwrap_or(99) as usize;
		anyhow::ensure!(doc["nodes"][joint0]["name"] == "flap.base");
		anyhow::ensure!(doc["meshes"][0]["name"] == "ear_left");
		Ok(())
	}
}
