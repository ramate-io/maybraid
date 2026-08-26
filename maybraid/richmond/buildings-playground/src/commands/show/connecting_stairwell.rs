//! `/show connecting-stairwell` — run-in floor + stair flight between two openings.

use bevy::prelude::*;
use clap::{Args, ValueEnum};

use super::ShowTransform;
use crate::preview::PreviewSubject;
use richmond_buildings::{
	StairwellFlightKind, LAPPING_RATIO_DEFAULT, SLAB_THICKNESS_M, TREAD_FILL_DEFAULT,
};

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
			Self::Stacked => {
				"run-in / last-tread / landing tops flush with Y=0 and Y=3; run-and-landing side-by-side U"
			}
			Self::NarrowSlot => "0.8 m slot: treads and pads may bleed the short sides",
			Self::Shallow => "1.0 m depth: treads and pads may bleed the short axis",
			Self::Offset => "one plan kink; landing pad at the mid station, not a sheared mountain",
			Self::NearOffset => "no polyline kink (~10 cm); flight stays on the stacked well",
			Self::Mismatch => "inscribe to the 1.2 m upper hole; floats inside the lower",
			Self::Opposite => "short upper pad off the last tread; walk-on is the far rim",
			Self::SameY => "floor-level facing holes; rise floors to one tread",
			Self::Tall => "9 m stacked rise; landings still flush with each storey",
			Self::Huge => "6×6 hole; outer rail / rim runs on the edge",
			Self::QuarterTurn => {
				"arrive on the +X rim; run-and-landing zig-zag then a landing to the west walk-on"
			}
			Self::LongOffset => "long L (8 m east / 6 m north); one rectangular kink pad",
			Self::Tiny => "0.9×0.9 well; ~0.65 m pads eat a side",
			Self::ShortRise => "1.1 m rise; one crossing (not enough to switchback)",
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
	/// Lapping ratio (preferred going / width). High values add rectangular-spiral circuits (stay 0.5–0.7 on a ~3 m well).
	#[arg(long, default_value_t = LAPPING_RATIO_DEFAULT)]
	pub lapping_ratio: f32,
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
				lapping_ratio: self.lapping_ratio,
			},
			self.transform.transform(),
		)
	}
}

/// One cell in `/show pathological-connecting-stairwell-gallery`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathologicalStairwellSpec {
	pub case: ConnectingStairwellCase,
	pub flight: ConnectingStairwellFlight,
	pub lapping_ratio: f32,
	pub upper_landing: bool,
	pub note: &'static str,
}

impl PathologicalStairwellSpec {
	pub fn label_text(self) -> String {
		let landing = if self.upper_landing { "upper landing" } else { "no upper landing" };
		format!(
			"{} / {}\nlapping {:.2}\n{landing}\n{}",
			self.case.slug(),
			self.flight.slug(),
			self.lapping_ratio,
			self.note
		)
	}

	pub fn label_style(self) -> richmond_building_components::LabelStyle {
		use richmond_building_components::LabelStyle;
		match self.flight {
			ConnectingStairwellFlight::RunAndLanding => LabelStyle::Cyan,
			ConnectingStairwellFlight::RectangularSpiral => LabelStyle::Yellow,
			ConnectingStairwellFlight::Spiral => LabelStyle::Green,
		}
	}

	/// Plan footprint used to space gallery cells (meters).
	pub fn plan_span(self) -> Vec2 {
		self.case.plan_span()
	}
}

impl ConnectingStairwellCase {
	pub fn plan_span(self) -> Vec2 {
		match self {
			Self::Stacked | Self::QuarterTurn | Self::Opposite | Self::ShortRise | Self::Skew => {
				Vec2::new(7.0, 7.0)
			}
			Self::NarrowSlot => Vec2::new(5.0, 7.0),
			Self::Shallow => Vec2::new(8.0, 5.0),
			Self::Mismatch => Vec2::new(7.0, 7.0),
			Self::NearOffset => Vec2::new(7.0, 7.0),
			Self::Offset => Vec2::new(10.0, 10.0),
			Self::SameY => Vec2::new(10.0, 7.0),
			Self::Tall => Vec2::new(7.0, 7.0),
			Self::Huge => Vec2::new(12.0, 12.0),
			Self::LongOffset => Vec2::new(16.0, 12.0),
			Self::Tiny => Vec2::new(5.0, 5.0),
			Self::SameYL => Vec2::new(10.0, 8.0),
			Self::OffsetTall => Vec2::new(12.0, 10.0),
			Self::OppositeOffset => Vec2::new(12.0, 12.0),
		}
	}
}

