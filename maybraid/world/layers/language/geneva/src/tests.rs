use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use chico::ForestGroveKind;
use durham::terrain::watersheds::{PocketWater, PocketWatersHighPass};
use durham::terrain::MassifHighPassStampCell;
use durham::{
	Durham, GeographicBand, GeographicFamily, GeographicFeatureId, GeographicFeatureKind,
	WatershedBandPass,
};
use lod::gen::{Id, Version};
use lod::hcsg::shared::HcsgStorage;
use procedural_common::Bounds2;
use richmond::{DiscoverablePlace, DiscoverablePlaceLabel, Richmond};
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;

use crate::english::{
	compose_english, geographic_terms, grove_kind_terms, named_grove_english, named_region_english,
	with_color_name, PLACE_COLORS,
};
use crate::index::{LanguageIndex, LanguageSourceDeps, NameKey};
use crate::name::{canonicalize_terms, pick_terms, terms_fingerprint, PlaceName};
use crate::present::LanguageOverlay;
use crate::shared::LANGUAGE_NAME_RADIUS;
use crate::sources::{
	places_from_world_xz, NameSources, NamedFeature, NamedPlace, NamingRegion, SourceRevisions,
};
use crate::tiles::{LargeTile, LARGE_TILE};

type Urban = Urbanization<Richmond<OnTerrain<Durham>>>;

const SEED: u64 = 0xC0DE_F00D_A11A;

fn feature_aabb(min_x: f32, min_z: f32, max_x: f32, max_z: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(min_x, -1.0, min_z), Vec3::new(max_x, 1.0, max_z))
}

fn origin_cell(min_x: f32, min_z: f32, max_x: f32, max_z: f32) -> Id {
	Id::from_cell(feature_aabb(min_x, min_z, max_x, max_z))
}

#[test]
fn explicit_vocab_does_not_split_debug_idents() -> anyhow::Result<()> {
	let riparian = grove_kind_terms(ForestGroveKind::RiparianMix);
	anyhow::ensure!(riparian.contains(&"riparian".to_owned()));
	anyhow::ensure!(riparian
		.iter()
		.any(|word| ["grove", "stand", "gallery", "woods"].contains(&word.as_str())));
	let oak = grove_kind_terms(ForestGroveKind::RollingOaks);
	anyhow::ensure!(oak.contains(&"oak".to_owned()));
	anyhow::ensure!(oak
		.iter()
		.any(|word| ["grove", "stand", "copse", "woods"].contains(&word.as_str())));
	let stream = geographic_terms(GeographicFeatureKind::StreamsGraph);
	anyhow::ensure!(stream.contains(&"stream".to_owned()));
	let massif = geographic_terms(GeographicFeatureKind::Massif);
	anyhow::ensure!(
		massif.contains(&"massif".to_owned()) && massif.contains(&"mountain".to_owned())
	);
	Ok(())
}

#[test]
fn grove_english_keeps_a_noun_head() -> anyhow::Result<()> {
	let phrase = named_grove_english([ForestGroveKind::RiparianMix], 7);
	anyhow::ensure!(phrase.first().is_some_and(|word| PLACE_COLORS.contains(&word.as_str())));
	anyhow::ensure!(
		phrase
			.last()
			.is_some_and(|word| ["grove", "stand", "gallery", "woods"].contains(&word.as_str())),
		"expected a grove noun, got {phrase:?}"
	);
	anyhow::ensure!(phrase.len() >= 2);
	let rolling = compose_english(&["rolling"], &["hills", "downs"], 3, false);
	anyhow::ensure!(
		rolling.last() == Some(&"hills".to_owned()) || rolling.last() == Some(&"downs".to_owned())
	);
	Ok(())
}

#[test]
fn color_names_vary_by_seed_and_keep_the_kind() -> anyhow::Result<()> {
	let first = with_color_name(vec!["bush".to_owned()], 1);
	anyhow::ensure!(first.len() == 2 && first[1] == "bush");
	anyhow::ensure!(PLACE_COLORS.contains(&first[0].as_str()));
	let colors: std::collections::HashSet<_> = (0..48)
		.map(|seed| with_color_name(vec!["bush".to_owned()], seed)[0].clone())
		.collect();
	anyhow::ensure!(colors.len() > 4, "expected several colors across seeds, got {colors:?}");
	Ok(())
}

