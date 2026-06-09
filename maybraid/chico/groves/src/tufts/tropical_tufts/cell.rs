//! [`TropicalTuftsCell`] bucket enum ([RFC-183 §3.4.4.5]).

use super::{TropicalPalmBush, TropicalTuftClump};
use crate::grove::{PaletteMix, PlacementConstraints};
use crate::{grove_buckets, unit_range};

grove_buckets! {
	/// Ordered tropical-tufts variants with explicit `None` bucket ([RFC-183 §3.4.2.2]).
	#[derive(Debug, Clone, PartialEq)]
	pub enum TropicalTuftsCell {
		@none None {
			weight: 29.4,
			placement_constraints: PlacementConstraints::UNCONSTRAINED,
		},
		BrightTuft {
			weight: 2.0,
			placement_constraints: PlacementConstraints::new(
				unit_range!(0.0..0.65),
				unit_range!(0.0..0.35),
			),
			palette_mix: [
				[bright_green..lime_green],
				[lush_green..fresh_green],
				[yellow_green..light_green],
			],
			item: TropicalTuftClump {
				height: 0.25..0.50,
				width: 0.14..0.34,
			},
		},
		DeepTuft {
			weight: 1.5,
			placement_constraints: PlacementConstraints::new(
				unit_range!(0.0..0.65),
				unit_range!(0.0..0.75),
			),
			palette_mix: [
				[deep_green..emerald_green],
				[dark_green..wet_green],
				[blue_green..bright_green],
			],
			item: TropicalTuftClump {
				height: 0.30..0.55,
				width: 0.16..0.38,
			},
		},
		YellowGreenTuft {
			weight: 1.0,
			placement_constraints: PlacementConstraints::new(
				unit_range!(0.0..0.65),
				unit_range!(0.0..0.75),
			),
			palette_mix: [
				[yellow_green..fresh_green],
				[lime_green..light_green],
				[young_green..bright_green],
			],
			item: TropicalTuftClump {
				height: 0.25..0.45,
				width: 0.12..0.30,
			},
		},
		SmallPalmBush {
			weight: 0.75,
			placement_constraints: PlacementConstraints::new(
				unit_range!(0.0..0.65),
				unit_range!(0.0..0.75),
			),
			palette_mix: [
				[lush_green..bright_green],
				[deep_green..fresh_green],
				[wet_green..lime_green],
			],
			item: TropicalPalmBush {
				height: 0.35..0.80,
				frond_count: 4..=7,
				frond_length: 0.18..0.45,
				crown_spread: 0.25..0.55,
			},
		},
		JuvenilePalmBush {
			weight: 0.35,
			placement_constraints: PlacementConstraints::new(
				unit_range!(0.0..0.55),
				unit_range!(0.0..0.75),
			),
			palette_mix: [
				[young_green..lime_green],
				[fresh_green..light_green],
				[bright_green..yellow_green],
			],
			item: TropicalPalmBush {
				height: 0.50..1.10,
				frond_count: 3..=5,
				frond_length: 0.25..0.60,
				crown_spread: 0.30..0.70,
			},
		},
	}
}

impl TropicalTuftsCell {
	pub fn palette_mix(&self) -> &PaletteMix {
		match self {
			TropicalTuftsCell::None(bucket) => &bucket.palette_mix,
			TropicalTuftsCell::BrightTuft(bucket) => &bucket.palette_mix,
			TropicalTuftsCell::DeepTuft(bucket) => &bucket.palette_mix,
			TropicalTuftsCell::YellowGreenTuft(bucket) => &bucket.palette_mix,
			TropicalTuftsCell::SmallPalmBush(bucket) => &bucket.palette_mix,
			TropicalTuftsCell::JuvenilePalmBush(bucket) => &bucket.palette_mix,
		}
	}
}
