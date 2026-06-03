use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::render::stick::StickRenderRule;
use chico_sbs_geometry::{BallStickSegment, KamakuraTorchChain, StorybookTreePhase};
use chico_stick_components::chico_stick::ChicoStick;
use procedural_common::NoiseParams;

#[derive(Clone)]
pub(crate) struct KamakuraTorchStickRule<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>>,
{
	pub surface_noise: NoiseParams,
	pub stick_material: StickS,
	pub(crate) __marker: PhantomData<fn() -> StickM>,
}

impl<StickM, StickS> StickRenderRule<ChicoStick<StickM, StickS>, KamakuraTorchChain>
	for KamakuraTorchStickRule<StickM, StickS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Send + Sync + 'static + Default,
{
	fn stick_render_item_for(
		&self,
		segment: &BallStickSegment<'_>,
		parent_hysteresis: &KamakuraTorchChain,
		_child_hysteresis: &KamakuraTorchChain,
	) -> Option<ChicoStick<StickM, StickS>> {
		let seed = self.surface_noise.seed
			+ segment.start.position.length() as i32
			+ segment.end.position.length() as i32;
		let mut stick =
			self.surface_noise.with_seed(seed).build_scalar::<ChicoStick<StickM, StickS>>();
		if matches!(parent_hysteresis.phase, StorybookTreePhase::Stalk(_)) {
			let (base_r, top_r) = ChicoStick::<StickM, StickS>::storybook_stalk_unit_taper();
			stick.segment_base_radius = base_r;
			stick.segment_top_radius = top_r;
			stick.cylinder_bounds_margin = (stick.amplitude * 4.0).max(0.15);
		}
		stick.material = self.stick_material.clone();
		Some(stick)
	}
}
