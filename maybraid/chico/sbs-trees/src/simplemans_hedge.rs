//! **Simpleman's Hedge** — a dense, low ball-and-plane-splay mat over a small ground area
//! ([#320](https://github.com/ramate-io/maybraid/issues/320), [RFC §3.1.7.16](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/16-simpleman-s-hedge/README.md)).

pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use clap::Args;
use procedural_common::{FromScalarNoise, NoiseConfig, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::SkippedLeafMeshMaterial;

/// Typical [`StandardMaterial`] Simpleman's Hedge using CLI-skipped leaf handles.
pub type SimplemansHedgeStd =
	SimplemansHedge<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;

/// A patch of low hedge clumps scattered over an XZ footprint.
#[derive(Component, Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct SimplemansHedge<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	/// Number of ball-and-splay clumps scattered over the hedge.
	#[arg(long, default_value_t = 9)]
	pub clump_count: u32,

	/// Hedge band height `H` (m).
	#[arg(long, default_value_t = 1.2)]
	pub height: f32,

	/// Square hedge footprint side length (m) the clumps scatter within.
	#[arg(long, default_value_t = 1.2)]
	pub footprint_xz: f32,

	/// Fill density in `0.0..1.0`; modulates clump size and plate reach.
	#[arg(long, default_value_t = 0.5)]
	pub density: f32,

	/// Deterministic surface seed.
	#[arg(long, default_value_t = 0)]
	pub seed: u32,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	#[arg(skip)]
	__marker: PhantomData<fn() -> LeafM>,
}

impl<LeafM, LeafS> Default for SimplemansHedge<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			clump_count: 9,
			height: 1.2,
			footprint_xz: 1.2,
			density: 0.5,
			seed: 0,
			leaf_material: LeafS::default(),
			__marker: PhantomData,
		}
	}
}

impl<LeafM, LeafS> SimplemansHedge<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Default,
{
	pub fn new(
		height: f32,
		footprint_xz: f32,
		density: f32,
		seed: u32,
		leaf_material: LeafS,
	) -> Self {
		Self {
			clump_count: 9,
			height,
			footprint_xz,
			density,
			seed,
			leaf_material,
			__marker: PhantomData,
		}
	}

	/// RFC `hedge_radius = 0.08 * H`, widened slightly by authored density.
	pub fn hedge_radius(&self) -> f32 {
		let density = self.density.clamp(0.0, 1.0);
		0.08 * self.height.max(0.1) * (0.9 + 0.35 * density)
	}

	fn clump_height(&self) -> f32 {
		self.height.max(0.1) * 0.42
	}

	fn clump_radius(&self) -> f32 {
		let density = self.density.clamp(0.0, 1.0);
		let footprint_radius = self.footprint_xz.max(0.1) * (0.20 + 0.08 * density);
		footprint_radius.max(self.hedge_radius() * 2.0)
	}

	/// Deterministic patch-local clump anchors, scattered within the XZ footprint.
	pub fn clump_anchors(&self) -> Vec<Vec3> {
		let config = NoiseConfig::new(NoiseParams::from_scalar(self.seed as f32, 1.0, 1.0, 1));
		let half = (self.footprint_xz * 0.18).max(0.0);
		(0..self.clump_count)
			.map(|i| {
				// Non-integer lanes keep samples off gradient-noise lattice points.
				let lane = (i as f32 + 0.5) * 3.7;
				let x = config.sample_range_f32_4d(-half, half, lane, 0.0, 0.0, 1.0);
				let z = config.sample_range_f32_4d(-half, half, lane, 0.0, 0.0, 2.0);
				Vec3::new(x, 0.0, z)
			})
			.collect()
	}

	/// Grounded [`ChicoBall`] transform for clump `index` at `anchor`.
	fn clump_ball_transform(&self, index: u32, anchor: Vec3) -> Transform {
		let radius = self.clump_radius();
		let height = self.clump_height();
		let jitter = 0.95 + 0.06 * (index % 3) as f32;
		Transform {
			translation: anchor + Vec3::new(0.0, height, 0.0),
			rotation: Quat::IDENTITY,
			scale: Vec3::new(radius * 2.4 * jitter, height, radius * 1.9 * jitter),
		}
	}

	/// Grounded [`PlaneSplay`] transform for clump `index` at `anchor`.
	fn clump_splay_transform(&self, index: u32, anchor: Vec3) -> Transform {
		let radius = self.clump_radius();
		let core_radius = radius * 0.55;
		let vertical_scale = (self.clump_height() / (core_radius * 2.0)).max(0.2);
		let horizontal_scale = 1.45 + 0.10 * (index % 2) as f32;
		Transform {
			translation: anchor + Vec3::new(0.0, core_radius * vertical_scale, 0.0),
			rotation: Quat::IDENTITY,
			scale: Vec3::new(horizontal_scale, vertical_scale, horizontal_scale * 0.9),
		}
	}

