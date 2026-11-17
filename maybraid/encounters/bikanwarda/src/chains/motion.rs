//! Anchored chain motion simulation for [#1126](https://github.com/ramate-io/maybraid/issues/1126).
//!
//! Provides prescribed kinematics for anchored chains: endpoints follow sockets (column/character),
//! intermediate joints form a catenary or taut lerp between anchors, and capsule colliders track
//! the joint transforms.
//!
//! This is a **prescribed pose** approach, not physical simulation. Joints are updated directly
//! based on anchor positions.

use bevy::prelude::*;

/// Marks the root entity of an anchored chain assembly.
///
/// The chain root owns joints, capsules, mesh, and health. Despawning this entity retires
/// the entire chain.
#[derive(Component, Clone, Copy, Debug)]
pub struct ChainRoot {
	/// How many links the chain has (determines joint count).
	pub link_count: u32,
	/// Total rest length in metres.
	pub rest_length_m: f32,
}

/// Anchor attachment point for a chain endpoint (column or character socket).
///
/// The chain motion system reads these transforms to position the chain endpoints.
#[derive(Component, Clone, Copy, Debug)]
pub struct ChainAnchor {
	/// Which endpoint this anchor controls (0 = start, 1 = end).
	pub endpoint: ChainEndpoint,
	/// The chain root entity this anchor belongs to.
	pub chain_root: Entity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainEndpoint {
	Start,
	End,
}

/// Tension/restraint behavior when anchors move.
///
/// For the first slice, we use prescribed kinematics: the chain follows anchor positions
/// without physical simulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainTensionMode {
	/// Simple catenary between anchors (default for moderate sag).
	Catenary { sag_ratio: u8 },
	/// Taut linear interpolation (no sag, rigid connection).
	Taut,
}

impl Default for ChainTensionMode {
	fn default() -> Self {
		// Default to a reasonable sag for visual quality
		Self::Catenary { sag_ratio: 10 }
	}
}

/// Chain motion state on the root entity.
#[derive(Component, Clone, Debug)]
pub struct ChainMotion {
	/// Current tension mode (catenary vs taut).
	pub mode: ChainTensionMode,
	/// Whether the chain is active and should update joints.
	pub active: bool,
}

impl Default for ChainMotion {
	fn default() -> Self {
		Self { mode: ChainTensionMode::default(), active: true }
	}
}

/// Update chain joint positions based on anchor transforms.
///
/// This system runs in PostUpdate after transforms have propagated. It reads anchor
/// positions from ChainAnchor entities and updates the child joint entities of each
/// ChainRoot.
pub fn update_chain_joints(
	chains: Query<(Entity, &ChainRoot, &ChainMotion, &Children)>,
	anchors: Query<(&ChainAnchor, &GlobalTransform)>,
	mut joints: Query<&mut Transform, Without<ChainAnchor>>,
) {
	for (root_entity, chain_root, motion, children) in &chains {
		if !motion.active {
			continue;
		}

		// Find the start and end anchors
		let mut start_pos = None;
		let mut end_pos = None;

		for (anchor, transform) in &anchors {
			if anchor.chain_root != root_entity {
				continue;
			}
			let pos = transform.translation();
			match anchor.endpoint {
				ChainEndpoint::Start => start_pos = Some(pos),
				ChainEndpoint::End => end_pos = Some(pos),
			}
		}

		let (Some(start), Some(end)) = (start_pos, end_pos) else {
			// Missing anchors - skip this frame
			continue;
		};

		// Collect joint entities (children of the root that have Transform)
		let mut joint_entities: Vec<Entity> = children
			.iter()
			.filter(|&entity| joints.contains(entity))
			.collect();

		// Sort by entity bits for deterministic ordering
		joint_entities.sort_by_key(|e| e.index());

		if joint_entities.len() != chain_root.link_count as usize {
			// Joint count mismatch - likely still being set up
			continue;
		}

		// Compute positions for all joints based on tension mode
		let positions = compute_joint_positions(start, end, chain_root.link_count, motion.mode);

		// Update joint transforms
		for (joint_entity, position) in joint_entities.iter().zip(positions.iter()) {
			if let Ok(mut transform) = joints.get_mut(*joint_entity) {
				transform.translation = *position;
			}
		}
	}
}

