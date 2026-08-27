//! `/show connecting-stairwell` — exclusive AABB well with a circular spiral.

use bevy::prelude::*;
use clap::{Args, ValueEnum};

use richmond_buildings::{ConnectingStairwell as Stairwell, WellAabb, WellSide};

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum StairwellCase {
	/// Same-plan shaft, walk-on and walk-off on −Z.
	#[default]
	Stacked,
	/// Walk-on −Z, walk-off +Z.
	Opposite,
	/// Walk-on −Z, walk-off −X.
	QuarterTurn,
	/// Tight well.
	Tiny,
	/// 6 m rise (extra turns to keep going).
	Tall,
	/// Two exclusive wells stacked; shared face is the join.
	StackedPair,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ConnectingStairwell {
	#[command(flatten)]
	pub transform: ShowTransform,
	/// Well plan / doors / height.
	#[arg(long, value_enum, default_value_t = StairwellCase::Stacked)]
	pub case: StairwellCase,
	/// Tread span as a fraction of the tighter half-extent.
	#[arg(long, default_value_t = 0.4)]
	pub tread_fill: f32,
}

impl ConnectingStairwell {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ConnectingStairwell { case: self.case, tread_fill: self.tread_fill },
			self.transform.transform(),
		)
	}
}

/// Exclusive wells for a playground case (one, or two for [`StairwellCase::StackedPair`]).
pub fn preview_wells(case: StairwellCase, tread_fill: f32) -> Vec<WellAabb> {
	let well = |min: Vec3, max: Vec3, on: WellSide, off: WellSide| {
		WellAabb::from_plan(min, max, on, off, tread_fill)
	};
	match case {
		StairwellCase::Stacked => vec![well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
		)],
		StairwellCase::Opposite => vec![well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::PosZ,
		)],
		StairwellCase::QuarterTurn => vec![well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::NegX,
		)],
		StairwellCase::Tiny => vec![well(
			Vec3::new(-0.6, 0.0, -0.6),
			Vec3::new(0.6, 1.5, 0.6),
			WellSide::NegZ,
			WellSide::NegZ,
		)],
		StairwellCase::Tall => vec![well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 6.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
		)],
		StairwellCase::StackedPair => vec![
			well(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::NegZ,
			),
			well(
				Vec3::new(-1.2, 3.0, -1.2),
				Vec3::new(1.2, 6.0, 1.2),
				WellSide::NegZ,
				WellSide::NegZ,
			),
		],
	}
}

pub fn preview_stairwells(case: StairwellCase, tread_fill: f32) -> Vec<Stairwell> {
	use richmond_building_components::panels::PanelStyle;
	preview_wells(case, tread_fill)
		.into_iter()
		.enumerate()
		.map(|(i, w)| {
			let well = Stairwell::from_well(PanelStyle::RoughStonework, w);
			if case == StairwellCase::StackedPair && i == 0 {
				well.with_upper_landing(false)
			} else {
				well
			}
		})
		.collect()
}