/// Review set for the run-and-landing zig-zag / arrive-landing work.
pub fn pathological_stairwell_gallery() -> [PathologicalStairwellSpec; 16] {
	use ConnectingStairwellCase as Case;
	use ConnectingStairwellFlight as Flight;
	let ral = Flight::RunAndLanding;
	let def = LAPPING_RATIO_DEFAULT;
	[
		PathologicalStairwellSpec {
			case: Case::QuarterTurn,
			flight: ral,
			lapping_ratio: def,
			upper_landing: true,
			note: "L: zig-zag then land west — no reverse on last column",
		},
		PathologicalStairwellSpec {
			case: Case::QuarterTurn,
			flight: ral,
			lapping_ratio: 1.2,
			upper_landing: true,
			note: "extra laps on two corridors; landing to west walk-on",
		},
		PathologicalStairwellSpec {
			case: Case::QuarterTurn,
			flight: ral,
			lapping_ratio: 1.2,
			upper_landing: false,
			note: "same as 1.2; well pads only",
		},
		PathologicalStairwellSpec {
			case: Case::Stacked,
			flight: ral,
			lapping_ratio: def,
			upper_landing: true,
			note: "side-by-side U; return at inner edge",
		},
		PathologicalStairwellSpec {
			case: Case::Stacked,
			flight: ral,
			lapping_ratio: 2.0,
			upper_landing: true,
			note: "stacked U laps, not new laterals",
		},
		PathologicalStairwellSpec {
			case: Case::ShortRise,
			flight: ral,
			lapping_ratio: def,
			upper_landing: true,
			note: "one crossing; no pad",
		},
		PathologicalStairwellSpec {
			case: Case::SameYL,
			flight: ral,
			lapping_ratio: def,
			upper_landing: true,
			note: "ground L; first riser on outgoing edge",
		},
		PathologicalStairwellSpec {
			case: Case::LongOffset,
			flight: ral,
			lapping_ratio: def,
			upper_landing: true,
			note: "polyline L; one level kink pad",
		},
		PathologicalStairwellSpec {
			case: Case::Tiny,
			flight: ral,
			lapping_ratio: def,
			upper_landing: true,
			note: "0.9 m well; pads eat a side",
		},
		PathologicalStairwellSpec {
			case: Case::Opposite,
			flight: ral,
			lapping_ratio: def,
			upper_landing: true,
			note: "I or odd bank; last vs far walk-on",
		},
		PathologicalStairwellSpec {
			case: Case::Skew,
			flight: ral,
			lapping_ratio: 1.2,
			upper_landing: true,
			note: "45° openings; pads stay planar",
		},
		PathologicalStairwellSpec {
			case: Case::Mismatch,
			flight: ral,
			lapping_ratio: def,
			upper_landing: true,
			note: "inscribe to smaller hole",
		},
		PathologicalStairwellSpec {
			case: Case::OffsetTall,
			flight: ral,
			lapping_ratio: 1.2,
			upper_landing: true,
			note: "9 m + offset; Y still stitches",
		},
		PathologicalStairwellSpec {
			case: Case::Tiny,
			flight: Flight::RectangularSpiral,
			lapping_ratio: def,
			upper_landing: true,
			note: "rim circuits; pad per interior joint",
		},
		PathologicalStairwellSpec {
			case: Case::QuarterTurn,
			flight: Flight::RectangularSpiral,
			lapping_ratio: 1.2,
			upper_landing: true,
			note: "extra rim lap; headroom shrinks",
		},
		PathologicalStairwellSpec {
			case: Case::QuarterTurn,
			flight: Flight::Spiral,
			lapping_ratio: def,
			upper_landing: true,
			note: "control: one-tread nodes still nest",
		},
	]
}
