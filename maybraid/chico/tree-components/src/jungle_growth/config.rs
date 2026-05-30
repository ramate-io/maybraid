//! Shape parameters for a single jungle-growth cluster at one anchor.

use chico_ball_components::frond::FrondCrownShape;
use chico_ball_components::tuft::BuddhaHandTuftShape;

/// Epiphyte-scale defaults for [`FrondCrownShape`] (outward arching shoots).
fn default_jungle_frond() -> FrondCrownShape {
	FrondCrownShape {
		frond_count: 6,
		length: 0.72,
		width: 0.11,
		droop: 0.42,
		twist: 0.28,
		leaflet_count: 22,
		spine_segments: 12,
		shoot_half_radius: 0.016,
		rachis_half_thickness: 0.006,
		leaflet_length_scale: 1.7,
		downward_tilt_radians: 0.48,
		outward_spread_radians: 0.58,
		seed: 0,
	}
}

/// Central upward fingers concealing the growth anchor (RFC palm-bush tuft).
fn default_jungle_buddha_hand() -> BuddhaHandTuftShape {
	BuddhaHandTuftShape {
		finger_count: 7,
		finger_length: 0.6,
		base_half_width: 0.036,
		belly_half_width: 0.085,
		max_tilt_radians: 0.2,
		noise_amplitude: 0.06,
		noise_frequency: 4.5,
		seed: 0,
	}
}

/// Per-instance configuration ([RFC-183 §3.1.6.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/04-jungle-growths/README.md)).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct JungleGrowthShape {
	/// Inner noisy-ball radius multiplier relative to spawn transform uniform scale (node radius).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.72))]
	pub inner_ball_scale: f32,
	/// Uniform scale for the arching frond crown in world units.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.25))]
	pub foliage_world_scale: f32,
	/// Buddha's-hand tuft scale relative to [`foliage_world_scale`].
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.58))]
	pub buddha_hand_scale: f32,
	/// Frond crown lift along local +Y as a fraction of inner ball radius (sits atop the mass).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.88))]
	pub frond_crown_lift: f32,
	/// Buddha's-hand lift along local +Y as a fraction of inner ball radius (above the crown).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.32))]
	pub buddha_hand_lift: f32,
	/// Deterministic seed for body and foliage noise at this anchor.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
	#[cfg_attr(feature = "clap", arg(skip))]
	pub frond: FrondCrownShape,
	#[cfg_attr(feature = "clap", arg(skip))]
	pub buddha_hand: BuddhaHandTuftShape,
}

impl Default for JungleGrowthShape {
	fn default() -> Self {
		Self {
			inner_ball_scale: 0.72,
			foliage_world_scale: 0.25,
			buddha_hand_scale: 0.58,
			frond_crown_lift: 0.88,
			buddha_hand_lift: 1.32,
			seed: 0,
			frond: default_jungle_frond(),
			buddha_hand: default_jungle_buddha_hand(),
		}
	}
}

impl JungleGrowthShape {
	pub fn ball_radius(&self, node_radius: f32) -> f32 {
		node_radius * self.inner_ball_scale
	}

	pub fn frond_shape(&self) -> FrondCrownShape {
		let mut frond = self.frond.clone();
		frond.seed = self.seed;
		frond
	}

	pub fn buddha_hand_shape(&self) -> BuddhaHandTuftShape {
		let mut buddha = self.buddha_hand.clone();
		buddha.seed = self.seed.wrapping_add(31);
		buddha
	}
}
