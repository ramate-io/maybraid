//! Single-anchor jungle growth: inner mass + frond crown and Buddha's-hand foliage.

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::frond::FrondCrown;
use chico_ball_components::tuft::BuddhaHandTuft;
use procedural_common::NoiseParams;
use render_item::{CascadeChunk, RenderItem};

use super::config::JungleGrowthShape;

/// Secondary foliage cluster at one canopy anchor ([#226](https://github.com/ramate-io/maybraid/issues/226)).
///
/// The composing tree chooses **where** to place instances; this type builds and spawns the
/// inner dirt/wood mass plus a **frond crown** (outward arching shoots) mixed with a central
/// **Buddha's-hand tuft** (upward fingers at the anchor).
#[derive(Component, Clone)]
pub struct JungleGrowth<BodyM, BodyS, FoliageM, FoliageS>
where
	BodyM: Material,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>>,
	FoliageM: Material,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>>,
{
	pub shape: JungleGrowthShape,
	pub body_noise: NoiseParams,
	pub foliage_noise: NoiseParams,
	pub body_material: BodyS,
	pub foliage_material: FoliageS,
	pub(crate) __marker: PhantomData<fn() -> (BodyM, FoliageM)>,
}

impl<BodyM, BodyS, FoliageM, FoliageS> Default for JungleGrowth<BodyM, BodyS, FoliageM, FoliageS>
where
	BodyM: Material,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>> + Default,
	FoliageM: Material,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>> + Default,
{
	fn default() -> Self {
		Self {
			shape: JungleGrowthShape::default(),
			body_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			body_material: BodyS::default(),
			foliage_material: FoliageS::default(),
			__marker: PhantomData,
		}
	}
}

impl<BodyM, BodyS, FoliageM, FoliageS> JungleGrowth<BodyM, BodyS, FoliageM, FoliageS>
where
	BodyM: Material,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>> + Default,
	FoliageM: Material,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>> + Default,
{
	pub fn inner_ball(&self) -> ChicoBall<BodyM, BodyS> {
		let mut ball = self
			.body_noise
			.with_seed(self.shape.seed)
			.build_scalar::<ChicoBall<BodyM, BodyS>>();
		ball.material = self.body_material.clone();
		ball
	}

	pub fn frond_crown(&self) -> FrondCrown<FoliageM, FoliageS> {
		FrondCrown::from_shape(
			self.shape.frond_shape(&self.foliage_noise),
			self.foliage_material.clone(),
		)
	}

	pub fn buddha_hand(&self) -> BuddhaHandTuft<FoliageM, FoliageS> {
		BuddhaHandTuft::from_shape(
			self.shape.buddha_hand_shape(&self.foliage_noise),
			self.foliage_material.clone(),
		)
	}

	/// Arching frond crown mesh at unit scale (apply [`JungleGrowthShape::local_frond_transform`] scale).
	pub fn build_frond_mesh(&self) -> Mesh {
		self.frond_crown().build_mesh(1.0)
	}

	fn spawn_parts_under(
		&self,
		commands: &mut Commands,
		assembly: Entity,
		cascade_chunk: &CascadeChunk,
	) where
		BodyM: Send + Sync + 'static,
		BodyS: Send + Sync + 'static,
		FoliageM: Send + Sync + 'static,
		FoliageS: Send + Sync + 'static,
	{
		let _ = self.inner_ball().spawn_render_items_under(
			commands,
			cascade_chunk,
			self.shape.local_body_transform(),
			Some(assembly),
		);
		let _ = self.frond_crown().spawn_render_items_under(
			commands,
			cascade_chunk,
			self.shape.local_frond_transform(),
			Some(assembly),
		);
		let _ = self.buddha_hand().spawn_render_items_under(
			commands,
			cascade_chunk,
			self.shape.local_buddha_transform(),
			Some(assembly),
		);
	}

	/// Spawn body + foliage under one assembly root at `transform`.
	///
	/// Uniform scale on `transform` is the anchor node radius. All parts are [`ChildOf`] the
	/// assembly root with local transforms so the cluster repositions as one unit.
	pub fn spawn_at(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity>
	where
		BodyM: Send + Sync + 'static,
		BodyS: Send + Sync + 'static,
		FoliageM: Send + Sync + 'static,
		FoliageS: Send + Sync + 'static,
	{
		let node_radius = transform
			.scale
			.x
			.abs()
			.max(transform.scale.y.abs())
			.max(transform.scale.z.abs())
			.max(1e-8);

		let assembly = commands
			.spawn((
				self.clone(),
				cascade_chunk.clone(),
				Transform {
					translation: transform.translation,
					rotation: transform.rotation,
					scale: Vec3::splat(node_radius),
				},
				Visibility::default(),
			))
			.id();

		self.spawn_parts_under(commands, assembly, cascade_chunk);

		vec![assembly]
	}
}

impl<BodyM, BodyS, FoliageM, FoliageS> RenderItem for JungleGrowth<BodyM, BodyS, FoliageM, FoliageS>
where
	BodyM: Material + Send + Sync + 'static,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>> + Send + Sync + 'static + Default,
	FoliageM: Material + Send + Sync + 'static,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>> + Send + Sync + 'static + Default,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.spawn_at(commands, cascade_chunk, transform)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn foliage_components_use_shape_noise() -> Result<()> {
		let mut growth = JungleGrowth::<
			StandardMaterial,
			MeshMaterial3d<StandardMaterial>,
			StandardMaterial,
			MeshMaterial3d<StandardMaterial>,
		>::default();
		growth.shape.seed = 17;
		growth.foliage_noise = NoiseParams::from_scalar(0.0, 4.25, 0.09, 1);
		let crown = growth.frond_crown();
		assert_eq!(crown.shape.seed, 17_i32.wrapping_add(31));
		let tuft = growth.buddha_hand();
		assert_eq!(tuft.shape.seed, 17_i32.wrapping_add(31));
		assert!((tuft.shape.noise_frequency - 4.25).abs() < 1e-5);
		assert!((tuft.shape.noise_amplitude - 0.09).abs() < 1e-5);
		Ok(())
	}
}
