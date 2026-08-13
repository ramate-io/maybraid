//! Ground radial shoot assembly with stick and foliage render.

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderHelper;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::render::tuft::TuftRenderHelper;
use chico_sbs_geometry::{BallStickChain, HighBushChain};
use procedural_common::NoiseParams;
use render_item::{CascadeChunk, RenderItem};

use super::canopy::{HighBushSplayCanopyRule, HighBushTuftCanopyRule};
use super::config::{HighBushFoliageStyle, HighBushShootsShape};
use super::preset::apply_common_high_bush_preset;
use super::stick::HighBushStickRule;

/// Trunkless upward radial shoots from a near-ground anchor ([#225](https://github.com/ramate-io/maybraid/issues/225)).
#[derive(Component, Clone)]
pub struct HighBushShoots<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>>,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub shape: HighBushShootsShape,
	pub stick_surface_noise: NoiseParams,
	pub leaf_surface_noise: NoiseParams,
	pub stick_material: StickS,
	pub leaf_material: LeafS,
	pub(crate) __marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS> Default for HighBushShoots<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Default,
{
	fn default() -> Self {
		Self {
			shape: HighBushShootsShape::default(),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, LeafM, LeafS> HighBushShoots<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Default,
{
	/// Shape snapshot with Common High Bush preset applied (safe after CLI parse).
	pub fn shape_for_render(&self) -> HighBushShootsShape {
		let mut shape = self.shape.clone();
		apply_common_high_bush_preset(&mut shape);
		shape
	}

	pub fn build_chain(&self) -> BallStickChain<HighBushChain> {
		self.shape_for_render().build_chain()
	}

	/// Spawn one bush from an authored shape without merging the Common High Bush preset.
	pub fn spawn_from_shape(
		shape: HighBushShootsShape,
		stick_surface_noise: NoiseParams,
		leaf_surface_noise: NoiseParams,
		stick_material: StickS,
		leaf_material: LeafS,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity>
	where
		StickM: Material + Send + Sync + 'static,
		StickS: Clone + Into<MeshMaterial3d<StickM>> + Send + Sync + 'static,
		LeafM: Material + Send + Sync + 'static,
		LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
	{
		let root = commands
			.spawn((
				Self {
					shape: shape.clone(),
					stick_surface_noise,
					leaf_surface_noise,
					stick_material: stick_material.clone(),
					leaf_material: leaf_material.clone(),
					__marker: PhantomData,
				},
				cascade_chunk.clone(),
				transform,
				Visibility::default(),
			))
			.id();
		let chain = shape.build_chain();

		let stick_rule = HighBushStickRule::<StickM, StickS> {
			surface_noise: stick_surface_noise,
			stick_material,
			__marker: PhantomData,
		};
		StickRenderHelper::new(chain.clone(), stick_rule).spawn_render_items_under(
			commands,
			cascade_chunk,
			Transform::IDENTITY,
			Some(root),
		);

		match shape.foliage_style {
			// Ball kits have no dedicated RenderItem path yet — same plane-splay canopy.
			HighBushFoliageStyle::PlaneSplay
			| HighBushFoliageStyle::CheapBall
			| HighBushFoliageStyle::LayeredBall => {
				let mut leaf_splay = PlaneSplay::<LeafM, LeafS>::default();
				leaf_splay.material = leaf_material.clone();
				let leaf_rule = HighBushSplayCanopyRule::<LeafM, LeafS> {
					leaf_splay,
					leaf_radius_world: shape.leaf_radius_world(),
				};
				BallRenderHelper::new(chain, leaf_rule).spawn_render_items_under(
					commands,
					cascade_chunk,
					Transform::IDENTITY,
					Some(root),
				);
			}
			HighBushFoliageStyle::Tuft => {
				let tuft_rule = HighBushTuftCanopyRule::<LeafM, LeafS> {
					tuft_world_scale: shape.leaf_radius_world(),
					leaf_surface_noise,
					leaf_material,
					__marker: PhantomData,
				};
				TuftRenderHelper::new(chain, tuft_rule).spawn_render_items_under(
					commands,
					cascade_chunk,
					Transform::IDENTITY,
					Some(root),
				);
			}
		}

		vec![root]
	}
}

impl<StickM, StickS, LeafM, LeafS> RenderItem for HighBushShoots<StickM, StickS, LeafM, LeafS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Default + Send + Sync + 'static,
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Default + Send + Sync + 'static,
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
		let shape = self.shape_for_render();
		let chain = shape.build_chain();

		let stick_rule = HighBushStickRule::<StickM, StickS> {
			surface_noise: self.stick_surface_noise,
			stick_material: self.stick_material.clone(),
			__marker: PhantomData,
		};
		StickRenderHelper::new(chain.clone(), stick_rule).spawn_render_items_under(
			commands,
			cascade_chunk,
			Transform::IDENTITY,
			Some(root),
		);

		match shape.foliage_style {
			HighBushFoliageStyle::PlaneSplay
			| HighBushFoliageStyle::CheapBall
			| HighBushFoliageStyle::LayeredBall => {
				let mut leaf_splay = PlaneSplay::<LeafM, LeafS>::default();
				leaf_splay.material = self.leaf_material.clone();
				let leaf_rule = HighBushSplayCanopyRule::<LeafM, LeafS> {
					leaf_splay,
					leaf_radius_world: shape.leaf_radius_world(),
				};
				BallRenderHelper::new(chain, leaf_rule).spawn_render_items_under(
					commands,
					cascade_chunk,
					Transform::IDENTITY,
					Some(root),
				);
			}
			HighBushFoliageStyle::Tuft => {
				let tuft_rule = HighBushTuftCanopyRule::<LeafM, LeafS> {
					tuft_world_scale: shape.leaf_radius_world(),
					leaf_surface_noise: self.leaf_surface_noise,
					leaf_material: self.leaf_material.clone(),
					__marker: PhantomData,
				};
				TuftRenderHelper::new(chain, tuft_rule).spawn_render_items_under(
					commands,
					cascade_chunk,
					Transform::IDENTITY,
					Some(root),
				);
			}
		}

		vec![root]
	}
}
