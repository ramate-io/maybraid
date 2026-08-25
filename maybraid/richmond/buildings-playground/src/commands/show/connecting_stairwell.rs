//! `/show connecting-stairwell` — run-in floor + spiral flight between two openings.

use bevy::prelude::*;
use clap::{Args, ValueEnum};

use super::ShowTransform;
use crate::preview::PreviewSubject;
use richmond_buildings::LANDING_THICKNESS_M;

/// Named shaft pairs that stress spiral fit (inscription, center, arrive).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum ConnectingStairwellCase {
	/// 2.4×2.4 stacked well, 3 m rise, both walk-offs +Z.
	#[default]
	Stacked,
	/// 0.8×2.4 slot. Radius floor (0.35) plus half tread bleeds the short sides.
	NarrowSlot,
	/// 3.2×1.0 shallow well. Same min-radius bleed on the short axis.
	Shallow,
	/// Upper shaft 3 m east / 3 m north of lower. Spiral sits on the plan midpoint.
	Offset,
	/// 10 cm plan shift (no polyline kink). Spiral centers on the upper face.
	NearOffset,
	/// Lower 2.4×2.4, upper 1.2×1.2. Inscribed to the smaller hole.
	Mismatch,
	/// Stacked, walk-ons on opposite sides (south → north).
	Opposite,
	/// Same Y, 4 m apart, facing each other. Rise floors to one tread.
	SameY,
	/// 9 m stacked rise (turns approach the 3.0 clamp).
	Tall,
	/// 6×6 well. Outer rail should still sit on the hole.
	Huge,
}

impl ConnectingStairwellCase {
	pub fn slug(self) -> &'static str {
		match self {
			Self::Stacked => "stacked",
			Self::NarrowSlot => "narrow-slot",
			Self::Shallow => "shallow",
			Self::Offset => "offset",
			Self::NearOffset => "near-offset",
			Self::Mismatch => "mismatch",
			Self::Opposite => "opposite",
			Self::SameY => "same-y",
			Self::Tall => "tall",
			Self::Huge => "huge",
		}
	}

	/// What the preview is meant to expose.
	pub fn look_for(self) -> &'static str {
		match self {
			Self::Stacked => "outer rail on the south walk-on; same-side arrive wraps a full extra turn",
			Self::NarrowSlot => "outer rail ~12 cm past the 0.8 m slot sides (radius floor)",
			Self::Shallow => "outer rail past the 1.0 m depth (radius floor)",
			Self::Offset => "spiral centered between the two holes, not in either",
			Self::NearOffset => "spiral centered on the upper hole; ~10 cm bleed on +X of lower",
			Self::Mismatch => "outer rail on the 1.2 m upper hole; floats inside the lower",
			Self::Opposite => "landing is a short pad off the last tread in its travel direction",
			Self::SameY => "18 cm spiral on the ground between two floor-level holes",
			Self::Tall => "tight 9 m helix; still inscribed if stacked",
			Self::Huge => "wide ring in a 6×6 hole; outer rail on the edge",
		}
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ConnectingStairwell {
	/// Pathological shaft pair. Default is the stacked 2.4×2.4 demo.
	#[arg(long, value_enum, default_value_t = ConnectingStairwellCase::Stacked)]
	pub case: ConnectingStairwellCase,
	/// Skip the upper landing (a follow-on stairwell would own it).
	#[arg(long, default_value_t = false)]
	pub no_upper_landing: bool,
	/// Upper-landing slab thickness (meters).
	#[arg(long, default_value_t = LANDING_THICKNESS_M)]
	pub landing_thickness: f32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ConnectingStairwell {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ConnectingStairwell {
				case: self.case,
				upper_landing: !self.no_upper_landing,
				landing_thickness: self.landing_thickness,
			},
			self.transform.transform(),
		)
	}
}