#[test]
fn shuffled_terms_canonicalize_before_seeded_pick() -> anyhow::Result<()> {
	let a = ["taiga".to_owned(), "orchard".to_owned(), "river".to_owned()];
	let b = ["river".to_owned(), "orchard".to_owned(), "taiga".to_owned()];
	anyhow::ensure!(canonicalize_terms(&a) == canonicalize_terms(&b));
	anyhow::ensure!(pick_terms(&a, 9) == pick_terms(&b, 9));
	anyhow::ensure!(terms_fingerprint(&a) == terms_fingerprint(&b));
	Ok(())
}

#[test]
fn large_tiles_compose_three_to_twelve_languages() -> anyhow::Result<()> {
	for iz in -2..=2 {
		for ix in -2..=2 {
			let tile = LargeTile::generate(SEED, ix, iz);
			anyhow::ensure!((3..=12).contains(&tile.languages.len()));
			anyhow::ensure!(!tile.small.is_empty());
			for small in &tile.small {
				anyhow::ensure!(!small.language_ids.is_empty());
				anyhow::ensure!(small
					.language_ids
					.iter()
					.all(|id| (*id as usize) < tile.languages.len()));
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
	Ok(())
}

#[test]
fn small_tiles_stay_inside_the_large_tile() -> anyhow::Result<()> {
	for (ix, iz) in [(0, 0), (1, -2), (-3, 4)] {
		let tile = LargeTile::generate(SEED, ix, iz);
		let parent = tile.bounds();
		anyhow::ensure!((9..=25).contains(&tile.small.len()));
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
fn language_subset_uses_feature_key_not_lowest_id() -> anyhow::Result<()> {
	let mut index = LanguageIndex::default();
	index.ensure_tiles(SEED, feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE));
	let tile = index.large_tile(0, 0).ok_or_else(|| anyhow::anyhow!("tile"))?;
	let small = tile
		.small
		.iter()
		.find(|leaf| leaf.language_ids.len() > 1)
		.ok_or_else(|| anyhow::anyhow!("subset with more than one language"))?;
	let center = Vec2::new(
		(small.bounds.min[0] + small.bounds.max[0]) * 0.5,
		(small.bounds.min[1] + small.bounds.max[1]) * 0.5,
	);
	let first = tile
		.language(*small.language_ids.first().ok_or_else(|| anyhow::anyhow!("id"))?)
		.ok_or_else(|| anyhow::anyhow!("first language"))?
		.seed;
	let mut saw_other = false;
	for salt in 0..64u64 {
		let bundle = index
			.language_at_key(center, salt.wrapping_mul(0x9E37))
			.ok_or_else(|| anyhow::anyhow!("bundle"))?;
		if bundle.seed != first {
			saw_other = true;
			break;
		}
	}
	anyhow::ensure!(saw_other, "subset pick must not always use the lowest language id");
	Ok(())
}

#[test]
fn regional_names_come_from_seed_not_streamed_features() -> anyhow::Result<()> {
	let mut index = LanguageIndex::default();
	let region = feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE);
	let first = [NamedFeature::new(
		NameKey::Geographic(GeographicFeatureId {
			family: GeographicFamily::Watershed,
			band: GeographicBand::HighPass,
			source: origin_cell(100.0, 100.0, 200.0, 200.0),
		}),
		feature_aabb(100.0, 100.0, 200.0, 200.0),
		vec!["lake".to_owned()],
		1,
	)];
	index.assign_keep(SEED, region, &first, &[]);
	let first_name = index
		.name(NameKey::Region { ix: 0, iz: 0 })
		.ok_or_else(|| anyhow::anyhow!("first region"))?
		.clone();
	let assigned = index
		.assigned(NameKey::Region { ix: 0, iz: 0 })
		.ok_or_else(|| anyhow::anyhow!("assigned"))?;
	anyhow::ensure!(!assigned.provisional);
	anyhow::ensure!(!first_name.english.is_empty());
	anyhow::ensure!(!named_region_english(SEED, 0, 0).is_empty());

	let second = [
		first[0].clone(),
		NamedFeature::new(
			NameKey::Geographic(GeographicFeatureId {
				family: GeographicFamily::Massif,
				band: GeographicBand::HighPass,
				source: origin_cell(300.0, 300.0, 400.0, 400.0),
			}),
			feature_aabb(300.0, 300.0, 400.0, 400.0),
			vec!["massif".to_owned(), "mountain".to_owned()],
			1,
		),
	];
	index.assign_keep(SEED, region, &second, &[]);
	let second_name = index
		.name(NameKey::Region { ix: 0, iz: 0 })
		.ok_or_else(|| anyhow::anyhow!("second region"))?;
	anyhow::ensure!(second_name == &first_name, "region names must ignore streamed feature terms");
	Ok(())
}

#[test]
fn overlay_copies_name_anchors() -> anyhow::Result<()> {
	let mut index = LanguageIndex::default();
	let region = feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE);
	let key = NameKey::Geographic(GeographicFeatureId {
		family: GeographicFamily::Plateau,
		band: GeographicBand::LowPass,
		source: origin_cell(10.0, 10.0, 20.0, 20.0),
	});
	index.assign_keep(
		SEED,
		region,
		&[NamedFeature::new(
			key,
			feature_aabb(10.0, 10.0, 20.0, 20.0),
			vec!["plateau".to_owned()],
			1,
		)],
		&[],
	);
	let overlay = LanguageOverlay::from_index(&index);
	let feature = overlay
		.names
		.iter()
		.find(|name| name.key == key)
		.ok_or_else(|| anyhow::anyhow!("feature overlay"))?;
	anyhow::ensure!((feature.xz - Vec2::new(15.0, 15.0)).length() < 1e-3);
	anyhow::ensure!(feature.english == ["plateau".to_owned()]);
	anyhow::ensure!((feature.extent.min - Vec2::new(10.0, 10.0)).length() < 1e-3);
	anyhow::ensure!((feature.extent.max - Vec2::new(20.0, 20.0)).length() < 1e-3);
	let region_name = overlay
		.names
		.iter()
		.find(|name| matches!(name.key, NameKey::Region { ix: 0, iz: 0 }))
		.ok_or_else(|| anyhow::anyhow!("region overlay"))?;
	anyhow::ensure!((region_name.xz - Vec2::splat(LARGE_TILE * 0.5)).length() < 1e-3);
	anyhow::ensure!((region_name.extent.min - Vec2::ZERO).length() < 1e-3);
	anyhow::ensure!((region_name.extent.max - Vec2::splat(LARGE_TILE)).length() < 1e-3);
	Ok(())
}

#[test]
fn source_revision_invalidates_feature_names_unload_does_not() -> anyhow::Result<()> {
	let mut index = LanguageIndex::default();
	let region = feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE);
	let key = NameKey::Geographic(GeographicFeatureId {
		family: GeographicFamily::Plateau,
		band: GeographicBand::LowPass,
		source: origin_cell(10.0, 10.0, 20.0, 20.0),
	});
	let first =
		NamedFeature::new(key, feature_aabb(10.0, 10.0, 20.0, 20.0), vec!["plateau".to_owned()], 1);
	index.assign_keep(SEED, region, &[first.clone()], &[]);
	let first_name = index.name(key).ok_or_else(|| anyhow::anyhow!("first"))?.clone();

	index.assign_keep(SEED, region, &[], &[]);
	anyhow::ensure!(index.name(key) == Some(&first_name), "visual unload must keep the name");

	let restamp =
		NamedFeature::new(key, feature_aabb(10.0, 10.0, 20.0, 20.0), vec!["canyon".to_owned()], 2);
	index.assign_keep(SEED, region, &[restamp], &[]);
	let restamped = index.name(key).ok_or_else(|| anyhow::anyhow!("restamp"))?;
	anyhow::ensure!(
		restamped.english != first_name.english || restamped.surface != first_name.surface
	);
	Ok(())
}

#[test]
fn parented_elevated_pois_use_world_xz_and_host_identity() -> anyhow::Result<()> {
	let region = feature_aabb(0.0, 0.0, 100.0, 100.0);
	let host = origin_cell(0.0, 0.0, 50.0, 50.0);
	let host_place =
		DiscoverablePlace::host(DiscoverablePlaceLabel::House, 8.0, 1.1).with_identity(host, 4);
	let interior =
		DiscoverablePlace::high(DiscoverablePlaceLabel::Lounge, 3.0, 1.1).with_identity(host, 99);
	let elevated = places_from_world_xz(
		[
			(host_place, Vec3::new(12.0, 40.0, 18.0)),
			(interior, Vec3::new(12.5, 48.0, 18.5)),
			(
				DiscoverablePlace::high(DiscoverablePlaceLabel::Room, 2.0, 1.0),
				Vec3::new(200.0, 0.0, 200.0),
			),
		],
		region,
	);
	anyhow::ensure!(elevated.len() == 2, "Y slab must not exclude elevated POIs");
	anyhow::ensure!(elevated[0].key == NameKey::Place { host, local: 4 });
	anyhow::ensure!(elevated[1].key == NameKey::Place { host, local: 99 });
	anyhow::ensure!(!elevated[0].provisional);
	anyhow::ensure!(elevated[1].inherit_host_language);

	let mut index = LanguageIndex::default();
	index.assign_keep(SEED, region, &[], &elevated);
	let host_name = index
		.name(NameKey::Place { host, local: 4 })
		.ok_or_else(|| anyhow::anyhow!("host name"))?;
	let interior_name = index
		.name(NameKey::Place { host, local: 99 })
		.ok_or_else(|| anyhow::anyhow!("interior name"))?;
	anyhow::ensure!(host_name.language_seed == interior_name.language_seed);
	Ok(())
}

#[test]
fn a_new_root_version_clears_incompatible_names() -> anyhow::Result<()> {
	let mut index = LanguageIndex::default();
	index.begin_session(Version(1));
	index.assign_keep(
		1,
		feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE),
		&[NamedFeature::new(
			NameKey::Forest(origin_cell(0.0, 0.0, 10.0, 10.0)),
			feature_aabb(0.0, 0.0, 10.0, 10.0),
			vec!["taiga".to_owned()],
			1,
		)],
		&[],
	);
	anyhow::ensure!(index.name(NameKey::Forest(origin_cell(0.0, 0.0, 10.0, 10.0))).is_some());
	index.begin_session(Version(1));
	anyhow::ensure!(index.names().next().is_some(), "the same root keeps its names");
	index.begin_session(Version(2));
	anyhow::ensure!(index.names().next().is_none());
	anyhow::ensure!(index.large_tiles().next().is_none());
	anyhow::ensure!(index.session() == Some(Version(2)));
	index.end_session();
	anyhow::ensure!(index.session().is_none());
	Ok(())
}

