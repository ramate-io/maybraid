//! [`BraidGrassCell`] bucket enum ([RFC-183 §3.4.5.1]).

use crate::braid_grass::BraidGrassClump;
use crate::grove::{PaletteMix, PlacementConstraints};
use crate::{grove_buckets, unit_range};

grove_buckets! {
	/// Ordered braid-grass variants with explicit `None` bucket ([RFC-183 §3.4.2.2]).
	#[derive(Debug, Clone, PartialEq)]
	pub enum BraidGrassCell {
		@none None {
			weight: 2.5,
			placement_constraints: PlacementConstraints::UNCONSTRAINED,
		},
		DeepGreenBlade {
			weight: 2.0,
			placement_constraints: PlacementConstraints::new(
				unit_range!(0.0..0.75),
				unit_range!(0.0..0.60),
			),
			palette_mix: [
				[deep_green..wet_green],
				[dark_green..emerald_green],
				[blue_green..fresh_green],
			],
			item: BraidGrassClump {
				height: 1.0..2.2,
				width: 0.35..0.85,
				blade_count: 12..=28,
				braid_twist: 0.10..0.35,
			},
		},
		PaleReedBlade {
			weight: 1.0,
			placement_constraints: PlacementConstraints::new(
				unit_range!(0.0..0.75),
				unit_range!(0.0..0.60),
			),
			palette_mix: [
				[yellow_green..pale_straw],
				[dry_green..light_green],
				[tan_green..fresh_green],
			],
			item: BraidGrassClump {
				height: 1.2..2.6,
				width: 0.30..0.70,
				blade_count: 10..=22,
				braid_twist: 0.05..0.25,
			},
		},
		JungleBlade {
			weight: 1.0,
			placement_constraints: PlacementConstraints::new(
				unit_range!(0.0..0.45),
				unit_range!(0.0..0.30),
			),
			palette_mix: [
				[lush_green..bright_green],
				[wet_green..lime_green],
				[blue_green..deep_green],
			],
			item: BraidGrassClump {
				height: 1.6..3.0,
				width: 0.45..1.00,
				blade_count: 18..=36,
				braid_twist: 0.20..0.50,
			},
		},
		RedEdgeBlade {
			weight: 0.5,
			placement_constraints: PlacementConstraints::new(
				unit_range!(0.0..0.45),
				unit_range!(0.0..0.60),
			),
			palette_mix: [
				[red_green..deep_green],
				[copper_red..yellow_green],
				[dark_red..wet_green],
			],
			item: BraidGrassClump {
				height: 1.0..2.0,
				width: 0.30..0.75,
				blade_count: 10..=24,
				braid_twist: 0.10..0.30,
			},
		},
	}
}

impl BraidGrassCell {
	/// Authored palette ranges for this bucket (`None` carries an empty mix).
	pub fn palette_mix(&self) -> &PaletteMix {
		match self {
			BraidGrassCell::None(bucket) => &bucket.palette_mix,
			BraidGrassCell::DeepGreenBlade(bucket) => &bucket.palette_mix,
			BraidGrassCell::PaleReedBlade(bucket) => &bucket.palette_mix,
			BraidGrassCell::JungleBlade(bucket) => &bucket.palette_mix,
			BraidGrassCell::RedEdgeBlade(bucket) => &bucket.palette_mix,
		}
	}
}