	fn clump_ball(&self, index: u32) -> ChicoBall<LeafM, LeafS> {
		let mut ball = ChicoBall::from_scalar(NoiseParams::from_scalar(
			self.seed.wrapping_add((index + 1) * 131) as f32,
			1.0,
			0.03,
			1,
		));
		ball.material = self.leaf_material.clone();
		ball
	}

	fn clump_splay(&self, index: u32) -> PlaneSplay<LeafM, LeafS> {
		let radius = self.clump_radius();
		let mut splay = PlaneSplay::<LeafM, LeafS>::default();
		splay.core_radius = radius * 0.55;
		splay.leaf_disc_radius = radius * (0.75 + 0.05 * (index % 3) as f32);
		splay.material = self.leaf_material.clone();
		splay
	}
}

impl<LeafM, LeafS> RenderItem for SimplemansHedge<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Default + Send + Sync + 'static,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let root = commands
			.spawn((self.clone(), cascade_chunk.clone(), transform, Visibility::default()))
			.id();

		for (index, anchor) in self.clump_anchors().into_iter().enumerate() {
			let index = index as u32;
			self.clump_ball(index).spawn_render_items_under(
				commands,
				cascade_chunk,
				self.clump_ball_transform(index, anchor),
				Some(root),
			);
			self.clump_splay(index).spawn_render_items_under(
				commands,
				cascade_chunk,
				self.clump_splay_transform(index, anchor),
				Some(root),
			);
		}

		vec![root]
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	fn hedge(seed: u32) -> SimplemansHedgeStd {
		SimplemansHedgeStd { seed, ..SimplemansHedgeStd::default() }
	}

	#[test]
	fn hedge_radius_follows_rfc() -> Result<()> {
		let h = SimplemansHedgeStd { height: 2.0, density: 0.0, ..SimplemansHedgeStd::default() };
		assert!((h.hedge_radius() - 0.144).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn anchors_stay_within_patch_footprint() -> Result<()> {
		let hedge = hedge(7);
		let anchors = hedge.clump_anchors();
		assert_eq!(anchors.len(), hedge.clump_count as usize);
		let half = hedge.footprint_xz * 0.5;
		for anchor in &anchors {
			assert!(anchor.x.abs() <= half);
			assert!(anchor.z.abs() <= half);
			assert_eq!(anchor.y, 0.0);
		}
		Ok(())
	}

	#[test]
	fn anchors_are_deterministic_per_seed_and_scattered() -> Result<()> {
		let anchors = hedge(7).clump_anchors();
		assert_eq!(anchors, hedge(7).clump_anchors());
		let distinct = anchors
			.iter()
			.enumerate()
			.all(|(i, a)| anchors.iter().skip(i + 1).all(|b| a.distance(*b) > 1e-4));
		assert!(distinct, "expected scattered anchors, got {anchors:?}");
		Ok(())
	}

	#[test]
	fn clump_shapes_vary_by_seed_and_size() -> Result<()> {
		let hedge = hedge(7);
		let a = hedge.clump_ball(0);
		let b = hedge.clump_ball(1);
		assert_ne!(a.seed_scalar, b.seed_scalar);
		assert_eq!(a.frequency, b.frequency);
		assert_eq!(a.octaves, b.octaves);
		assert_ne!(
			hedge.clump_ball_transform(0, Vec3::ZERO).scale.x,
			hedge.clump_ball_transform(1, Vec3::ZERO).scale.x
		);
		Ok(())
	}

	#[test]
	fn clumps_sit_on_ground() -> Result<()> {
		let hedge = hedge(7);
		let anchor = Vec3::new(0.2, 0.0, -0.1);
		let ball = hedge.clump_ball_transform(0, anchor);
		let center_y = ball.translation.y - ball.scale.y * 0.5;
		let ball_base_y = center_y - ball.scale.y * 0.5;
		let splay = hedge.clump_splay_transform(0, anchor);
		let splay_base_y = splay.translation.y - hedge.clump_splay(0).core_radius * splay.scale.y;
		assert!(ball_base_y.abs() < 1e-4, "expected ball base at y=0, got {ball_base_y}");
		assert!(splay_base_y.abs() < 1e-4, "expected splay base at y=0, got {splay_base_y}");
		Ok(())
	}

	#[test]
	fn clumps_are_low_dense_and_overlapping() -> Result<()> {
		let hedge = hedge(7);
		assert_eq!(hedge.clump_count, 9);
		assert!(hedge.clump_height() < hedge.height * 0.5);

		let anchors = hedge.clump_anchors();
		let max_anchor_distance = anchors.iter().fold(0.0_f32, |max_distance, a| {
			anchors.iter().fold(max_distance, |inner_max, b| inner_max.max(a.distance(*b)))
		});
		let min_ball_width = (0..hedge.clump_count)
			.map(|index| hedge.clump_ball_transform(index, Vec3::ZERO).scale.x)
			.fold(f32::INFINITY, f32::min);
		assert!(
			max_anchor_distance < min_ball_width,
			"expected dense overlapping hedge clumps, max anchor distance {max_anchor_distance} >= min ball width {min_ball_width}"
		);
		Ok(())
	}
}
