//! Per-character movement sheet: body, covering, and planning budget.

use bevy::prelude::*;

use crate::surface::CandidateBudget;

pub const MAX_VANTAGE_STANDOFFS: usize = 8;

/// Copyable standoff radii for [`crate::MovementObjective::VantageOn`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VantageStandoffs {
	radii: [f32; MAX_VANTAGE_STANDOFFS],
	len: u8,
}

impl VantageStandoffs {
	pub fn from_radii(radii: &[f32]) -> Self {
		let mut stored = [0.0; MAX_VANTAGE_STANDOFFS];
		let len = radii.len().min(MAX_VANTAGE_STANDOFFS);
		stored[..len].copy_from_slice(&radii[..len]);
		Self { radii: stored, len: len as u8 }
	}

	pub fn as_slice(&self) -> &[f32] {
		&self.radii[..self.len as usize]
	}
}

impl Default for VantageStandoffs {
	fn default() -> Self {
		Self::from_radii(&[3.5, 6.5, 10.0])
	}
}

/// Lateral via span as a fraction of leftover walk work. Same ratio routing
/// bands use for `lateral_span` (`segment * 0.55`).
pub const WALK_DETOUR_WORK_FRACTION: f32 = 0.55;

/// Body, covering, and query budget for one mover.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovementAbility {
	/// Vertical curb / stair the mover may climb, meters.
	pub max_step: f32,
	/// Preferred XZ hop between walk vias. Walk stride ceiling, not stair height.
	pub path_segment: f32,
	/// Vertical jump budget (meters). Realization reads this; the planner does not.
	pub max_jump: f32,
	/// Largest unsupported drop this mover will deliberately enter.
	pub max_fall: f32,
	pub can_use_doors: bool,
	pub can_use_stairs: bool,
	pub agent_radius: f32,
	/// Height of the feet below the capsule origin (center).
	pub feet_below_origin: f32,
	pub eye_height: f32,
	pub hip_height: f32,
	pub candidate_budget: CandidateBudget,
	pub vantage_standoffs: VantageStandoffs,
	pub vantage_azimuths: u32,
}

impl Default for MovementAbility {
	fn default() -> Self {
		Self {
			max_step: 0.4,
			path_segment: 4.0,
			max_jump: 1.0,
			max_fall: 1.2,
			can_use_doors: false,
			can_use_stairs: true,
			agent_radius: 0.4,
			feet_below_origin: 0.9,
			eye_height: 1.45,
			hip_height: 0.55,
			candidate_budget: CandidateBudget::default(),
			vantage_standoffs: VantageStandoffs::default(),
			vantage_azimuths: 8,
		}
	}
}

/// Dimensions a collider-backed surface needs from [`MovementAbility`] (or another bag).
pub trait MovementBody {
	fn agent_radius(&self) -> f32;
	fn max_step(&self) -> f32;
	/// Preferred XZ hop between walk vias. Ceiling; leftover work may shrink it.
	fn path_segment(&self) -> f32 {
		4.0
	}
	/// Lateral via distances for one walk hop. `path_segment` is the ceiling;
	/// `remaining_work` (chord minus arrival) scales like routing laterals.
	fn walk_detour_offsets(&self, remaining_work: f32) -> [f32; 3] {
		let cap = self.path_segment().max(1.0);
		let segment = cap.min(remaining_work.max(0.0) * WALK_DETOUR_WORK_FRACTION).max(1.0);
		[segment, segment * 1.75, segment * 2.6]
	}
	/// Vertical jump budget (meters). Realization uses this to decide whether to hop.
	fn max_jump(&self) -> f32 {
		0.0
	}
	/// Largest unsupported drop this mover will deliberately enter.
	fn max_fall(&self) -> f32 {
		1.0
	}
	fn feet_below_origin(&self) -> f32;
	fn eye_height(&self) -> f32;
	fn hip_height(&self) -> f32;
	fn can_use_stairs(&self) -> bool {
		true
	}

	fn hip_point(&self, origin: Vec3) -> Vec3 {
		Vec3::new(origin.x, origin.y - self.feet_below_origin() + self.hip_height(), origin.z)
	}

	fn eye_point(&self, origin: Vec3) -> Vec3 {
		Vec3::new(origin.x, origin.y - self.feet_below_origin() + self.eye_height(), origin.z)
	}
}

