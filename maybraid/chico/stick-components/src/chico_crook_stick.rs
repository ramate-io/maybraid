//! **Chico crook stick**: plain [`CrookCylinder`](chico_sdf::CrookCylinder) along +Y for ball-stick segment meshes.
//!
//! Unlike [`ChicoStick`](crate::chico_stick::ChicoStick), this uses a bent centerline without surface noise.
//!
//! [`ChicoCrookStick::bend_strength`] is the intended **world** lateral XZ displacement per unit Y. SDF radius is
//! **`1 / bend_strength`** (thin radii like `0.05` yield strong visible bends); XZ entity scale is
//! **`target_radius / sdf_radius`** so world girth stays at the unit taper target.

pub mod render_item_plugin;

use std::f32::consts::PI;
use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sdf::CrookCylinder;
use render_item::{mesh::handle::Cached, CascadeChunk, RenderItem};

/// Fixed SDF bend magnitudes (small; visible curvature comes from XZ mesh scale).
const SDF_BEND_X: f32 = 0.12;
const SDF_BEND_Z: f32 = 0.08;

/// Unit-stick base/top radii the mesh must hit in world space (`node_radius ×` these fractions).
/// Fixing these helpes with cahcing. You can apply scaling to get the effect.
const UNIT_TARGET_BASE_RADIUS: f32 = 0.5;
const UNIT_TARGET_TOP_RADIUS: f32 = 0.42;

/// [`StandardMaterial`] crook stick using explicit mesh materials.
pub type ChicoCrookStickStd = ChicoCrookStick<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// Stick marker plus embedded render material (via [`Into`]).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ChicoCrookStick<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	/// World lateral XZ displacement per unit Y along the segment.
	pub bend_strength: f32,
	/// Deterministic phase variation for this segment (typically hashed from segment geometry).
	pub segment_key: u32,
	/// Converts into [`MeshMaterial3d`] at spawn (see [`Into`]).
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> ChicoCrookStick<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	/// Build a crook stick at the given lateral bend strength (XZ units per unit Y).
	pub fn new(bend_strength: f32, segment_key: u32, material: S) -> Self {
		Self {
			bend_strength: bend_strength.max(1e-4),
			segment_key,
			material,
			__marker: PhantomData,
		}
	}

	/// SDF base radius inversely proportional to bend strength (`≈ 1 / strength`).
	fn sdf_base_radius(&self) -> f32 {
		1.0 / self.bend_strength
	}

	/// XZ scale restores unit taper girth: `target_radius / sdf_radius`.
	fn xz_crook_scale(&self) -> f32 {
		UNIT_TARGET_BASE_RADIUS / self.sdf_base_radius()
	}

	fn sdf_radii(&self) -> (f32, f32) {
		let base = self.sdf_base_radius();
		let top = base * (UNIT_TARGET_TOP_RADIUS / UNIT_TARGET_BASE_RADIUS);
		(base, top)
	}

	/// Joint-aligned bend phases: `0` or `π` so the centerline passes through both ball joints.
	fn bend_phases(&self) -> (f32, f32) {
		let phase_x = if self.segment_key & 1 == 0 { 0.0 } else { PI };
		let phase_z = if self.segment_key.rotate_left(7) & 1 == 0 { 0.0 } else { PI };
		(phase_x, phase_z)
	}

	/// Unit-height crook segment in local stick space.
	pub fn crook_cylinder(&self) -> CrookCylinder {
		let (base_r, top_r) = self.sdf_radii();
		let (phase_x, phase_z) = self.bend_phases();
		let mut cyl = CrookCylinder {
			base_radius: base_r,
			top_radius: top_r,
			y_min: 0.0,
			height: 1.0,
			bounds_margin: 0.0,
			bend_x: SDF_BEND_X,
			bend_z: SDF_BEND_Z,
			phase_x,
			phase_z,
		};
		let xz_pad = cyl.chunk_mu_xz_pad();
		cyl.bounds_margin = xz_pad.max(0.06);
		cyl
	}

	/// Effective world base radius: `node_radius * 0.5` at unit taper.
	pub fn visible_base_radius(&self, node_radius: f32) -> f32 {
		node_radius * UNIT_TARGET_BASE_RADIUS
	}
}

impl<M: Material, S> RenderItem for ChicoCrookStick<M, S>
where
	M: Send + Sync + 'static,
	S: Clone + Into<MeshMaterial3d<M>> + Send + Sync + 'static,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let s = self.xz_crook_scale();
		let mut mesh_transform = transform;
		mesh_transform.scale.x *= s;
		mesh_transform.scale.z *= s;

		let local_offset = Vec3::new(
			-0.5 * mesh_transform.scale.x,
			-0.5 * mesh_transform.scale.y,
			-0.5 * mesh_transform.scale.z,
		);
		let centroid_offset = mesh_transform.rotation * local_offset;
		let translation = mesh_transform.translation + centroid_offset;

		let mesh_material: MeshMaterial3d<M> = self.material.clone().into();

		vec![commands
			.spawn((
				self.clone(),
				Cached::new(self.crook_cylinder()),
				cascade_chunk.clone(),
				mesh_transform.with_translation(translation),
				mesh_material,
			))
			.id()]
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn stronger_bend_uses_thinner_sdf_radius() {
		let lo = ChicoCrookStick::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::new(
			8.0,
			0,
			MeshMaterial3d::<StandardMaterial>::default(),
		);
		let hi = ChicoCrookStick::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::new(
			20.0,
			0,
			MeshMaterial3d::<StandardMaterial>::default(),
		);
		assert!(lo.crook_cylinder().base_radius > hi.crook_cylinder().base_radius);
		assert!((hi.crook_cylinder().base_radius - 0.05).abs() < 1e-5);
	}

	#[test]
	fn xz_scale_times_sdf_base_matches_unit_target() {
		let stick = ChicoCrookStick::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::new(
			12.0,
			0,
			MeshMaterial3d::<StandardMaterial>::default(),
		);
		let base = stick.crook_cylinder().base_radius;
		let xz = UNIT_TARGET_BASE_RADIUS / base;
		assert!((xz * base - UNIT_TARGET_BASE_RADIUS).abs() < 1e-5);
		assert!((base - 1.0 / 12.0).abs() < 1e-5);
	}

	#[test]
	fn bend_phases_are_joint_aligned() {
		for key in [0_u32, 1, 42, 0xdead_beef] {
			let stick = ChicoCrookStick::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::new(
				12.0,
				key,
				MeshMaterial3d::<StandardMaterial>::default(),
			);
			let cyl = stick.crook_cylinder();
			assert!((cyl.phase_x.rem_euclid(PI)).abs() < 1e-5, "phase_x {}", cyl.phase_x);
			assert!((cyl.phase_z.rem_euclid(PI)).abs() < 1e-5, "phase_z {}", cyl.phase_z);
			assert!((cyl.centerline(0.0).x).abs() < 1e-5);
			assert!((cyl.centerline(0.0).z).abs() < 1e-5);
			assert!((cyl.centerline(1.0).x).abs() < 1e-5);
			assert!((cyl.centerline(1.0).z).abs() < 1e-5);
		}
	}

	#[test]
	fn visible_radius_matches_node_radius_for_unit_taper() {
		let stick = ChicoCrookStick::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::new(
			15.0,
			1,
			MeshMaterial3d::<StandardMaterial>::default(),
		);
		let node_r = 0.8;
		assert!((stick.visible_base_radius(node_r) - node_r * 0.5).abs() < 1e-5);
	}
}
