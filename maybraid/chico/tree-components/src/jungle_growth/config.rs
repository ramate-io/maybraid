//! Shape parameters for a single jungle-growth cluster at one anchor.

use chico_ball_components::tuft::WeepingTuftShape;

/// Per-instance configuration ([RFC-183 §3.1.6.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/04-jungle-growths/README.md)).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct JungleGrowthShape {
	/// Inner noisy-ball radius multiplier relative to spawn transform uniform scale (node radius).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.72))]
	pub inner_ball_scale: f32,
	/// Tuft cluster uniform scale in world units.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.55))]
	pub tuft_world_scale: f32,
	/// Deterministic seed for body and foliage noise at this anchor.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
	#[cfg_attr(feature = "clap", command(flatten))]
	pub tuft: WeepingTuftShape,
}

impl Default for JungleGrowthShape {
	fn default() -> Self {
		Self {
			inner_ball_scale: 0.72,
			tuft_world_scale: 0.55,
			seed: 0,
			tuft: WeepingTuftShape::default(),
		}
	}
}