/// Per-character covering / planning knobs. Implemented by [`MovementAbility`].
pub trait Covering {
	fn candidate_budget(&self) -> CandidateBudget;
	fn vantage_standoffs(&self) -> &[f32];
	fn vantage_azimuths(&self) -> u32;
}

/// Body + covering. Plugin and collider surfaces take this bound.
pub trait MovementSheet: MovementBody + Covering {}

impl<T: MovementBody + Covering> MovementSheet for T {}

impl MovementBody for MovementAbility {
	fn agent_radius(&self) -> f32 {
		self.agent_radius
	}

	fn max_step(&self) -> f32 {
		self.max_step
	}

	fn path_segment(&self) -> f32 {
		self.path_segment
	}

	fn max_jump(&self) -> f32 {
		self.max_jump
	}

	fn max_fall(&self) -> f32 {
		self.max_fall
	}

	fn feet_below_origin(&self) -> f32 {
		self.feet_below_origin
	}

	fn eye_height(&self) -> f32 {
		self.eye_height
	}

	fn hip_height(&self) -> f32 {
		self.hip_height
	}

	fn can_use_stairs(&self) -> bool {
		self.can_use_stairs
	}
}

impl Covering for MovementAbility {
	fn candidate_budget(&self) -> CandidateBudget {
		self.candidate_budget
	}

	fn vantage_standoffs(&self) -> &[f32] {
		self.vantage_standoffs.as_slice()
	}

	fn vantage_azimuths(&self) -> u32 {
		self.vantage_azimuths.max(1)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hip_is_below_eye_and_above_feet() -> anyhow::Result<()> {
		let ability = MovementAbility::default();
		let origin = Vec3::new(0.0, 1.05, 0.0);
		let hip = ability.hip_point(origin);
		let eye = ability.eye_point(origin);
		assert!(hip.y < eye.y);
		assert!(hip.y > origin.y - ability.feet_below_origin);
		assert!((hip.y - 0.7).abs() < 1e-4, "{}", hip.y);
		assert!((eye.y - 1.6).abs() < 1e-4, "{}", eye.y);
		Ok(())
	}

	#[test]
	fn default_covering_exposes_standoffs_and_budget() -> anyhow::Result<()> {
		let ability = MovementAbility::default();
		assert_eq!(ability.vantage_standoffs(), &[3.5, 6.5, 10.0]);
		assert_eq!(ability.vantage_azimuths(), 8);
		assert_eq!(ability.candidate_budget().max_candidates, 16);
		Ok(())
	}

	#[test]
	fn walk_detours_use_path_segment_not_max_step() -> anyhow::Result<()> {
		let ability = MovementAbility { max_step: 0.4, path_segment: 4.0, ..Default::default() };
		let offsets = ability.walk_detour_offsets(12.0);
		assert!((offsets[0] - 4.0).abs() < 1e-4, "{offsets:?}");
		assert!((offsets[1] - 7.0).abs() < 1e-4, "{offsets:?}");
		assert!((offsets[2] - 10.4).abs() < 1e-4, "{offsets:?}");
		let tiny_step = MovementAbility { max_step: 0.05, path_segment: 4.0, ..Default::default() };
		assert_eq!(tiny_step.walk_detour_offsets(12.0), offsets);
		Ok(())
	}

	#[test]
	fn walk_detours_shrink_to_leftover_work() -> anyhow::Result<()> {
		let ability = MovementAbility::default();
		let leftover = ability.walk_detour_offsets(0.3);
		assert!((leftover[0] - 1.0).abs() < 1e-4, "{leftover:?}");
		let nearby = ability.walk_detour_offsets(3.0);
		assert!((nearby[0] - 3.0 * WALK_DETOUR_WORK_FRACTION).abs() < 1e-4, "{nearby:?}");
		assert!(nearby[0] < ability.path_segment);
		Ok(())
	}

	#[test]
	fn nearby_covering_hop_stays_tighter_than_path_segment() -> anyhow::Result<()> {
		let ability = MovementAbility::default();
		let work = 5.5;
		let offsets = ability.walk_detour_offsets(work);
		assert!((offsets[0] - work * WALK_DETOUR_WORK_FRACTION).abs() < 1e-4, "{offsets:?}");
		assert!(offsets[0] < ability.path_segment);
		Ok(())
	}
}
