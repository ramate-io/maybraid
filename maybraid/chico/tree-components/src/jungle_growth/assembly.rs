//! Single-anchor jungle growth: inner mass + frond crown and Buddha's-hand foliage.

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::frond::FrondCrown;
use chico_ball_components::tuft::BuddhaHandTuft;
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use super::config::JungleGrowthShape;

/// Secondary foliage cluster at one canopy anchor ([#226](https://github.com/ramate-io/maybraid/issues/226)).
///
/// The composing tree chooses **where** to place instances; this type builds and spawns the
/// inner dirt/wood mass plus a **frond crown** (outward arching shoots) mixed with a central
/// **Buddha's-hand tuft** (upward fingers at the anchor).
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
		let mut crown = self
			.foliage_noise
			.with_seed(self.shape.seed)
			.build_scalar::<FrondCrown<FoliageM, FoliageS>>();
		crown.shape = self.shape.frond_shape();
		crown.material = self.foliage_material.clone();
		crown
	}

	pub fn buddha_hand(&self) -> BuddhaHandTuft<FoliageM, FoliageS> {
		let seed = self.shape.seed.wrapping_add(31);
		let mut tuft = self
			.foliage_noise
			.with_seed(seed)
			.build_scalar::<BuddhaHandTuft<FoliageM, FoliageS>>();
		tuft.shape = self.shape.buddha_hand_shape();
		tuft.material = self.foliage_material.clone();
		tuft
	}

	/// Arching frond crown mesh at [`JungleGrowthShape::foliage_world_scale`].
	pub fn build_frond_mesh(&self) -> Mesh {
		self.frond_crown()
			.build_mesh(self.shape.foliage_world_scale.max(1e-8))
	}

	/// Spawn body + foliage at `transform`.
	///
	/// Uniform scale on `transform` is treated as the anchor node radius for the inner ball.
	/// Frond crown and Buddha's-hand tuft are lifted along local +Y by [`JungleGrowthShape::frond_crown_lift`]
	/// and [`JungleGrowthShape::buddha_hand_lift`] (fractions of inner ball radius).
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
			scale: Vec3::splat(self.shape.ball_radius(node_radius)),
		};

		let ball_radius = self.shape.ball_radius(node_radius);
		let local_up = transform.rotation * Vec3::Y;

		let frond_transform = Transform {
			translation: transform.translation
				+ local_up * (ball_radius * self.shape.frond_crown_lift),
			rotation: transform.rotation,
			scale: Vec3::splat(self.shape.foliage_world_scale),
		};

		let buddha_scale = self.shape.foliage_world_scale * self.shape.buddha_hand_scale;
		let buddha_transform = Transform {
			translation: transform.translation
				+ local_up * (ball_radius * self.shape.buddha_hand_lift),
			rotation: transform.rotation,
			scale: Vec3::splat(buddha_scale),
		};

		let mut out = self
			.inner_ball()
			.spawn_render_items(commands, cascade_chunk, body_transform);
		out.extend(
			self.frond_crown()
				.spawn_render_items(commands, cascade_chunk, frond_transform),
		);
		out.extend(
			self.buddha_hand()
				.spawn_render_items(commands, cascade_chunk, buddha_transform),
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
