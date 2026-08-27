//! `/show connecting-stairwell` — exclusive AABB well with a circular spiral.

use bevy::prelude::*;
use clap::{Args, ValueEnum};

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