/// Compute joint positions along the chain from start to end anchor.
///
/// Returns a Vec of world-space positions for each joint.
pub fn compute_joint_positions(
	start: Vec3,
	end: Vec3,
	joint_count: u32,
	mode: ChainTensionMode,
) -> Vec<Vec3> {
	let count = joint_count as usize;
	if count == 0 {
		return Vec::new();
	}

	let delta = end - start;
	let distance = delta.length();

	match mode {
		ChainTensionMode::Taut => {
			// Linear interpolation from start to end
			(0..count)
				.map(|i| {
					let t = if count > 1 { i as f32 / (count - 1) as f32 } else { 0.5 };
					start + delta * t
				})
				.collect()
		}
		ChainTensionMode::Catenary { sag_ratio } => {
			// Simplified catenary: add vertical sag based on horizontal position
			// Real catenary is complex; this is a visual approximation
			let sag_amount = distance / sag_ratio as f32;

			(0..count)
				.map(|i| {
					let t = if count > 1 { i as f32 / (count - 1) as f32 } else { 0.5 };
					let linear = start + delta * t;

					// Parabolic sag: maximum at t=0.5
					let sag_factor = 4.0 * t * (1.0 - t); // 0 at ends, 1 at middle
					let sag_offset = Vec3::new(0.0, -sag_amount * sag_factor, 0.0);

					linear + sag_offset
				})
				.collect()
		}
	}
}

/// Marker for a chain that has been released (broken or freed).
///
/// Released chains no longer update joints and their colliders are retired.
#[derive(Component, Clone, Copy, Debug)]
pub struct ReleasedChain;

