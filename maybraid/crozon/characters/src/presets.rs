//! Shared preset identifiers for concept-stage character resolution.
//!
//! Presets are not restrictions. They initialize or refine resolved values before
//! command/UI fields apply their final overrides.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum GenderPreset {
	#[default]
	Neutral,
	Male,
	Female,
	NonBinary,
}

impl GenderPreset {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Neutral => "neutral",
			Self::Male => "male",
			Self::Female => "female",
			Self::NonBinary => "non-binary",
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum BuildPreset {
	#[default]
	Average,
	Slender,
	Athletic,
	Heavy,
	Stocky,
	Lanky,
}

impl BuildPreset {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Average => "average",
			Self::Slender => "slender",
			Self::Athletic => "athletic",
			Self::Heavy => "heavy",
			Self::Stocky => "stocky",
			Self::Lanky => "lanky",
		}
	}
}
