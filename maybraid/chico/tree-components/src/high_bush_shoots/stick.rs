use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::render::stick::StickRenderRule;
use chico_sbs_geometry::{BallStickSegment, HighBushChain};
use chico_stick_components::chico_stick::ChicoStick;
use procedural_common::NoiseParams;

#[derive(Clone)]
pub(crate) struct HighBushStickRule<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>>,
{
	pub surface_noise: NoiseParams,
	pub stick_material: StickS,
	pub(crate) __marker: PhantomData<fn() -> StickM>,
}

impl<StickM, StickS> StickRenderRule<ChicoStick<StickM, StickS>, HighBushChain>
	for HighBushStickRule<StickM, StickS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Default + Send + Sync + 'static,
{
	fn stick_render_item_for(
		&self,
		segment: &BallStickSegment<'_>,
		_parent_hysteresis: &HighBushChain,
		_child_hysteresis: &HighBushChain,
	) -> Option<ChicoStick<StickM, StickS>> {
		let seed = self.surface_noise.seed
			+ segment.start.position.length() as i32
			+ segment.end.position.length() as i32;
		let mut stick =
			self.surface_noise.with_seed(seed).build_scalar::<ChicoStick<StickM, StickS>>();
		stick.material = self.stick_material.clone();
		Some(stick)
	}
}
