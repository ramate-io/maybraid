//! Runtime skinned chain mesh generation for [#1125](https://github.com/ramate-io/maybraid/issues/1125).
//!
//! Generates deformable chain meshes with runtime-created armatures and configurable segment counts.
//! Satisfies the material contract (UV0 around-link, UV1.x rest-arc metres, link IDs in vertex color R).
//!
//! ## Skinning strategy
//!
//! The implementation uses a **smooth blending** skinning approach that deforms continuously along
//! the chain. Each link is influenced by its local joint and the neighboring joints, creating
//! smooth transitions. This strategy is kept behind a clean seam (`ChainSkinningStrategy`) so
//! rigid-per-link skinning could be swapped in if needed (see open decision C1 in PR #1358).
//!
//! ## Bone/rest contract
//!
//! - **Joints**: One joint per link, positioned at the link center in rest pose along +Y axis.
//!   Joint `i` is at `y = link_spacing * i`, starting from `y = 0`.
//! - **Rest pose**: All links lie along a straight vertical line (+Y axis).
//! - **Inverse bind poses**: Transform from world space (rest pose) to joint-local space.
//! - **Deformation**: Applied by updating joint transforms. The skinned mesh follows the pose.
//!
//! ## Usage
//!
//! ```ignore
//! let config = SkinnedChainConfig {
//!     length_m: 3.1,
//!     link_count: 14,
//!     link_radius: 0.16,
//!     tube_radius: 0.052,
//!     ..Default::default()
//! };
//!
//! let chain = generate_skinned_chain(config);
//! // Use chain.mesh, chain.inverse_bindposes, and spawn joints at chain.rest_joint_positions
//! ```

use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;

/// Skinning weight distribution strategy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainSkinningStrategy {
	/// Smooth blending between joints (continuous deformation).
	/// Each link is influenced by its local joint and neighbors with falloff.
	SmoothBlend,
	/// Rigid per-link weights (one joint per link, weight = 1.0).
	/// Each link follows exactly one joint without blending.
	RigidPerLink,
}

impl Default for ChainSkinningStrategy {
	fn default() -> Self {
		// Default to smooth blending as specified in #1125
		Self::SmoothBlend
	}
}

/// Configuration for runtime skinned chain generation.
#[derive(Clone, Debug)]
pub struct SkinnedChainConfig {
	/// Total rest length of the chain in metres.
	pub length_m: f32,
	/// Number of chain links (minimum 2).
	pub link_count: u32,
	/// Radius of each link ring (distance from center to tube center).
	pub link_radius: f32,
	/// Radius of the tube cross-section.
	pub tube_radius: f32,
	/// Segments around each link ring (U direction, minimum 6).
	pub segments_ring: u32,
	/// Segments around the tube cross-section (V direction, minimum 4).
	pub segments_tube: u32,
	/// Skinning weight distribution strategy.
	pub skinning_strategy: ChainSkinningStrategy,
}

impl Default for SkinnedChainConfig {
	fn default() -> Self {
		Self {
			length_m: 3.1,
			link_count: 14,
			link_radius: 0.16,
			tube_radius: 0.052,
			segments_ring: 20,
			segments_tube: 12,
			skinning_strategy: ChainSkinningStrategy::default(),
		}
	}
}

impl SkinnedChainConfig {
	/// Validate and clamp configuration to safe ranges.
	pub fn validated(mut self) -> Self {
		self.length_m = self.length_m.max(0.1);
		self.link_count = self.link_count.max(2);
		self.link_radius = self.link_radius.max(0.01);
		self.tube_radius = self.tube_radius.max(0.001);
		self.segments_ring = self.segments_ring.max(6);
		self.segments_tube = self.segments_tube.max(4);
		self
	}

	/// Spacing between link centers in rest pose.
	pub fn link_spacing(&self) -> f32 {
		self.length_m / (self.link_count.saturating_sub(1) as f32).max(1.0)
	}
}

/// Generated skinned chain data ready for ECS spawning.
pub struct SkinnedChainData {
	/// The generated mesh with skinning attributes.
	pub mesh: Mesh,
	/// Inverse bind pose matrices (one per joint).
	pub inverse_bindposes: Vec<Mat4>,
	/// Rest positions for joint entities (world space, along +Y).
	pub rest_joint_positions: Vec<Vec3>,
}

