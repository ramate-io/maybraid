//! Character capabilities, separate from world affordances.

use bevy::prelude::*;

/// Body + action limits used when a surface probes the world.
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
		}
	}
}

/// Dimensions a collider-backed surface needs from [`MovementAbility`] (or another bag).
pub trait MovementBody {
	fn agent_radius(&self) -> f32;
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

impl MovementBody for MovementAbility {
	fn agent_radius(&self) -> f32 {
		self.agent_radius
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
}