fn massif(cell: Aabb3d, occupied: bool) -> MassifHighPassStampCell {
	use terrain_stamps::{CircleRegion, Region2D, RegionAffineModulation, StampModulation};
	let modulation = StampModulation::Affine(RegionAffineModulation::new(
		Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius: 8.0 }),
		1.0,
		0.0,
		4.0,
		8.0,
	));
	MassifHighPassStampCell { cell, modulations: if occupied { vec![modulation] } else { Vec::new() } }
}

fn shared_region(origin: Vec2) -> NamingRegion {
	NamingRegion::around(origin, LANGUAGE_NAME_RADIUS + 16.0, LANGUAGE_NAME_RADIUS + 96.0)
}

#[test]
fn shared_geography_names_occupied_stamps_and_each_authored_lake_once() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let occupied = feature_aabb(0.0, 0.0, 100.0, 100.0);
	let empty = feature_aabb(200.0, 200.0, 300.0, 300.0);
	storage.publish(Id::from_cell(occupied), Arc::new(massif(occupied, true)), occupied);
	storage.publish(Id::from_cell(empty), Arc::new(massif(empty, false)), empty);
	let lake_cell = feature_aabb(-300.0, -300.0, -100.0, -100.0);
	let lake = terrain_watersheds::Lake::from_bounds(
		Bounds2::from_xz(-260.0, -260.0, -140.0, -140.0),
		7,
		terrain_watersheds::LakeParams::default(),
		None,
	)
	.ok_or_else(|| anyhow::anyhow!("authored lake"))?;
	storage.publish(
		Id::from_cell(lake_cell),
		Arc::new(PocketWatersHighPass {
			cell: lake_cell,
			band: WatershedBandPass::High,
			authored: PocketWater::Lake(lake),
		}),
		lake_cell,
	);

	let sources = NameSources::<Urban>::new(&storage, None);
	let snapshot = sources
		.geography(shared_region(Vec2::ZERO), &LanguageIndex::default())
		.map_err(|_| anyhow::anyhow!("busy"))?;
	let mut kinds: Vec<_> = snapshot
		.work
		.iter()
		.map(|feature| match feature.key {
			NameKey::Geographic(id) => Some((id.family, id.source)),
			_ => None,
		})
		.collect();
	kinds.sort_by_key(|kind| kind.map(|(family, _)| family as u8));
	anyhow::ensure!(
		kinds
			== [
				Some((GeographicFamily::Massif, Id::from_cell(occupied))),
				Some((GeographicFamily::Watershed, Id::from_cell(lake_cell))),
			],
		"one name per occupied stamp and authored lake, got {kinds:?}"
	);
	let lake = snapshot
		.work
		.iter()
		.find(|feature| matches!(feature.key, NameKey::Geographic(id) if id.family == GeographicFamily::Watershed))
		.ok_or_else(|| anyhow::anyhow!("lake"))?;
	anyhow::ensure!(lake.bounds.min.x == -260.0 && lake.bounds.max.z == -140.0, "authored bounds");
	Ok(())
}