/// Generate a complete skinned chain mesh with joints and bind poses.
///
/// Returns the mesh, inverse bind poses, and rest joint positions ready for spawning.
/// The mesh satisfies the material contract:
/// - UV0.xy: around-link (u) and link-local (v), seams duplicated
/// - UV1.x: rest-arc metres (cumulative distance from start)
/// - UV1.y: total rest length
/// - Vertex color R: link index as f32
/// - Vertex color B: twist phase (0.0 even, 1.0 odd)
/// - JOINT_INDEX / JOINT_WEIGHT: skinning data
pub fn generate_skinned_chain(config: SkinnedChainConfig) -> SkinnedChainData {
	let config = config.validated();
	let link_spacing = config.link_spacing();

	// Generate rest joint positions (one per link, along +Y)
	let rest_joint_positions: Vec<Vec3> = (0..config.link_count)
		.map(|i| Vec3::new(0.0, link_spacing * i as f32, 0.0))
		.collect();

	// Generate inverse bind poses (world to joint-local)
	let inverse_bindposes: Vec<Mat4> = rest_joint_positions
		.iter()
		.map(|pos| Mat4::from_translation(*pos).inverse())
		.collect();

	// Generate mesh geometry
	let mesh = generate_chain_mesh(&config, &rest_joint_positions);

	SkinnedChainData { mesh, inverse_bindposes, rest_joint_positions }
}

/// Generate the chain mesh geometry with skinning attributes.
fn generate_chain_mesh(config: &SkinnedChainConfig, joint_positions: &[Vec3]) -> Mesh {
	let links = config.link_count;
	let ring_segs = config.segments_ring;
	let tube_segs = config.segments_tube;

	// Include seam duplicates for UV wrapping
	let tube_verts = tube_segs + 1;

	let mut positions = Vec::new();
	let mut normals = Vec::new();
	let mut uvs = Vec::new();
	let mut flow_uvs = Vec::new();
	let mut colors = Vec::new();
	let mut joint_indices = Vec::new();
	let mut joint_weights = Vec::new();
	let mut indices = Vec::new();

	// Generate vertices for each link
	for link_idx in 0..links {
		let link_center = joint_positions[link_idx as usize];
		let odd_link = link_idx % 2 == 1;
		let base_index = positions.len() as u32;

		// For each ring segment (around the link)
		for ring_i in 0..=ring_segs {
			let ring_u = ring_i as f32 / ring_segs as f32;
			let theta = TAU * ring_u;

			// Ring position on the torus centerline
			let ring_x = config.link_radius * theta.cos();
			let ring_z = config.link_radius * theta.sin();

			// For each tube segment (around the tube cross-section)
			for tube_j in 0..=tube_segs {
				let tube_v = tube_j as f32 / tube_segs as f32;
				let phi = TAU * tube_v;

				// Compute vertex position on torus
				let (vertex_pos, vertex_normal) =
					torus_point_and_normal(config, ring_x, ring_z, theta, phi, link_center, odd_link);

				positions.push([vertex_pos.x, vertex_pos.y, vertex_pos.z]);
				normals.push([vertex_normal.x, vertex_normal.y, vertex_normal.z]);

				// UV0: around-link (u) and link-local (v)
				// u wraps around the tube, v is link-local position
				uvs.push([tube_v, (link_idx as f32 + ring_u) / links as f32]);

				// UV1: rest-arc metres (cumulative from start)
				// Use link center Y as the rest arc position (link centers are along +Y)
				let rest_arc_m = link_center.y;
				flow_uvs.push([rest_arc_m, config.length_m]);

				// Vertex color: R = link id, G = 0 (legacy), B = twist phase, A = 1
				let twist_phase = if odd_link { 1.0 } else { 0.0 };
				colors.push([link_idx as f32, 0.0, twist_phase, 1.0]);

				// Skinning weights
				let (indices, weights) =
					compute_skin_weights(config, link_idx, ring_u, joint_positions.len());
				joint_indices.push(indices);
				joint_weights.push(weights);
			}
		}

		// Generate triangle indices for this link
		for ring_i in 0..ring_segs {
			for tube_j in 0..tube_segs {
				let a = base_index + ring_i * tube_verts + tube_j;
				let b = base_index + ring_i * tube_verts + tube_j + 1;
				let c = base_index + (ring_i + 1) * tube_verts + tube_j + 1;
				let d = base_index + (ring_i + 1) * tube_verts + tube_j;

				// Two triangles per quad
				indices.extend_from_slice(&[a, b, c, a, c, d]);
			}
		}
	}

	// Build the mesh
	let mut mesh = Mesh::new(
		PrimitiveTopology::TriangleList,
		RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
	);

	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
	mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
	mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
	mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, flow_uvs);
	mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
	mesh.insert_attribute(
		Mesh::ATTRIBUTE_JOINT_INDEX,
		VertexAttributeValues::Uint16x4(joint_indices),
	);
	mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_weights);
	mesh.insert_indices(Indices::U32(indices));

	mesh
}

