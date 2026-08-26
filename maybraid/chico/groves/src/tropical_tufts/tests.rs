use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = TropicalTuftsCell::distribution();
	assert_eq!(dist.len(), 9);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 10.0);
	assert_eq!(dist.buckets[1].item, Some(TropicalTuftsCell::BrightTuft));
	assert_eq!(dist.buckets[1].weight, 0.5);
	assert_eq!(dist.buckets[2].weight, 0.35);
	assert_eq!(dist.buckets[3].weight, 0.25);
	assert_eq!(dist.buckets[4].item, Some(TropicalTuftsCell::SmallPalmBush));
	assert_eq!(dist.buckets[4].weight, 0.75);
	assert_eq!(dist.buckets[5].item, Some(TropicalTuftsCell::JuvenilePalmBush));
	assert_eq!(dist.buckets[5].weight, 0.35);
	assert_eq!(dist.buckets[6].item, Some(TropicalTuftsCell::BrightTuftPatch));
	assert_eq!(dist.buckets[6].weight, 2.0);
	assert_eq!(dist.buckets[7].item, Some(TropicalTuftsCell::DeepTuftPatch));
	assert_eq!(dist.buckets[7].weight, 1.5);
	assert_eq!(dist.buckets[8].item, Some(TropicalTuftsCell::YellowGreenTuftPatch));
	assert_eq!(dist.buckets[8].weight, 0.9);
	Ok(())
}

#[test]
fn patches_outweigh_single_tufts() -> Result<()> {
	let tuft_weight = |patch: bool| -> f32 {
		TropicalTuftsCell::distribution()
			.buckets
			.iter()
			.filter(|b| {
				b.item.is_some_and(|cell| match cell.item() {
					TropicalTuftsItem::Tuft(_) => !patch,
					TropicalTuftsItem::Patch(_) => patch,
					TropicalTuftsItem::PalmBush(_) => false,
				})
			})
			.map(|b| b.weight)
			.sum()
	};
	assert!(tuft_weight(true) > 2.0 * tuft_weight(false), "patches should dominate tuft weight");
	Ok(())
}

#[test]
fn variants_map_to_typed_items() -> Result<()> {
	assert!(matches!(TropicalTuftsCell::BrightTuft.item(), TropicalTuftsItem::Tuft(_)));
	let TropicalTuftsItem::PalmBush(palm) = TropicalTuftsCell::SmallPalmBush.item() else {
		anyhow::bail!("expected palm bush item");
	};
	assert_eq!(palm.frond_count, 4..=7);
	let TropicalTuftsItem::Patch(patch) = TropicalTuftsCell::BrightTuftPatch.item() else {
		anyhow::bail!("expected patch item");
	};
	assert_eq!(patch.clump, BRIGHT_TUFT);
	Ok(())
}

#[test]
fn first_fit_from_placed_bucket_places_variant() -> Result<()> {
	let prepared =
		TropicalTuftsCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.4, steepness: 0.1 };
	let outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.4, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	assert!(matches!(outcome, GroveCellOutcome::Placed { .. }));
	Ok(())
}

#[test]
fn populated_grove_is_deterministic() -> Result<()> {
	let grove = Grove::assemble(
		definition(),
		ForestGroveBiases::default(),
		NoiseParams::default(),
		Vec3::ZERO,
	);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(20.0, 1.0, 20.0));
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	assert_eq!(grove.populate(&extent, &terrain), grove.populate(&extent, &terrain));
	Ok(())
}
