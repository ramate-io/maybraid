//! Crook-cylinder stick rule (braid-oak style) for stalk and limbs ([#254](https://github.com/ramate-io/maybraid/issues/254)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::render::stick::StickRenderRule;
use chico_sbs_geometry::{BallStickSegment, StorybookTreeChain, StorybookTreePhase};
use chico_stick_components::chico_crook_stick::ChicoCrookStick;
use procedural_common::{NoiseConfig, NoiseParams};

const STALK_BEND_STRENGTH: f32 = 10.0;
const BRANCH_BEND_STRENGTH: f32 = 16.0;
const BEND_STRENGTH_NOISE_GAIN: f32 = 0.40;
const MIN_BEND_STRENGTH: f32 = 4.0;

fn segment_key(segment: &BallStickSegment<'_>) -> u32 {
	(segment.start.position.x.to_bits() as u32)
		.wrapping_add(segment.end.position.y.to_bits().rotate_left(3))
		.wrapping_add(segment.end.position.z.to_bits().rotate_left(7))
}

#[derive(Clone)]
pub(crate) struct RorysHeadTrainedStickRule<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>>,
{
	pub stick_surface_noise: NoiseParams,
	pub stick_material: StickS,
	pub(crate) __marker: PhantomData<fn() -> StickM>,
}

impl<StickM, StickS> RorysHeadTrainedStickRule<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>>,
{
	fn bend_strength(
		&self,
		segment: &BallStickSegment<'_>,
		parent_hysteresis: &StorybookTreeChain,
	) -> f32 {
		let base = if matches!(parent_hysteresis.phase, StorybookTreePhase::Stalk(_)) {
			STALK_BEND_STRENGTH
		} else {
			BRANCH_BEND_STRENGTH
		};

		let mid = (segment.start.position + segment.end.position) * 0.5;
		let seed = self.stick_surface_noise.seed
			+ segment.start.position.length() as i32
			+ segment.end.position.length() as i32;
		let noise = NoiseConfig::new(self.stick_surface_noise.with_seed(seed));
		let n = noise.sample_3d(mid).clamp(-1.0, 1.0);
		(base * (1.0 + BEND_STRENGTH_NOISE_GAIN * n)).max(MIN_BEND_STRENGTH)
	}
}

impl<StickM, StickS> StickRenderRule<ChicoCrookStick<StickM, StickS>, StorybookTreeChain>
	for RorysHeadTrainedStickRule<StickM, StickS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Send + Sync + 'static + Default,
{
	fn stick_render_item_for(
		&self,
		segment: &BallStickSegment<'_>,
		parent_hysteresis: &StorybookTreeChain,
		_child_hysteresis: &StorybookTreeChain,
	) -> Option<ChicoCrookStick<StickM, StickS>> {
		Some(ChicoCrookStick::new(
			self.bend_strength(segment, parent_hysteresis),
			segment_key(segment),
			self.stick_material.clone(),
		))
	}
}