#[test]
fn unrelated_storage_writes_do_not_move_the_geography_revision() -> anyhow::Result<()> {
	struct Furniture;
	let storage = HcsgStorage::default();
	let revisions = |storage: &HcsgStorage| {
		NameSources::<Urban>::new(storage, None).revisions().map_err(|_| anyhow::anyhow!("busy"))
	};
	let before = revisions(&storage)?;
	let bounds = feature_aabb(0.0, 0.0, 160.0, 160.0);
	storage.publish(Id::from_cell(bounds), Arc::new(Furniture), bounds);
	let after = revisions(&storage)?;
	anyhow::ensure!(after == before, "furniture writes must not trigger rediscovery");
	storage.publish(Id::from_cell(bounds), Arc::new(massif(bounds, true)), bounds);
	let stamped = revisions(&storage)?;
	anyhow::ensure!(stamped.terrain > after.terrain, "a stamp write is a source change");
	anyhow::ensure!(stamped.urban == after.urban && stamped.forest == after.forest);
	Ok(())
}

#[test]
fn simultaneous_source_revisions_do_not_cancel() -> anyhow::Result<()> {
	let region = feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE);
	let first = LanguageSourceDeps::from_windows(
		SourceRevisions { forest: 1, urban: 1, terrain: 0, places: 0, tiles: 0 },
		SEED,
		region,
		Vec2::ZERO,
	);
	let second = LanguageSourceDeps::from_windows(
		SourceRevisions { forest: 2, urban: 2, terrain: 0, places: 0, tiles: 0 },
		SEED,
		region,
		Vec2::ZERO,
	);
	anyhow::ensure!(first != second, "paired revision bumps must stay distinct");
	let moved = LanguageSourceDeps::from_windows(
		first.revisions,
		SEED,
		feature_aabb(LARGE_TILE, LARGE_TILE, LARGE_TILE * 2.0, LARGE_TILE * 2.0),
		Vec2::ZERO,
	);
	anyhow::ensure!(first != moved, "keep-tile range is part of the dependency tuple");
	let naming_moved =
		LanguageSourceDeps::from_windows(first.revisions, SEED, region, Vec2::new(64.0, 0.0));
	anyhow::ensure!(first != naming_moved, "naming window is part of the dependency tuple");
	let admitted = LanguageSourceDeps::from_windows(
		SourceRevisions { tiles: 1, ..first.revisions },
		SEED,
		region,
		Vec2::ZERO,
	);
	anyhow::ensure!(!first.tiles_match(&admitted), "admitted tiles retry tile-less features");
	Ok(())
}

