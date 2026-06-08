//! [`BraidGrassCell`] bucket enum ([RFC-183 §3.4.5.1]).

use crate::braid_grass::BraidGrass;
use crate::grove::PlacementConstraints;
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
			item: BraidGrass {
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
			item: BraidGrass {
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
			item: BraidGrass {
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
			item: BraidGrass {
				height: 1.0..2.0,
				width: 0.30..0.75,
				blade_count: 10..=24,
				braid_twist: 0.10..0.30,
			},
		},
	}
}
