//! Tuberwaber color palettes.
//!
//! Earth-toned swatches with a bit more chroma than Braidman's shared
//! [`ItemColor`] skin defaults — clay, ochre, olive, and soft rose rather than
//! flat natural / primary accents.

use clap::ValueEnum;

/// Shared Tuberwaber swatch set for body and head features.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TuberwaberColor {
	#[default]
	Clay,
	Ochre,
	Terracotta,
	Sand,
	Olive,
	Sage,
	Rust,
	DustyRose,
	Bark,
	SlateClay,
	Amber,
	Jade,
	Copper,
}

impl TuberwaberColor {
	pub const VALUES: &'static [Self] = &[
		Self::Clay,
		Self::Ochre,
		Self::Terracotta,
		Self::Sand,
		Self::Olive,
		Self::Sage,
		Self::Rust,
		Self::DustyRose,
		Self::Bark,
		Self::SlateClay,
		Self::Amber,
		Self::Jade,
		Self::Copper,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Clay => "clay",
			Self::Ochre => "ochre",
			Self::Terracotta => "terracotta",
			Self::Sand => "sand",
			Self::Olive => "olive",
			Self::Sage => "sage",
			Self::Rust => "rust",
			Self::DustyRose => "dusty-rose",
			Self::Bark => "bark",
			Self::SlateClay => "slate-clay",
			Self::Amber => "amber",
			Self::Jade => "jade",
			Self::Copper => "copper",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Clay => "#C9956E",
			Self::Ochre => "#D4A04A",
			Self::Terracotta => "#C47A5A",
			Self::Sand => "#E2C9A0",
			Self::Olive => "#8B9A5C",
			Self::Sage => "#9AAB7E",
			Self::Rust => "#B85C3A",
			Self::DustyRose => "#C48A8A",
			Self::Bark => "#5C4030",
			Self::SlateClay => "#7A8A7E",
			Self::Amber => "#E0A030",
			Self::Jade => "#4A8A6A",
			Self::Copper => "#B8734A",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Clay => bevy::prelude::Color::srgb(0.79, 0.58, 0.43),
			Self::Ochre => bevy::prelude::Color::srgb(0.83, 0.63, 0.29),
			Self::Terracotta => bevy::prelude::Color::srgb(0.77, 0.48, 0.35),
			Self::Sand => bevy::prelude::Color::srgb(0.89, 0.79, 0.63),
			Self::Olive => bevy::prelude::Color::srgb(0.55, 0.60, 0.36),
			Self::Sage => bevy::prelude::Color::srgb(0.60, 0.67, 0.49),
			Self::Rust => bevy::prelude::Color::srgb(0.72, 0.36, 0.23),
			Self::DustyRose => bevy::prelude::Color::srgb(0.77, 0.54, 0.54),
			Self::Bark => bevy::prelude::Color::srgb(0.36, 0.25, 0.19),
			Self::SlateClay => bevy::prelude::Color::srgb(0.48, 0.54, 0.49),
			Self::Amber => bevy::prelude::Color::srgb(0.88, 0.63, 0.19),
			Self::Jade => bevy::prelude::Color::srgb(0.29, 0.54, 0.42),
			Self::Copper => bevy::prelude::Color::srgb(0.72, 0.45, 0.29),
		}
	}
}