/// Compute vertex position and normal on a torus.
fn torus_point_and_normal(
	config: &SkinnedChainConfig,
	ring_x: f32,
	ring_z: f32,
	theta: f32,
	phi: f32,
	center: Vec3,
	odd_link: bool,
) -> (Vec3, Vec3) {
	// For odd links, rotate by 90 degrees to create the interlocking pattern
	let rotation = if odd_link { PI / 2.0 } else { 0.0 };
	let cos_rot = rotation.cos();
	let sin_rot = rotation.sin();

	// Tube offset from ring centerline
	let tube_x = config.tube_radius * phi.cos();
	let tube_y = config.tube_radius * phi.sin();

	// Apply rotation for odd links
	let rotated_x = ring_x * cos_rot - ring_z * sin_rot;
	let rotated_z = ring_x * sin_rot + ring_z * cos_rot;

	// Final position
	let local_x = rotated_x + tube_x * theta.cos();
	let local_z = rotated_z + tube_x * theta.sin();
	let local_y = tube_y;

	let position = center + Vec3::new(local_x, local_y, local_z);

	// Compute normal (direction from ring center to tube surface)
	let tube_dir = Vec3::new(phi.cos() * theta.cos(), phi.sin(), phi.cos() * theta.sin());
	let normal = tube_dir.normalize();

	(position, normal)
}

/// Compute skinning joint indices and weights for a vertex.
fn compute_skin_weights(
	config: &SkinnedChainConfig,
	link_idx: u32,
	ring_param: f32,
	joint_count: usize,
) -> ([u16; 4], [f32; 4]) {
	match config.skinning_strategy {
		ChainSkinningStrategy::RigidPerLink => {
			// Rigid: each link follows exactly one joint
			([link_idx as u16, 0, 0, 0], [1.0, 0.0, 0.0, 0.0])
		}
		ChainSkinningStrategy::SmoothBlend => {
			// Smooth: blend between adjacent joints based on position along the link
			// Weight distribution: current joint gets most weight, neighbors get falloff
			let current = link_idx as u16;
			let next = (link_idx + 1).min(joint_count as u32 - 1) as u16;

			// Blend factor based on ring position (0.0 at link start, 1.0 at link end)
			// Use a smooth falloff to avoid harsh seams
			let blend = (ring_param * 0.5).min(0.3); // Conservative blend at link boundaries

			let current_weight = 1.0 - blend;
			let next_weight = blend;

			([current, next, 0, 0], [current_weight, next_weight, 0.0, 0.0])
		}
	}
}

