//! Single-anchor jungle growth: darker inner mass + drooping tuft foliage.

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::tuft::WeepingTuft;
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use super::config::JungleGrowthShape;

/// Secondary foliage cluster at one canopy anchor ([#226](https://github.com/ramate-io/maybraid/issues/226)).
///
/// The composing tree chooses **where** to place instances; this type only builds and spawns the
/// paired body (dirt / wood mass) and foliage (drooping tuft) meshes.
#[derive(Clone)]
pub struct JungleGrowth<BodyM, BodyS, FoliageM, FoliageS>
where
	BodyM: Material,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>>,
	FoliageM: Material,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>>,
{
	pub shape: JungleGrowthShape,
	pub body_noise: NoiseParams,
	pub tuft_noise: NoiseParams,
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
			tuft_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
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

	pub fn foliage_tuft(&self) -> WeepingTuft<FoliageM, FoliageS> {
		let mut tuft = self
			.tuft_noise
			.with_seed(self.shape.seed.wrapping_add(17))
			.build_scalar::<WeepingTuft<FoliageM, FoliageS>>();
		tuft.shape = self.shape.tuft.clone();
		tuft.shape.seed = self.shape.seed.wrapping_add(17);
		tuft.material = self.foliage_material.clone();
		tuft
	}

	/// Drooping strand mesh at [`JungleGrowthShape::tuft_world_scale`].
	pub fn build_foliage_mesh(&self) -> Mesh {
		self.foliage_tuft()
			.build_mesh(self.shape.tuft_world_scale.max(1e-8))
	}

	/// Spawn body + foliage at `transform`.
	///
	/// Uniform scale on `transform` is treated as the anchor node radius for the inner ball;
	/// tuft scale comes from [`JungleGrowthShape::tuft_world_scale`]. Rotation aims the tuft.
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

		let body_transform = Transform {
			translation: transform.translation,
			rotation: transform.rotation,
			scale: Vec3::splat(node_radius * self.shape.inner_ball_scale),
		};

		let tuft_transform = Transform {
			translation: transform.translation,
			rotation: transform.rotation,
			scale: Vec3::splat(self.shape.tuft_world_scale),
		};

		let mut out = self
			.inner_ball()
			.spawn_render_items(commands, cascade_chunk, body_transform);
		out.extend(
			self.foliage_tuft()
				.spawn_render_items(commands, cascade_chunk, tuft_transform),
		);
		out
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
