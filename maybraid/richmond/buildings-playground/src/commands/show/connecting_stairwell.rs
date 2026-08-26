//! `/show connecting-stairwell` — run-in floor + stair flight between two openings.

use bevy::prelude::*;
use clap::{Args, ValueEnum};

use super::ShowTransform;
use crate::preview::PreviewSubject;
use richmond_buildings::{StairwellFlightKind, SLAB_THICKNESS_M, TREAD_FILL_DEFAULT};

/// Flight family. Independent of [`ConnectingStairwellCase`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum ConnectingStairwellFlight {
	#[default]
	Spiral,
	RectangularSpiral,
	RunAndLanding,
}

impl ConnectingStairwellFlight {
	pub fn slug(self) -> &'static str {
		match self {
			Self::Spiral => "spiral",
			Self::RectangularSpiral => "rectangular-spiral",
			Self::RunAndLanding => "run-and-landing",
		}
	}

	pub fn kind(self) -> StairwellFlightKind {
		match self {
			Self::Spiral => StairwellFlightKind::Spiral,
			Self::RectangularSpiral => StairwellFlightKind::RectangularSpiral,
			Self::RunAndLanding => StairwellFlightKind::RunAndLanding,
		}
	}
}

/// Named shaft pairs. Independent of [`ConnectingStairwellFlight`].
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
	/// Stacked, upper walk-off +X (90° from the lower +Z).
	QuarterTurn,
	/// 8 m east / 6 m north. Long run-and-landing path; one real kink.
	LongOffset,
	/// 0.9×0.9 well. Corner pads (~0.65 m) eat most of a side.
	Tiny,
	/// 1.1 m stacked rise — a handful of treads, fat relative landings.
	ShortRise,
	/// Both openings yawed 45° (northeast walk-off).
	Skew,
	/// Floor-level L: south hole to an east hole, no rise.
	SameYL,
	/// 9 m rise plus a 4 m east / 3 m north plan offset.
	OffsetTall,
	/// L in plan and opposite walk-ons (south → north on the far hole).
	OppositeOffset,
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
			Self::QuarterTurn => "quarter-turn",
			Self::LongOffset => "long-offset",
			Self::Tiny => "tiny",
			Self::ShortRise => "short-rise",
			Self::Skew => "skew",
			Self::SameYL => "same-y-l",
			Self::OffsetTall => "offset-tall",
			Self::OppositeOffset => "opposite-offset",
		}
	}

	/// What the preview is meant to expose.
	pub fn look_for(self) -> &'static str {
		match self {
			Self::Stacked => "run-in / last-tread / landing tops flush with Y=0 and Y=3; same-side arrive",
			Self::NarrowSlot => "0.8 m slot: treads and pads may bleed the short sides",
			Self::Shallow => "1.0 m depth: treads and pads may bleed the short axis",
			Self::Offset => "one plan kink; landing pad at the mid station, not a sheared mountain",
			Self::NearOffset => "no polyline kink (~10 cm); flight stays on the stacked well",
			Self::Mismatch => "inscribe to the 1.2 m upper hole; floats inside the lower",
			Self::Opposite => "short upper pad off the last tread; walk-on is the far rim",
			Self::SameY => "floor-level facing holes; rise floors to one tread",
			Self::Tall => "9 m stacked rise; landings still flush with each storey",
			Self::Huge => "6×6 hole; outer rail / rim runs on the edge",
			Self::QuarterTurn => "arrive on the +X rim; rect-spiral must finish a different side",
			Self::LongOffset => "long L (8 m east / 6 m north); one rectangular kink pad",
			Self::Tiny => "0.9×0.9 well; ~0.65 m pads eat a side",
			Self::ShortRise => "1.1 m rise; fat landings vs a few treads",
			Self::Skew => "45° openings; pads stay planar on yawed rims",
			Self::SameYL => "no rise, true L on the ground; one corner pad",
			Self::OffsetTall => "9 m plus a 4×3 m offset; stacked laps then a long last run",
			Self::OppositeOffset => "L then opposite arrive — last tread vs far walk-on",
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
	/// Kit thickness of the run-in and upper-landing slabs (meters).
	#[arg(long, default_value_t = SLAB_THICKNESS_M)]
	pub slab_thickness: f32,
	/// Tread span as a fraction of the tighter opening half-extent.
	#[arg(long, default_value_t = TREAD_FILL_DEFAULT)]
	pub tread_fill: f32,
	/// Shaft fill. Independent of `--case`.
	#[arg(long, value_enum, default_value_t = ConnectingStairwellFlight::Spiral)]
	pub flight: ConnectingStairwellFlight,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ConnectingStairwell {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ConnectingStairwell {
				case: self.case,
				flight: self.flight,
				upper_landing: !self.no_upper_landing,
				slab_thickness: self.slab_thickness,
				tread_fill: self.tread_fill,
			},
			self.transform.transform(),
		)
	}
}