#[test]
fn overlay_shows_only_active_keep_names() -> anyhow::Result<()> {
	let mut index = LanguageIndex::default();
	let region = feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE);
	let key = NameKey::Geographic(GeographicFeatureId {
		family: GeographicFamily::Plateau,
		band: GeographicBand::LowPass,
		source: origin_cell(10.0, 10.0, 20.0, 20.0),
	});
	index.assign_keep(
		SEED,
		region,
		&[NamedFeature::new(
			key,
			feature_aabb(10.0, 10.0, 20.0, 20.0),
			vec!["plateau".to_owned()],
			1,
		)],
		&[],
	);
	anyhow::ensure!(LanguageOverlay::from_index(&index).names.iter().any(|name| name.key == key));

	index.assign_keep(SEED, region, &[], &[]);
	anyhow::ensure!(index.name(key).is_some(), "durable assignment survives unload");
	anyhow::ensure!(
		LanguageOverlay::from_index(&index).names.iter().all(|name| name.key != key),
		"inactive names stay off the map overlay"
	);
	Ok(())
}

#[test]
fn superseded_provisional_place_is_retired() -> anyhow::Result<()> {
	let mut index = LanguageIndex::default();
	let region = feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE);
	let provisional = NameKey::ProvisionalPlace { qx: 12, qz: 18, label: 1 };
	let host = origin_cell(0.0, 0.0, 50.0, 50.0);
	index.assign_keep(
		SEED,
		region,
		&[],
		&[NamedPlace {
			key: provisional,
			xz: Vec2::new(12.0, 18.0),
			english: vec!["house".to_owned()],
			persistent: false,
			revision: 1,
			fingerprint: terms_fingerprint(&["house".to_owned()]),
			provisional: true,
			host: None,
			inherit_host_language: false,
		}],
	);
	anyhow::ensure!(index.name(provisional).is_some());

	let durable = NameKey::Place { host, local: 4 };
	index.assign_keep(
		SEED,
		region,
		&[],
		&[NamedPlace {
			key: durable,
			xz: Vec2::new(12.0, 18.0),
			english: vec!["house".to_owned()],
			persistent: true,
			revision: 1,
			fingerprint: terms_fingerprint(&["house".to_owned()]),
			provisional: false,
			host: Some(host),
			inherit_host_language: false,
		}],
	);
	anyhow::ensure!(index.name(durable).is_some());
	anyhow::ensure!(index.name(provisional).is_none(), "replaced provisional keys must retire");
	Ok(())
}

