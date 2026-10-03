use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;

use crate::english::{english_words, GeographicKind};
use crate::index::{LanguageIndex, LanguageWorldSeed, NameKey};
use crate::name::PlaceName;
use crate::sources::NamedFeature;
use crate::tiles::{language_count, LargeTile, LARGE_TILE};

const SEED: u64 = 0xC0DE_F00D_A11A;

#[test]
fn english_words_split_pascal_and_kebab() -> anyhow::Result<()> {
	anyhow::ensure!(english_words("RollingOaks") == ["rolling", "oaks"]);
	anyhow::ensure!(english_words("old-city-market") == ["old", "city", "market"]);
	anyhow::ensure!(english_words("None").is_empty());
	anyhow::ensure!(GeographicKind::PocketWater.english() == ["pocket", "water"]);
	Ok(())
}

#[test]
fn large_tiles_compose_three_to_twelve_languages() -> anyhow::Result<()> {
	for iz in -2..=2 {
		for ix in -2..=2 {
			let count = language_count(SEED, ix, iz);
			anyhow::ensure!((3..=12).contains(&count), "tile ({ix},{iz}) count {count}");
			let tile = LargeTile::generate(SEED, ix, iz);
			anyhow::ensure!(tile.languages.len() == count as usize);
			anyhow::ensure!(!tile.small.is_empty(), "tile ({ix},{iz}) has small tiles");
			for small in &tile.small {
				anyhow::ensure!(!small.language_ids.is_empty());
				anyhow::ensure!(small.language_ids.iter().all(|id| (*id as usize) < tile.languages.len()));
			}
		}
	}
	Ok(())
}

#[test]
fn neighboring_large_tiles_share_grammar_more_often_than_a_fresh_draw() -> anyhow::Result<()> {
	let a = LargeTile::generate(SEED, 0, 0);
	let neighbor = LargeTile::generate(SEED, 1, 0);
	let far = LargeTile::generate(SEED, 8, 8);
	let same_as_neighbor = a
		.languages
		.iter()
		.zip(neighbor.languages.iter())
		.filter(|(left, right)| left.grammar == right.grammar)
		.count();
	let same_as_far = a
		.languages
		.iter()
		.zip(far.languages.iter())
		.filter(|(left, right)| left.grammar == right.grammar)
		.count();
	anyhow::ensure!(
		same_as_neighbor >= same_as_far,
		"neighbor shares {same_as_neighbor} grammars, far shares {same_as_far}"
	);
	Ok(())
}

#[test]
fn tiles_and_names_are_deterministic() -> anyhow::Result<()> {
	let first = LargeTile::generate(SEED, 2, -1);
	let second = LargeTile::generate(SEED, 2, -1);
	anyhow::ensure!(first == second);

	let english = vec!["orchard".to_owned(), "hill".to_owned()];
	let name_a = PlaceName::translate(&first.languages[0], &english, 9);
	let name_b = PlaceName::translate(&second.languages[0], &english, 9);
	anyhow::ensure!(name_a == name_b);
	anyhow::ensure!(!name_a.surface.is_empty());
	anyhow::ensure!(!name_a.english.is_empty());
	Ok(())
}

#[test]
fn small_tiles_stay_inside_the_large_tile() -> anyhow::Result<()> {
	for (ix, iz) in [(0, 0), (1, -2), (-3, 4)] {
		let tile = LargeTile::generate(SEED, ix, iz);
		let parent = tile.bounds();
		anyhow::ensure!(
			(9..=25).contains(&tile.small.len()),
			"tile ({ix},{iz}) expected 9–25 small tiles, got {}",
			tile.small.len()
		);
		for small in &tile.small {
			anyhow::ensure!(small.bounds.min[0] >= parent.min[0] - 1e-3);
			anyhow::ensure!(small.bounds.min[1] >= parent.min[1] - 1e-3);
			anyhow::ensure!(small.bounds.max[0] <= parent.max[0] + 1e-3);
			anyhow::ensure!(small.bounds.max[1] <= parent.max[1] + 1e-3);
		}
	}
	Ok(())
}

#[test]
fn region_and_feature_names_assign_from_english_terms() -> anyhow::Result<()> {
	let mut index = LanguageIndex::default();
	let region = Aabb3d::from_min_max(
		bevy::math::Vec3::new(0.0, -1.0, 0.0),
		bevy::math::Vec3::new(LARGE_TILE, 1.0, LARGE_TILE),
	);
	let features = [NamedFeature {
		key: NameKey::Geographic(1),
		bounds: Aabb3d::from_min_max(
			bevy::math::Vec3::new(100.0, -1.0, 100.0),
			bevy::math::Vec3::new(200.0, 1.0, 200.0),
		),
		english: vec!["orchard".to_owned(), "river".to_owned(), "taiga".to_owned()],
	}];
	index.assign_keep(SEED, region, &features, &[]);
	let region_name = index
		.name(NameKey::Region { ix: 0, iz: 0 })
		.ok_or_else(|| anyhow::anyhow!("region name"))?;
	anyhow::ensure!(!region_name.surface.is_empty());
	let feature_name = index
		.name(NameKey::Geographic(1))
		.ok_or_else(|| anyhow::anyhow!("feature name"))?;
	anyhow::ensure!(!feature_name.surface.is_empty());
	anyhow::ensure!(index.language_at(Vec2::new(150.0, 150.0)).is_some());
	let _ = LanguageWorldSeed::default();
	Ok(())
}
