//! Tuft component ([RFC-183 §3.1.2.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/06-tufts/README.md)).
//!
//! A tuft is a cluster of vertical noisy capsules: several spikes radiate from a shared base in the
//! **+Y** direction of local space, with procedural sway on each spike.

pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sdf::sdf::Sdf;
use chunk::cascade::CascadeChunk;
use procedural_common::{
	sdf_band_margin, FromScalarNoise, NoiseConfig, NoiseParams, NUMERIC_SURFACE_EPSILON,
};
use render_item::{
	mesh::{handle::Cached, IdentifiedMesh, MeshId},
	NormalizeChunk, RenderItem,
};

/// Unit-space tuft SDF: union of noisy vertical capsules on a ring.
#[derive(Clone, Debug)]
pub struct TuftCluster {
	pub spike_count: u32,
	pub spike_height: f32,
	pub spike_radius: f32,
	pub cluster_radius: f32,
	pub seed: i32,
	pub sway: NoiseParams,
}

impl Default for TuftCluster {
	fn default() -> Self {
		Self {
			spike_count: 6,
			spike_height: 0.75,
			spike_radius: 0.04,
			cluster_radius: 0.22,
			seed: 0,
			sway: NoiseParams {
				frequency: 6.0,
				amplitude: 0.06,
				octaves: 1,
				..Default::default()
			},
		}
	}
}

impl TuftCluster {
	fn capsule_distance(p: Vec3, height: f32, radius: f32) -> f32 {
		let y = p.y.clamp(0.0, height);
		let closest = Vec3::new(0.0, y, 0.0);
		(p - closest).length() - radius
	}
}

impl Sdf for TuftCluster {
	fn distance(&self, p: Vec3) -> f32 {
		let noise = NoiseConfig::new(self.sway.with_seed(self.seed));
		let count = self.spike_count.max(1);
		let mut min_dist = f32::MAX;

		for i in 0..count {
			let fi = i as f32;
			let angle = fi * std::f32::consts::TAU / count as f32;
			let offset =
				Vec3::new(angle.cos() * self.cluster_radius, 0.0, angle.sin() * self.cluster_radius);

			let mut spike_p = p - offset;
			let sway = noise.sample_3d(spike_p.x, spike_p.y, spike_p.z);
			spike_p.x += sway;
			spike_p.z += sway;

			let d = Self::capsule_distance(spike_p, self.spike_height, self.spike_radius);
			min_dist = min_dist.min(d);
		}

		min_dist
	}
}

impl IdentifiedMesh for TuftCluster {
	fn id(&self) -> MeshId {
		MeshId::new(format!("{self:?}"))
	}
}

impl NormalizeChunk for TuftCluster {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		let m = sdf_band_margin(&self.sway);
		let mu_xz = self.cluster_radius + self.spike_radius + m;
		let mu_y = self.spike_height + self.spike_radius + m + NUMERIC_SURFACE_EPSILON;
		CascadeChunk::unit_center_chunk_with_mu_xz_y(mu_xz, mu_y).with_res_2(cascade_chunk.res_2)
	}
}

/// [`StandardMaterial`] tuft using explicit mesh materials (common default).
pub type ChicoTuftStd = ChicoTuft<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// Marker plus embedded material for a single tuft instance at a ball-stick joint.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ChicoTuft<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub spike_count: u32,
	pub spike_height: f32,
	pub spike_radius: f32,
	pub cluster_radius: f32,
	pub seed: i32,
	pub sway: NoiseParams,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for ChicoTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self {
			spike_count: TuftCluster::default().spike_count,
			spike_height: TuftCluster::default().spike_height,
			spike_radius: TuftCluster::default().spike_radius,
			cluster_radius: TuftCluster::default().cluster_radius,
			seed: 0,
			sway: TuftCluster::default().sway,
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> FromScalarNoise for ChicoTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn from_scalar(seed_scalar: f32, frequency: f32, amplitude: f32, octaves: u32) -> Self {
		Self {
			seed: seed_scalar as i32,
			sway: NoiseParams::from_scalar(seed_scalar, frequency, amplitude, octaves),
			..Self::default()
		}
	}
}

impl<M: Material, S> ChicoTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub fn tuft_cluster(&self) -> TuftCluster {
		TuftCluster {
			spike_count: self.spike_count,
			spike_height: self.spike_height,
			spike_radius: self.spike_radius,
			cluster_radius: self.cluster_radius,
			seed: self.seed,
			sway: self.sway,
		}
	}
}

impl<M: Material, S> RenderItem for ChicoTuft<M, S>
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
		// Unit tuft is authored with spikes rising from local y = 0; anchor is the xz centroid at the base.
		let local_offset =
			Vec3::new(-0.5 * transform.scale.x, 0.0, -0.5 * transform.scale.z);
		let translation = transform.translation + transform.rotation * local_offset;
		let mesh_material: MeshMaterial3d<M> = self.material.clone().into();

		vec![commands
			.spawn((
				self.clone(),
				Cached::new(self.tuft_cluster()),
				cascade_chunk.clone(),
				transform.with_translation(translation),
				mesh_material,
			))
			.id()]
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn same_seed_same_distance() -> Result<()> {
		let a = TuftCluster { seed: 42, ..TuftCluster::default() };
		let b = TuftCluster { seed: 42, ..TuftCluster::default() };
		let p = Vec3::new(0.1, 0.4, -0.2);
		assert_eq!(a.distance(p), b.distance(p));
		Ok(())
	}

	#[test]
	fn different_seed_can_differ() -> Result<()> {
		let a = TuftCluster { seed: 1, ..TuftCluster::default() };
		let b = TuftCluster { seed: 2, ..TuftCluster::default() };
		let p = Vec3::new(0.05, 0.5, 0.05);
		assert_ne!(a.distance(p), b.distance(p));
		Ok(())
	}
}
