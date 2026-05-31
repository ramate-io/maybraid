//! Shape parameters for a single jungle-growth cluster at one anchor.

use bevy::prelude::*;
use chico_ball_components::frond::FrondCrownShape;
use chico_ball_components::tuft::BuddhaHandTuftShape;
use procedural_common::NoiseParams;

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

/// Frond crown anchor at the ball apex (shoots drape from the upper surface).
const FROND_CROWN_Y_FRACTION: f32 = 0.7;

/// Buddha's-hand anchor below the crown, still inside the upper ball mass.
const BUDDHA_HAND_Y_FRACTION: f32 = 0.6;

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
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.6))]
	pub foliage_world_scale: f32,
	/// Buddha's-hand tuft scale relative to [`foliage_world_scale`].
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.8))]
	pub buddha_hand_scale: f32,
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
			foliage_world_scale: 0.6,
			buddha_hand_scale: 0.8,
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

	/// Inner ball transform in assembly-local space (parent scale = node radius).
	pub fn local_body_transform(&self) -> Transform {
		Transform::from_scale(Vec3::splat(self.inner_ball_scale))
	}

	fn foliage_anchor_y(&self, y_fraction: f32) -> f32 {
		self.inner_ball_scale * y_fraction
	}

	/// Frond crown transform in assembly-local space (anchored at the inner-ball apex).
	pub fn local_frond_transform(&self) -> Transform {
		Transform {
			translation: Vec3::Y * self.foliage_anchor_y(FROND_CROWN_Y_FRACTION),
			scale: Vec3::splat(self.foliage_world_scale),
			..default()
		}
	}

	/// Buddha's-hand transform in assembly-local space (fixed offset below the crown, inside the mass).
	pub fn local_buddha_transform(&self) -> Transform {
		Transform {
			translation: Vec3::Y * self.foliage_anchor_y(BUDDHA_HAND_Y_FRACTION),
			scale: Vec3::splat(self.foliage_world_scale * self.buddha_hand_scale),
			..default()
		}
	}

	/// Jungle frond crown geometry with [`foliage_noise`] and [`Self::seed`] applied.
	pub fn frond_shape(&self, foliage_noise: &NoiseParams) -> FrondCrownShape {
		let mut frond = self.frond.clone();
		let noise = foliage_noise.with_seed(self.seed.wrapping_add(31));
		frond.seed = noise.seed;
		frond
	}

	/// Jungle Buddha's-hand geometry with [`foliage_noise`] and a derived seed applied.
	pub fn buddha_hand_shape(&self, foliage_noise: &NoiseParams) -> BuddhaHandTuftShape {
		let mut buddha = self.buddha_hand.clone();
		let noise = foliage_noise.with_seed(self.seed.wrapping_add(31));
		buddha.seed = noise.seed;
		buddha.noise_frequency = noise.frequency;
		buddha.noise_amplitude = noise.amplitude;
		buddha
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use procedural_common::FromScalarNoise;

	#[test]
	fn foliage_anchors_track_inner_ball_scale() -> Result<()> {
		for inner_ball_scale in [0.5_f32, 0.72, 1.1] {
			let shape = JungleGrowthShape { inner_ball_scale, ..JungleGrowthShape::default() };
			let frond = shape.local_frond_transform();
			let buddha = shape.local_buddha_transform();
			assert!((frond.translation.y - inner_ball_scale * FROND_CROWN_Y_FRACTION).abs() < 1e-5);
			assert!(
				(buddha.translation.y - inner_ball_scale * BUDDHA_HAND_Y_FRACTION).abs() < 1e-5
			);
			assert!(buddha.translation.y < frond.translation.y);
			assert!((frond.scale.x - shape.foliage_world_scale).abs() < 1e-5);
			assert!(
				(buddha.scale.x - shape.foliage_world_scale * shape.buddha_hand_scale).abs() < 1e-5
			);
		}
		Ok(())
	}

	#[test]
	fn frond_and_buddha_shapes_take_foliage_noise() -> Result<()> {
		let shape = JungleGrowthShape { seed: 42, ..JungleGrowthShape::default() };
		let foliage_noise = NoiseParams::from_scalar(0.0, 3.5, 0.12, 2);
		let frond = shape.frond_shape(&foliage_noise);
		assert_eq!(frond.seed, 42_i32.wrapping_add(31));
		let buddha = shape.buddha_hand_shape(&foliage_noise);
		assert_eq!(buddha.seed, 42_i32.wrapping_add(31));
		assert!((buddha.noise_frequency - 3.5).abs() < 1e-5);
		assert!((buddha.noise_amplitude - 0.12).abs() < 1e-5);
		Ok(())
	}
}
