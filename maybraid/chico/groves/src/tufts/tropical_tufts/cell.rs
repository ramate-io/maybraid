//! [`TropicalTuftsCell`] bucket enum ([RFC-183 §3.4.4.5]).

use super::{TropicalPalmBush, TropicalTuftClump};
use crate::grove::{
	Bucket, GroveBucket, GroveDistribution, PaletteMix, PlacementConstraints,
};
use crate::{palette_slot, unit_range};

/// Placed bucket weight sum used to tune the sparse `None` bucket.
const PLACED_WEIGHT_SUM: f32 = 5.6;

/// `None` weight targeting ~16% fill (midpoint of RFC `0.08..0.24` density band).
const NONE_WEIGHT: f32 = PLACED_WEIGHT_SUM * (1.0 / 0.16 - 1.0);

/// Ordered tropical-tufts variants with explicit `None` bucket ([RFC-183 §3.4.2.2]).
#[derive(Debug, Clone, PartialEq)]
pub enum TropicalTuftsCell {
	None(Bucket<()>),
	BrightTuft(Bucket<TropicalTuftClump>),
	DeepTuft(Bucket<TropicalTuftClump>),
	YellowGreenTuft(Bucket<TropicalTuftClump>),
	SmallPalmBush(Bucket<TropicalPalmBush>),
	JuvenilePalmBush(Bucket<TropicalPalmBush>),
}

impl TropicalTuftsCell {
	/// Ordered [`GroveDistribution`] matching declaration order.
	pub fn grove_distribution() -> GroveDistribution<Self> {
		let constraints_mild = PlacementConstraints::new(
			unit_range!(0.0..0.65),
			unit_range!(0.0..0.35),
		);
		let constraints_standard = PlacementConstraints::new(
			unit_range!(0.0..0.65),
			unit_range!(0.0..0.75),
		);
		let constraints_juvenile = PlacementConstraints::new(
			unit_range!(0.0..0.55),
			unit_range!(0.0..0.75),
		);

		let mut dist = GroveDistribution::new();
		dist.push(GroveBucket {
			weight: NONE_WEIGHT,
			constraints: PlacementConstraints::UNCONSTRAINED,
			item: None,
		});
		dist.push(GroveBucket {
			weight: 2.0,
			constraints: constraints_mild,
			item: Some(TropicalTuftsCell::BrightTuft(Bucket {
				weight: 2.0,
				placement_constraints: constraints_mild,
				palette_mix: PaletteMix::from_slots(vec![
					palette_slot!(bright_green..lime_green),
					palette_slot!(lush_green..fresh_green),
					palette_slot!(yellow_green..light_green),
				]),
				item: TropicalTuftClump {
					height: unit_range!(0.25..0.50),
					width: unit_range!(0.14..0.34),
				},
			})),
		});
		dist.push(GroveBucket {
			weight: 1.5,
			constraints: constraints_standard,
			item: Some(TropicalTuftsCell::DeepTuft(Bucket {
				weight: 1.5,
				placement_constraints: constraints_standard,
				palette_mix: PaletteMix::from_slots(vec![
					palette_slot!(deep_green..emerald_green),
					palette_slot!(dark_green..wet_green),
					palette_slot!(blue_green..bright_green),
				]),
				item: TropicalTuftClump {
					height: unit_range!(0.30..0.55),
					width: unit_range!(0.16..0.38),
				},
			})),
		});
		dist.push(GroveBucket {
			weight: 1.0,
			constraints: constraints_standard,
			item: Some(TropicalTuftsCell::YellowGreenTuft(Bucket {
				weight: 1.0,
				placement_constraints: constraints_standard,
				palette_mix: PaletteMix::from_slots(vec![
					palette_slot!(yellow_green..fresh_green),
					palette_slot!(lime_green..light_green),
					palette_slot!(young_green..bright_green),
				]),
				item: TropicalTuftClump {
					height: unit_range!(0.25..0.45),
					width: unit_range!(0.12..0.30),
				},
			})),
		});
		dist.push(GroveBucket {
			weight: 0.75,
			constraints: constraints_standard,
			item: Some(TropicalTuftsCell::SmallPalmBush(Bucket {
				weight: 0.75,
				placement_constraints: constraints_standard,
				palette_mix: PaletteMix::from_slots(vec![
					palette_slot!(lush_green..bright_green),
					palette_slot!(deep_green..fresh_green),
					palette_slot!(wet_green..lime_green),
				]),
				item: TropicalPalmBush {
					height: unit_range!(0.35..0.80),
					frond_count: 4..=7,
					frond_length: unit_range!(0.18..0.45),
					crown_spread: unit_range!(0.25..0.55),
				},
			})),
		});
		dist.push(GroveBucket {
			weight: 0.35,
			constraints: constraints_juvenile,
			item: Some(TropicalTuftsCell::JuvenilePalmBush(Bucket {
				weight: 0.35,
				placement_constraints: constraints_juvenile,
				palette_mix: PaletteMix::from_slots(vec![
					palette_slot!(young_green..lime_green),
					palette_slot!(fresh_green..light_green),
					palette_slot!(bright_green..yellow_green),
				]),
				item: TropicalPalmBush {
					height: unit_range!(0.50..1.10),
					frond_count: 3..=5,
					frond_length: unit_range!(0.25..0.60),
					crown_spread: unit_range!(0.30..0.70),
				},
			})),
		});
		dist
	}

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