#[test]
fn child_assigned_before_host_adopts_the_host_language() -> anyhow::Result<()> {
	let region = feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE);
	let host = origin_cell(0.0, 0.0, 50.0, 50.0);
	let host_place =
		DiscoverablePlace::host(DiscoverablePlaceLabel::House, 8.0, 1.1).with_identity(host, 4);
	let interior =
		DiscoverablePlace::high(DiscoverablePlaceLabel::Lounge, 3.0, 1.1).with_identity(host, 99);
	let child_only = places_from_world_xz([(interior, Vec3::new(12.5, 48.0, 18.5))], region);
	let both = places_from_world_xz(
		[(host_place, Vec3::new(12.0, 40.0, 18.0)), (interior, Vec3::new(12.5, 48.0, 18.5))],
		region,
	);

	let mut index = LanguageIndex::default();
	index.assign_keep(SEED, region, &[], &child_only);
	let child_key = NameKey::Place { host, local: 99 };
	let host_key = NameKey::Place { host, local: 4 };
	let first = index.assigned(child_key).ok_or_else(|| anyhow::anyhow!("child first"))?.clone();

	index.assign_keep(SEED, region, &[], &both);
	let host_name = index.assigned(host_key).ok_or_else(|| anyhow::anyhow!("host"))?;
	let child = index.assigned(child_key).ok_or_else(|| anyhow::anyhow!("child later"))?;
	anyhow::ensure!(child.name.language_seed == host_name.name.language_seed);
	anyhow::ensure!(child.inherited_language == Some(host_name.name.language_seed));
	anyhow::ensure!(
		first.inherited_language != child.inherited_language
			|| first.name.language_seed == child.name.language_seed
	);
	Ok(())
}