/// Spawn a complete skinned chain entity with joints.
///
/// Returns the root entity (owns the chain), mesh entity, and joint entities.
pub fn spawn_skinned_chain(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
	config: SkinnedChainConfig,
	transform: Transform,
	name: &str,
) -> (Entity, Entity, Vec<Entity>) {
	let data = generate_skinned_chain(config);

	let mesh_handle = meshes.add(data.mesh);
	let inverse_handle = bindposes.add(SkinnedMeshInverseBindposes::from(data.inverse_bindposes));

	// Create root entity (owns everything)
	let root = commands
		.spawn((
			Name::new(format!("{name}-chain-root")),
			Transform::IDENTITY,
			Visibility::default(),
		))
		.id();

	// Create joint entities
	let mut joint_entities = Vec::new();
	for (i, pos) in data.rest_joint_positions.iter().enumerate() {
		let joint = commands
			.spawn((
				Name::new(format!("{name}-joint-{i}")),
				Transform::from_translation(*pos),
				Visibility::default(),
			))
			.id();
		commands.entity(root).add_child(joint);
		joint_entities.push(joint);
	}

	// Create mesh entity with skinning
	let mesh_entity = commands
		.spawn((
			Name::new(format!("{name}-mesh")),
			Mesh3d(mesh_handle),
			transform,
			Visibility::default(),
			SkinnedMesh { inverse_bindposes: inverse_handle, joints: joint_entities.clone() },
		))
		.id();
	commands.entity(root).add_child(mesh_entity);

	(root, mesh_entity, joint_entities)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn generates_valid_chain_data() {
		let config = SkinnedChainConfig::default();
		let data = generate_skinned_chain(config.clone());

		// Check joint count matches link count
		assert_eq!(data.rest_joint_positions.len(), config.link_count as usize);
		assert_eq!(data.inverse_bindposes.len(), config.link_count as usize);

		// Check joint positions are along +Y
		for (i, pos) in data.rest_joint_positions.iter().enumerate() {
			let expected_y = config.link_spacing() * i as f32;
			assert!((pos.y - expected_y).abs() < 1e-4);
			assert!(pos.x.abs() < 1e-6);
			assert!(pos.z.abs() < 1e-6);
		}
	}

	#[test]
	fn mesh_has_required_attributes() {
		let config = SkinnedChainConfig::default();
		let data = generate_skinned_chain(config);

		// Required attributes for material contract
		assert!(data.mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
		assert!(data.mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
		assert!(data.mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
		assert!(data.mesh.attribute(Mesh::ATTRIBUTE_UV_1).is_some());
		assert!(data.mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_some());

		// Skinning attributes
		assert!(data.mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_some());
		assert!(data.mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_some());
	}

	#[test]
	fn uv1_contains_rest_arc_metres() {
		let config = SkinnedChainConfig { link_count: 4, length_m: 2.0, ..Default::default() };
		let data = generate_skinned_chain(config.clone());

		if let Some(VertexAttributeValues::Float32x2(uvs)) = data.mesh.attribute(Mesh::ATTRIBUTE_UV_1)
		{
			// Check that UV1.x values are in the expected range [0, length_m]
			let mut min_s = f32::MAX;
			let mut max_s = f32::MIN;
			for uv in uvs {
				min_s = min_s.min(uv[0]);
				max_s = max_s.max(uv[0]);
				// UV1.y should be total length
				assert!((uv[1] - config.length_m).abs() < 1e-4);
			}
			assert!(min_s >= -1e-4);
			assert!(max_s <= config.length_m + 1e-4);
		} else {
			panic!("Missing UV1 attribute");
		}
	}

	#[test]
	fn link_ids_in_vertex_color() {
		let config = SkinnedChainConfig { link_count: 5, ..Default::default() };
		let data = generate_skinned_chain(config.clone());

		if let Some(VertexAttributeValues::Float32x4(colors)) =
			data.mesh.attribute(Mesh::ATTRIBUTE_COLOR)
		{
			// Check that link IDs are present and in range
			let mut found_links = std::collections::HashSet::new();
			for color in colors {
				let link_id = color[0].round() as u32;
				assert!(link_id < config.link_count);
				found_links.insert(link_id);
			}
			// All links should be represented
			assert_eq!(found_links.len(), config.link_count as usize);
		} else {
			panic!("Missing vertex color attribute");
		}
	}

	#[test]
	fn rigid_skinning_uses_one_joint_per_link() {
		let config =
			SkinnedChainConfig { skinning_strategy: ChainSkinningStrategy::RigidPerLink, ..Default::default() };
		let data = generate_skinned_chain(config);

		if let Some(VertexAttributeValues::Float32x4(weights)) =
			data.mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
		{
			// For rigid skinning, first weight should be 1.0, rest should be 0.0
			for w in weights {
				assert!((w[0] - 1.0).abs() < 1e-4);
				assert!(w[1].abs() < 1e-4);
				assert!(w[2].abs() < 1e-4);
				assert!(w[3].abs() < 1e-4);
			}
		} else {
			panic!("Missing joint weights");
		}
	}

	#[test]
	fn smooth_skinning_blends_weights() {
		let config = SkinnedChainConfig {
			skinning_strategy: ChainSkinningStrategy::SmoothBlend,
			..Default::default()
		};
		let data = generate_skinned_chain(config);

		if let Some(VertexAttributeValues::Float32x4(weights)) =
			data.mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
		{
			// For smooth skinning, some vertices should have blended weights
			let mut found_blended = false;
			for w in weights {
				let sum = w[0] + w[1] + w[2] + w[3];
				assert!((sum - 1.0).abs() < 1e-4); // Weights sum to 1
				if w[1] > 1e-4 {
					found_blended = true;
				}
			}
			// Should find at least some blended weights
			assert!(found_blended);
		} else {
			panic!("Missing joint weights");
		}
	}

	#[test]
	fn validates_config_bounds() {
		let config = SkinnedChainConfig {
			length_m: -1.0,
			link_count: 1,
			link_radius: 0.0,
			segments_ring: 2,
			..Default::default()
		}
		.validated();

		assert!(config.length_m >= 0.1);
		assert!(config.link_count >= 2);
		assert!(config.link_radius >= 0.01);
		assert!(config.segments_ring >= 6);
	}

	#[test]
	fn different_lengths_produce_correct_spacing() {
		for length in [1.0, 3.0, 5.0] {
			let config = SkinnedChainConfig { length_m: length, link_count: 10, ..Default::default() };
			let data = generate_skinned_chain(config.clone());

			let spacing = data.rest_joint_positions[1].y - data.rest_joint_positions[0].y;
			let expected_spacing = config.link_spacing();
			assert!((spacing - expected_spacing).abs() < 1e-4);
		}
	}

	#[test]
	fn inverse_bind_poses_invert_rest_positions() {
		let config = SkinnedChainConfig::default();
		let data = generate_skinned_chain(config);

		for (pos, inv_bind) in data.rest_joint_positions.iter().zip(data.inverse_bindposes.iter()) {
			let bind = Mat4::from_translation(*pos);
			let should_be_identity = bind * (*inv_bind);

			// Check it's close to identity
			for i in 0..4 {
				for j in 0..4 {
					let expected = if i == j { 1.0 } else { 0.0 };
					assert!((should_be_identity.to_cols_array_2d()[i][j] - expected).abs() < 1e-4);
				}
			}
		}
	}
}
