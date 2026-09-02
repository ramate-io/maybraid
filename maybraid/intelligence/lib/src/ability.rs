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

/// Body, covering, and query budget for one mover.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovementAbility {
	pub max_step: f32,
	pub max_jump: f32,
	pub can_use_doors: bool,
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
			max_jump: 1.0,
			can_use_doors: false,
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
	fn feet_below_origin(&self) -> f32;
	fn eye_height(&self) -> f32;
	fn hip_height(&self) -> f32;

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

	fn feet_below_origin(&self) -> f32 {
		self.feet_below_origin
	}

	fn eye_height(&self) -> f32 {
		self.eye_height
	}

	fn hip_height(&self) -> f32 {
		self.hip_height
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
}