#[test]
fn moved_place_updates_anchor_without_retranslation() -> anyhow::Result<()> {
	let mut index = LanguageIndex::default();
	let region = feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE);
	let host = origin_cell(0.0, 0.0, 50.0, 50.0);
	let key = NameKey::Place { host, local: 4 };
	let first = NamedPlace {
		key,
		xz: Vec2::new(12.0, 18.0),
		english: vec!["house".to_owned()],
		persistent: true,
		revision: 1,
		fingerprint: terms_fingerprint(&["house".to_owned()]),
		provisional: false,
		host: Some(host),
		inherit_host_language: false,
	};
	index.assign_keep(SEED, region, &[], &[first.clone()]);
	let name = index.name(key).ok_or_else(|| anyhow::anyhow!("name"))?.clone();
	let mut moved = first;
	moved.xz = Vec2::new(40.0, 18.0);
	index.queue_keep(SEED, region, &[], &[moved]);
	index.assign_budgeted(SEED, 32);
	anyhow::ensure!(index.name(key) == Some(&name), "identity-stable move must keep the name");
	anyhow::ensure!(index.anchor(key) == Some(Vec2::new(40.0, 18.0)));
	Ok(())
}

#[test]
fn repeat_keep_does_not_requeue_current_names() -> anyhow::Result<()> {
	let mut index = LanguageIndex::default();
	let region = feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE);
	let key = NameKey::Forest(origin_cell(0.0, 0.0, 10.0, 10.0));
	let feature =
		NamedFeature::new(key, feature_aabb(0.0, 0.0, 10.0, 10.0), vec!["taiga".to_owned()], 1);
	index.assign_keep(SEED, region, &[feature.clone()], &[]);
	let epoch = index.epoch;
	index.queue_keep(SEED, region, &[feature], &[]);
	anyhow::ensure!(index.epoch == epoch, "unchanged keep must not bump epoch");
	index.assign_budgeted(SEED, 32);
	anyhow::ensure!(index.epoch == epoch);
	Ok(())
}

#[test]
fn overlay_dirty_upserts_without_dropping_other_names() -> anyhow::Result<()> {
	let mut index = LanguageIndex::default();
	let region = feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE);
	let forest = NameKey::Forest(origin_cell(0.0, 0.0, 10.0, 10.0));
	index.assign_keep(
		SEED,
		region,
		&[NamedFeature::new(
			forest,
			feature_aabb(0.0, 0.0, 10.0, 10.0),
			vec!["taiga".to_owned()],
			1,
		)],
		&[],
	);
	let mut overlay = LanguageOverlay::from_index(&index);
	let grove = NameKey::Grove(origin_cell(20.0, 20.0, 30.0, 30.0));
	index.queue_feature_snapshot(
		crate::FeatureSnapshot {
			active: [forest, grove].into_iter().collect(),
			work: vec![NamedFeature::new(
				grove,
				feature_aabb(20.0, 20.0, 30.0, 30.0),
				vec!["oak".to_owned()],
				1,
			)],
			moved: Vec::new(),
		},
		crate::index::SourceClass::Vegetation,
	);
	index.assign_budgeted(SEED, 32);
	let dirty = index.take_overlay_dirty();
	overlay.apply_dirty(&index, dirty);
	anyhow::ensure!(overlay.names.iter().any(|name| name.key == forest));
	anyhow::ensure!(overlay.names.iter().any(|name| name.key == grove));
	Ok(())
}

fn naming_region(origin: Vec2) -> NamingRegion {
	NamingRegion::around(origin, LANGUAGE_NAME_RADIUS + 16.0, LANGUAGE_NAME_RADIUS + 96.0)
}

#[test]
fn nearby_movement_names_existing_features_without_source_changes() -> anyhow::Result<()> {
	let near = NameKey::Forest(origin_cell(0.0, 0.0, 10.0, 10.0));
	let ahead = NameKey::Grove(origin_cell(500.0, 0.0, 510.0, 10.0));
	let near_feature =
		NamedFeature::new(near, feature_aabb(0.0, 0.0, 10.0, 10.0), vec!["taiga".to_owned()], 1);
	let ahead_feature =
		NamedFeature::new(ahead, feature_aabb(500.0, 0.0, 510.0, 10.0), vec!["oak".to_owned()], 1);
	let mut index = LanguageIndex::default();
	index.ensure_tiles(SEED, feature_aabb(-LARGE_TILE, -LARGE_TILE, LARGE_TILE, LARGE_TILE));
	index.queue_feature_snapshot(
		crate::sources::features_to_snapshot(
			vec![near_feature.clone(), ahead_feature.clone()],
			&index,
			naming_region(Vec2::ZERO),
		),
		crate::index::SourceClass::Vegetation,
	);
	index.assign_budgeted(SEED, 32);
	anyhow::ensure!(index.name(near).is_some());
	anyhow::ensure!(index.name(ahead).is_none(), "500 m grove stays outside the first window");

	index.queue_feature_snapshot(
		crate::sources::features_to_snapshot(
			vec![near_feature, ahead_feature],
			&index,
			naming_region(Vec2::new(350.0, 0.0)),
		),
		crate::index::SourceClass::Vegetation,
	);
	index.assign_budgeted(SEED, 32);
	anyhow::ensure!(index.name(ahead).is_some(), "moving the window must name the existing grove");
	Ok(())
}

