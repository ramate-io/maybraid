//! **Chico crook stick**: plain [`CrookCylinder`](chico_sdf::CrookCylinder) along +Y for ball-stick segment meshes.
//!
//! Unlike [`ChicoStick`](crate::chico_stick::ChicoStick), this uses a bent centerline without surface noise.
//!
//! [`ChicoCrookStick::bend_strength`] is the intended **world** lateral XZ displacement per unit Y. The scale trick
//! (thin SDF radii + amplified XZ entity scale) is applied internally so callers only set strength.

pub mod render_item_plugin;

use std::f32::consts::TAU;
use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sdf::CrookCylinder;
use render_item::{mesh::handle::Cached, CascadeChunk, RenderItem};

/// Fixed SDF bend magnitudes (small; visible curvature comes from XZ mesh scale).
const SDF_BEND_X: f32 = 0.12;
const SDF_BEND_Z: f32 = 0.08;
/// Reference lateral offset at unit height with [`SDF_BEND_X`] and XZ scale 1.
const SDF_BEND_REFERENCE: f32 = SDF_BEND_X;

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
			bend_strength: bend_strength.max(0.0),
			segment_key,
			material,
			__marker: PhantomData,
		}
	}

	fn xz_crook_scale(&self) -> f32 {
		(self.bend_strength / SDF_BEND_REFERENCE).max(1.0)
	}

	fn unit_taper_radii(&self) -> (f32, f32) {
		(0.5, 0.42)
	}

	fn sdf_radii(&self) -> (f32, f32) {
		let s = self.xz_crook_scale().max(1e-4);
		let (base, top) = self.unit_taper_radii();
		(base / s, top / s)
	}

	fn bend_phases(&self) -> (f32, f32) {
		let k = self.segment_key;
		let t0 = k as f32 / u32::MAX as f32;
		let t1 = k.wrapping_mul(2_654_435_761) as f32 / u32::MAX as f32;
		(t0 * TAU, t1 * TAU)
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
		node_radius * 0.5
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
	fn xz_scale_tracks_bend_strength() {
		let lo = ChicoCrookStick::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::new(
			0.12,
			0,
			MeshMaterial3d::<StandardMaterial>::default(),
		);
		let hi = ChicoCrookStick::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::new(
			0.36,
			0,
			MeshMaterial3d::<StandardMaterial>::default(),
		);
		assert!(lo.crook_cylinder().base_radius > hi.crook_cylinder().base_radius);
	}

	#[test]
	fn visible_radius_matches_node_radius_for_unit_taper() {
		let stick = ChicoCrookStick::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::new(
			0.24,
			1,
			MeshMaterial3d::<StandardMaterial>::default(),
		);
		let node_r = 0.8;
		assert!((stick.visible_base_radius(node_r) - node_r * 0.5).abs() < 1e-5);
	}
}
