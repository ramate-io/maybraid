//! Rechargeable grenade catalog types. Runtime throw lives in `grenades`.

use serde::{Deserialize, Serialize};

/// Visual / grip identity for a grenade.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GrenadeMesh {
	#[default]
	Standard,
}

impl GrenadeMesh {
	pub const VALUES: &'static [Self] = &[Self::Standard];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Standard => "Grenade",
		}
	}

	pub const fn nouns(self) -> &'static [&'static str] {
		match self {
			Self::Standard => &["Grenade", "Charge", "Canister", "Pineapple"],
		}
	}
}

/// Authored model and animation profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GrenadeSpec {
	pub mesh: GrenadeMesh,
}

impl GrenadeSpec {
	pub const fn standard() -> Self {
		Self { mesh: GrenadeMesh::Standard }
	}
}

/// Fuse, recharge, and launch numbers. Seconds, not process-time deadlines.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GrenadeStats {
	pub fuse: f32,
	pub recharge: f32,
	pub launch_speed: f32,
	pub upward_bias: f32,
	pub inherit_velocity: f32,
	pub radius: f32,
	pub mass: f32,
	pub restitution: f32,
	pub friction: f32,
	pub throw_secs: f32,
	pub release_at: f32,
	pub weight: u16,
	pub effect_scale: f32,
	pub effect_intensity: f32,
	pub effect_playback: f32,
}

impl PartialEq for GrenadeStats {
	fn eq(&self, other: &Self) -> bool {
		self.fuse.to_bits() == other.fuse.to_bits()
			&& self.recharge.to_bits() == other.recharge.to_bits()
			&& self.launch_speed.to_bits() == other.launch_speed.to_bits()
			&& self.upward_bias.to_bits() == other.upward_bias.to_bits()
			&& self.inherit_velocity.to_bits() == other.inherit_velocity.to_bits()
			&& self.radius.to_bits() == other.radius.to_bits()
			&& self.mass.to_bits() == other.mass.to_bits()
			&& self.restitution.to_bits() == other.restitution.to_bits()
			&& self.friction.to_bits() == other.friction.to_bits()
			&& self.throw_secs.to_bits() == other.throw_secs.to_bits()
			&& self.release_at.to_bits() == other.release_at.to_bits()
			&& self.weight == other.weight
			&& self.effect_scale.to_bits() == other.effect_scale.to_bits()
			&& self.effect_intensity.to_bits() == other.effect_intensity.to_bits()
			&& self.effect_playback.to_bits() == other.effect_playback.to_bits()
	}
}

impl Eq for GrenadeStats {}

impl Default for GrenadeStats {
	fn default() -> Self {
		Self::standard()
	}
}

impl GrenadeStats {
	pub const fn standard() -> Self {
		Self {
			fuse: 2.5,
			recharge: 5.0,
			launch_speed: 6.5,
			upward_bias: 0.40,
			inherit_velocity: 0.18,
			radius: 0.08,
			mass: 0.45,
			restitution: 0.28,
			friction: 0.55,
			throw_secs: 0.92,
			release_at: 0.56,
			weight: 12,
			effect_scale: 1.0,
			effect_intensity: 1.0,
			effect_playback: 1.0,
		}
	}

	pub fn catalog_detail(&self) -> String {
		format!("fuse {:.1}s · recharge {:.1}s · {:>3} wt", self.fuse, self.recharge, self.weight)
	}

	pub fn stat_rows(&self) -> Vec<(String, String)> {
		vec![
			(String::from("Fuse"), format!("{:.1}s", self.fuse)),
			(String::from("Recharge"), format!("{:.1}s", self.recharge)),
			(String::from("Throw"), format!("{:.0} m/s", self.launch_speed)),
			(String::from("Weight"), self.weight.to_string()),
		]
	}
}

/// Remaining recharge on this owned instance. Ignored by catalog equality.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct GrenadeRecharge {
	pub remaining: f32,
}

impl PartialEq for GrenadeRecharge {
	fn eq(&self, _other: &Self) -> bool {
		true
	}
}

impl Eq for GrenadeRecharge {}

impl GrenadeRecharge {
	pub fn ready(self) -> bool {
		self.remaining <= 1e-4
	}

	pub fn start(&mut self, duration: f32) {
		self.remaining = duration.max(0.0);
	}

	pub fn tick(&mut self, dt: f32) {
		self.remaining = (self.remaining - dt.max(0.0)).max(0.0);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn recharge_is_ignored_by_equality() {
		let a = GrenadeRecharge { remaining: 4.0 };
		let b = GrenadeRecharge { remaining: 0.0 };
		assert_eq!(a, b);
	}

	#[test]
	fn release_is_inside_the_throw_window() {
		let stats = GrenadeStats::standard();
		assert!(stats.release_at > 0.0);
		assert!(stats.release_at < stats.throw_secs);
		assert!(stats.recharge > stats.fuse);
	}
}