#[test]
fn distant_feature_is_not_collected() -> anyhow::Result<()> {
	let distant = NameKey::Forest(origin_cell(20_000.0, 0.0, 20_010.0, 10.0));
	let feature = NamedFeature::new(
		distant,
		feature_aabb(20_000.0, 0.0, 20_010.0, 10.0),
		vec!["taiga".to_owned()],
		1,
	);
	let region = naming_region(Vec2::ZERO);
	anyhow::ensure!(!region.collects_bounds(feature.bounds));
	let mut index = LanguageIndex::default();
	index.queue_feature_snapshot(
		crate::sources::features_to_snapshot(vec![feature], &index, region),
		crate::index::SourceClass::Vegetation,
	);
	index.assign_budgeted(SEED, 32);
	anyhow::ensure!(index.name(distant).is_none());
	anyhow::ensure!(!index.is_active(distant));
	Ok(())
}

#[test]
fn intersecting_feature_with_distant_center_is_named() -> anyhow::Result<()> {
	let key = NameKey::Geographic(GeographicFeatureId {
		family: GeographicFamily::Massif,
		band: GeographicBand::HighPass,
		source: origin_cell(0.0, 0.0, 8_000.0, 200.0),
	});
	let feature = NamedFeature::new(
		key,
		feature_aabb(0.0, 0.0, 8_000.0, 200.0),
		vec!["massif".to_owned()],
		1,
	);
	let region = naming_region(Vec2::ZERO);
	anyhow::ensure!(region.collects_bounds(feature.bounds), "bounds reach the player");
	anyhow::ensure!(feature_aabb(0.0, 0.0, 8_000.0, 200.0).min.x < 400.0);
	let mut index = LanguageIndex::default();
	index.assign_keep(
		SEED,
		feature_aabb(-LARGE_TILE, -LARGE_TILE, LARGE_TILE, LARGE_TILE),
		&[feature],
		&[],
	);
	anyhow::ensure!(index.name(key).is_some());
	Ok(())
}

#[test]
fn feature_name_is_independent_of_visit_order() -> anyhow::Result<()> {
	let region = feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE);
	let forest = NamedFeature::new(
		NameKey::Forest(origin_cell(0.0, 0.0, 10.0, 10.0)),
		feature_aabb(0.0, 0.0, 10.0, 10.0),
		vec!["taiga".to_owned()],
		1,
	);
	let grove = NamedFeature::new(
		NameKey::Grove(origin_cell(20.0, 20.0, 30.0, 30.0)),
		feature_aabb(20.0, 20.0, 30.0, 30.0),
		vec!["oak".to_owned()],
		1,
	);
	let mut first = LanguageIndex::default();
	first.assign_keep(SEED, region, &[forest.clone()], &[]);
	first.assign_keep(SEED, region, &[forest.clone(), grove.clone()], &[]);
	let mut second = LanguageIndex::default();
	second.assign_keep(SEED, region, &[grove.clone()], &[]);
	second.assign_keep(SEED, region, &[forest, grove], &[]);
	anyhow::ensure!(
		first.name(NameKey::Forest(origin_cell(0.0, 0.0, 10.0, 10.0)))
			== second.name(NameKey::Forest(origin_cell(0.0, 0.0, 10.0, 10.0)))
	);
	anyhow::ensure!(
		first.name(NameKey::Grove(origin_cell(20.0, 20.0, 30.0, 30.0)))
			== second.name(NameKey::Grove(origin_cell(20.0, 20.0, 30.0, 30.0)))
	);
	anyhow::ensure!(
		first.name(NameKey::Region { ix: 0, iz: 0 })
			== second.name(NameKey::Region { ix: 0, iz: 0 })
	);
	Ok(())
}
