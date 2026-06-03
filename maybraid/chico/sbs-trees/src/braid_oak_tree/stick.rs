use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::render::stick::StickRenderRule;
use chico_sbs_geometry::{BallStickSegment, StorybookTreeChain, StorybookTreePhase};
use chico_stick_components::chico_crook_stick::ChicoCrookStick;
use procedural_common::{NoiseConfig, NoiseParams};

fn segment_key(segment: &BallStickSegment<'_>) -> u32 {
	(segment.start.position.x.to_bits() as u32)
		.wrapping_add(segment.end.position.y.to_bits().rotate_left(3))
		.wrapping_add(segment.end.position.z.to_bits().rotate_left(7))
}

#[derive(Clone)]
pub(crate) struct BraidOakTreeStickRule<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>>,
{
	pub stick_surface_noise: NoiseParams,
	pub stick_material: StickS,
	pub(crate) __marker: PhantomData<fn() -> StickM>,
}

impl<StickM, StickS> BraidOakTreeStickRule<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>>,
{
	fn bend_strength(
		&self,
		segment: &BallStickSegment<'_>,
		parent_hysteresis: &StorybookTreeChain,
	) -> f32 {
		let is_stalk = matches!(parent_hysteresis.phase, StorybookTreePhase::Stalk(_));
		let base = if is_stalk {
			10.0
		} else {
			14.0 + 10.0 * parent_hysteresis.ring_u.clamp(0.0, 1.0)
		};

		let mid = (segment.start.position + segment.end.position) * 0.5;
		let seed = self.stick_surface_noise.seed
			+ segment.start.position.length() as i32
			+ segment.end.position.length() as i32;
		let noise = NoiseConfig::new(self.stick_surface_noise.with_seed(seed));
		let n = noise.sample_3d(mid.x, mid.y, mid.z).clamp(-1.0, 1.0);
		(base * (1.0 + 0.40 * n)).max(4.0)
	}
}

impl<StickM, StickS> StickRenderRule<ChicoCrookStick<StickM, StickS>, StorybookTreeChain>
	for BraidOakTreeStickRule<StickM, StickS>
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
		let strength = self.bend_strength(segment, parent_hysteresis);
		Some(ChicoCrookStick::new(
			strength,
			segment_key(segment),
			self.stick_material.clone(),
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Vec3;
	use chico_sbs_geometry::BallStickNode;
	use procedural_common::FromScalarNoise;

	#[test]
	fn branch_bend_strength_grows_with_ring_height() {
		let rule = BraidOakTreeStickRule::<StandardMaterial, MeshMaterial3d<StandardMaterial>> {
			stick_surface_noise: NoiseParams::from_scalar(42.0, 1.0, 0.05, 1),
			stick_material: MeshMaterial3d::<StandardMaterial>::default(),
			__marker: PhantomData,
		};
		let segment = BallStickSegment {
			start: &BallStickNode::new(Vec3::ZERO, 0.4),
			end: &BallStickNode::new(Vec3::new(0.0, 2.0, 0.0), 0.35),
		};
		let low = StorybookTreeChain::new(
			procedural_common::NoiseConfig::new(NoiseParams::default()),
			6.0,
			3,
			0.0,
			0.1,
			0.65,
			StorybookTreePhase::BranchOut(chico_sbs_geometry::DepthBudget {
				inner: chico_sbs_geometry::BranchOut::radial_out_horizontal(
					BallStickNode::new(Vec3::ZERO, 0.04),
					Vec3::X,
				),
				remaining: 3,
			}),
		);
		let high = StorybookTreeChain::new(
			procedural_common::NoiseConfig::new(NoiseParams::default()),
			6.0,
			3,
			0.0,
			0.9,
			0.65,
			StorybookTreePhase::BranchOut(chico_sbs_geometry::DepthBudget {
				inner: chico_sbs_geometry::BranchOut::radial_out_horizontal(
					BallStickNode::new(Vec3::ZERO, 0.04),
					Vec3::X,
				),
				remaining: 3,
			}),
		);
		let s_lo = rule.bend_strength(&segment, &low);
		let s_hi = rule.bend_strength(&segment, &high);
		assert!(s_hi > s_lo);
	}
}