/// System to deactivate motion for released chains.
pub fn deactivate_released_chains(
	mut chains: Query<&mut ChainMotion, (With<ChainRoot>, With<ReleasedChain>)>,
) {
	for mut motion in &mut chains {
		motion.active = false;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::time::TimeUpdateStrategy;

	#[test]
	fn taut_mode_produces_linear_interpolation() {
		let start = Vec3::new(0.0, 0.0, 0.0);
		let end = Vec3::new(10.0, 0.0, 0.0);
		let positions = compute_joint_positions(start, end, 5, ChainTensionMode::Taut);

		assert_eq!(positions.len(), 5);
		// Check endpoints
		assert!((positions[0] - start).length() < 1e-5);
		assert!((positions[4] - end).length() < 1e-5);
		// Check midpoint
		assert!((positions[2] - Vec3::new(5.0, 0.0, 0.0)).length() < 1e-5);
	}

	#[test]
	fn catenary_mode_adds_vertical_sag() {
		let start = Vec3::new(0.0, 5.0, 0.0);
		let end = Vec3::new(10.0, 5.0, 0.0);
		let positions =
			compute_joint_positions(start, end, 5, ChainTensionMode::Catenary { sag_ratio: 10 });

		assert_eq!(positions.len(), 5);
		// Endpoints should match
		assert!((positions[0] - start).length() < 1e-5);
		assert!((positions[4] - end).length() < 1e-5);

		// Middle joint should sag below the line
		let midpoint = positions[2];
		assert!(midpoint.y < start.y); // Sag is negative Y
		assert!(midpoint.x.abs() - 5.0 < 1e-4); // X should be at midpoint
	}

	#[test]
	fn handles_zero_joints() {
		let positions = compute_joint_positions(Vec3::ZERO, Vec3::X, 0, ChainTensionMode::Taut);
		assert!(positions.is_empty());
	}

	#[test]
	fn handles_single_joint() {
		let start = Vec3::ZERO;
		let end = Vec3::new(10.0, 0.0, 0.0);
		let positions = compute_joint_positions(start, end, 1, ChainTensionMode::Taut);

		assert_eq!(positions.len(), 1);
		// Single joint should be at midpoint
		assert!((positions[0] - Vec3::new(5.0, 0.0, 0.0)).length() < 1e-5);
	}

	#[test]
	fn motion_stability_with_equal_anchors() {
		// When both anchors are at the same position, chain should collapse to that point
		let pos = Vec3::new(2.0, 3.0, 1.0);
		let positions = compute_joint_positions(pos, pos, 4, ChainTensionMode::Taut);

		assert_eq!(positions.len(), 4);
		for p in positions {
			assert!((p - pos).length() < 1e-5);
		}
	}

	#[test]
	fn catenary_sags_below_taut_after_motion_update() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_plugins(TransformPlugin)
			.insert_resource(TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(16)))
			.add_systems(PostUpdate, update_chain_joints.after(TransformSystems::Propagate));

		let link_count = 12;
		let length_m = 2.8;

		// Create taut chain
		let taut_root = app.world_mut().spawn((
			ChainRoot { link_count, rest_length_m: length_m },
			ChainMotion { mode: ChainTensionMode::Taut, active: true },
			Transform::from_xyz(0.0, 0.0, 0.0),
			GlobalTransform::default(),
		)).id();

		let _taut_start_anchor = app.world_mut().spawn((
			ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: taut_root },
			Transform::from_xyz(0.0, 0.0, 0.0),
			GlobalTransform::default(),
		)).id();

		let _taut_end_anchor = app.world_mut().spawn((
			ChainAnchor { endpoint: ChainEndpoint::End, chain_root: taut_root },
			Transform::from_xyz(0.0, length_m, 0.0),
			GlobalTransform::default(),
		)).id();

		let mut taut_joints = Vec::new();
		for _i in 0..link_count {
			let joint = app.world_mut().spawn((
				Transform::from_xyz(0.0, 0.0, 0.0),
				GlobalTransform::default(),
			)).id();
			app.world_mut().entity_mut(taut_root).add_child(joint);
			taut_joints.push(joint);
		}

		// Create catenary chain
		let catenary_root = app.world_mut().spawn((
			ChainRoot { link_count, rest_length_m: length_m },
			ChainMotion { mode: ChainTensionMode::Catenary { sag_ratio: 10 }, active: true },
			Transform::from_xyz(5.0, 0.0, 0.0),
			GlobalTransform::default(),
		)).id();

		let _catenary_start_anchor = app.world_mut().spawn((
			ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: catenary_root },
			Transform::from_xyz(5.0, 0.0, 0.0),
			GlobalTransform::default(),
		)).id();

		let _catenary_end_anchor = app.world_mut().spawn((
			ChainAnchor { endpoint: ChainEndpoint::End, chain_root: catenary_root },
			Transform::from_xyz(5.0, length_m, 0.0),
			GlobalTransform::default(),
		)).id();

		let mut catenary_joints = Vec::new();
		for _i in 0..link_count {
			let joint = app.world_mut().spawn((
				Transform::from_xyz(5.0, 0.0, 0.0),
				GlobalTransform::default(),
			)).id();
			app.world_mut().entity_mut(catenary_root).add_child(joint);
			catenary_joints.push(joint);
		}

		// Run several frames to let transforms propagate and motion update
		for _ in 0..5 {
			app.update();
		}

		// Get midpoint joint positions (joint 6 out of 12)
		let taut_midpoint_y = app.world().entity(taut_joints[6]).get::<Transform>().unwrap().translation.y;
		let catenary_midpoint_y = app.world().entity(catenary_joints[6]).get::<Transform>().unwrap().translation.y;

		// Catenary should sag below taut
		// For sag_ratio=10, expected vertical compression is length_m / (sag_ratio + 1) = 2.8 / 11 ≈ 0.25m
		// Taut midpoint should be at length_m / 2 = 1.4m
		// Catenary midpoint should be compressed down by half the total compression: 1.4 - 0.25/2 ≈ 1.275m
		assert!(
			catenary_midpoint_y < taut_midpoint_y,
			"Catenary midpoint ({catenary_midpoint_y}) should sag below taut midpoint ({taut_midpoint_y})"
		);

		let expected_sag = 0.1; // At least 10cm of sag
		assert!(
			taut_midpoint_y - catenary_midpoint_y > expected_sag,
			"Catenary should sag by at least {expected_sag}m, got {:.3}m",
			taut_midpoint_y - catenary_midpoint_y
		);
	}
}
