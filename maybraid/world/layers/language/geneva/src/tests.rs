use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{App, MinimalPlugins, Res, Resource, Transform, World};
use bevy::state::app::StatesPlugin;
use chico::ForestGroveKind;
use durham::{GeographicBand, GeographicFamily, GeographicFeatureId, GeographicFeatureKind};
use language_layer_model::{Language, LanguageGeneration};
use layer_stack::{Generate, GenerationMode, GenerationModePlugin, Present, Scheme};
use lod::gen::Id;
use lod::lod_ref::LodRef;
use lod::LodPresentGate;
use richmond::{DiscoverablePlace, DiscoverablePlaceLabel};
use terrain_layer_model::{HeightField, OnTerrain, TerrainCell, TerrainGeneration, TerrainModel};

use crate::english::{
	compose_english, geographic_terms, grove_kind_terms, named_grove_english, with_color_name,
	PLACE_COLORS,
};
use crate::index::{LanguageConfig, LanguageIndex, LanguageSourceDeps, LanguageWorldSeed, NameKey};
use crate::name::{canonicalize_terms, pick_terms, terms_fingerprint, PlaceName};
use crate::present::LanguageOverlay;
use crate::sources::{
	places_from_world_xz, places_signature_from_world_xz, NamedFeature, NamedPlace, NamedWorld,
	SourceRevisions,
};
use crate::tiles::{LargeTile, LARGE_TILE};
use crate::Geneva;

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
fn regional_names_stay_provisional_and_refresh_when_inputs_change() -> anyhow::Result<()> {
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
	anyhow::ensure!(assigned.provisional);

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
	let second_assigned = index
		.assigned(NameKey::Region { ix: 0, iz: 0 })
		.ok_or_else(|| anyhow::anyhow!("second region"))?;
	anyhow::ensure!(second_assigned.provisional);
	anyhow::ensure!(
		second_assigned.fingerprint != terms_fingerprint(&first_name.english)
			|| second_assigned.name.english != first_name.english,
		"changed regional inputs must refresh the provisional assignment fingerprint"
	);
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
fn seed_change_clears_incompatible_names() -> anyhow::Result<()> {
	let mut app = App::new();
	app.insert_resource(LanguageWorldSeed(1));
	let mut index = LanguageIndex::default();
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
	app.insert_resource(index);
	Geneva::<RecordingWorld>::apply_generation(app.world_mut(), &LanguageConfig { seed: 99 });
	anyhow::ensure!(app.world().resource::<LanguageIndex>().names().next().is_none());
	anyhow::ensure!(app.world().resource::<LanguageWorldSeed>().0 == 99);
	Ok(())
}

#[derive(Clone)]
struct SilentField;

impl HeightField for SilentField {
	fn height_at(&self, _xz: Vec2) -> Option<f32> {
		None
	}
	fn fallback_height_at(&self, _xz: Vec2) -> f32 {
		0.0
	}
}

struct SilentCell;

impl TerrainCell for SilentCell {
	type Mesh = ();
	fn bounds(&self) -> Aabb3d {
		feature_aabb(0.0, 0.0, 1.0, 1.0)
	}
	fn mesh_builder(&self) {}
	fn chunk_pose(&self) -> Transform {
		Transform::IDENTITY
	}
	fn seeds_collision(&self) -> bool {
		false
	}
	fn res_2(&self) -> u8 {
		0
	}
}

#[derive(Resource, Default)]
struct RecordingSources {
	fingerprint: u64,
	features: Vec<NamedFeature>,
	places: Vec<NamedPlace>,
}

struct RecordingWorld;

impl NamedWorld for RecordingWorld {
	type Read = Res<'static, RecordingSources>;

	fn source_revisions(read: &SystemParamItem<'_, '_, Self::Read>) -> SourceRevisions {
		SourceRevisions { forest: read.fingerprint, urban: 0, terrain: 0, places: 0 }
	}

	fn groves_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<NamedFeature> {
		read.features.clone()
	}

	fn geography_overlapping(
		_read: &SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<NamedFeature> {
		Vec::new()
	}

	fn urban_overlapping(
		_read: &SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<NamedFeature> {
		Vec::new()
	}

	fn places_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<NamedPlace> {
		read.places.clone()
	}
}

impl TerrainModel for RecordingWorld {
	type Base = Self;
	type Cell = SilentCell;
	type Read = ();
	type Snapshot = SilentField;
	type Prepare = ();

	fn prepare(
		_prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
	}

	fn height_at(_read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> Option<f32> {
		None
	}

	fn fallback_height_at(_read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> f32 {
		0.0
	}

	fn overlay_cell<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_bounds: Aabb3d,
		_target_size: f32,
		_overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = ()>> {
		None
	}

	fn snapshot(_read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> SilentField {
		SilentField
	}

	fn require_generation(_app: &bevy::prelude::App) {}
}

impl TerrainGeneration for RecordingWorld {
	const LABEL: &'static str = "recording-ground";
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
	fn install_presentation(_app: &mut App) {}
}

struct HookMode;

impl GenerationMode for HookMode {}

impl Scheme<Language<Geneva<RecordingWorld>>> for HookMode {
	fn install(app: &mut App, _config: &LanguageConfig) {
		crate::install_language_stream::<HookMode, RecordingWorld>(app);
	}
}

#[test]
fn generation_and_presentation_hooks_refresh_overlay_only_when_open() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<HookMode>::initial(),
		layer_stack::LayerGenerationCore::<OnTerrain<RecordingWorld>>::default(),
		Generate::<HookMode, Language<Geneva<RecordingWorld>>>::new(
			LanguageConfig::world_defaults(),
		),
		Present::<HookMode, Language<Geneva<RecordingWorld>>>::default(),
	));
	app.insert_resource(RecordingSources {
		fingerprint: 1,
		features: vec![NamedFeature::new(
			NameKey::Forest(origin_cell(8.0, 8.0, 16.0, 16.0)),
			feature_aabb(8.0, 8.0, 16.0, 16.0),
			vec!["taiga".to_owned()],
			1,
		)],
		places: Vec::new(),
	});
	app.finish();
	app.update();
	anyhow::ensure!(
		app.world()
			.resource::<LanguageIndex>()
			.name(NameKey::Forest(origin_cell(8.0, 8.0, 16.0, 16.0)))
			.is_some(),
		"generation hook assigns from source records"
	);
	app.update();

	anyhow::ensure!(
		app.world().resource::<LodPresentGate<Language<Geneva<RecordingWorld>>>>().open
	);
	let overlay = app.world().resource::<LanguageOverlay>().clone();
	anyhow::ensure!(!overlay.names.is_empty(), "open gate materializes names");
	let epoch = app.world().resource::<LanguageIndex>().epoch;
	app.update();
	anyhow::ensure!(
		app.world().resource::<LanguageIndex>().epoch == epoch,
		"stationary sources must not rebuild assignment"
	);

	app.world_mut()
		.resource_mut::<LodPresentGate<Language<Geneva<RecordingWorld>>>>()
		.open = false;
	app.world_mut().resource_mut::<RecordingSources>().fingerprint = 2;
	app.world_mut().resource_mut::<RecordingSources>().features = vec![NamedFeature::new(
		NameKey::Grove(origin_cell(20.0, 20.0, 30.0, 30.0)),
		feature_aabb(20.0, 20.0, 30.0, 30.0),
		vec!["oak".to_owned()],
		1,
	)];
	app.update();
	anyhow::ensure!(
		app.world().resource::<LanguageOverlay>().names == overlay.names,
		"closed presentation gate must not refresh overlay"
	);
	Ok(())
}

#[test]
fn simultaneous_source_revisions_do_not_cancel() -> anyhow::Result<()> {
	let region = feature_aabb(0.0, 0.0, LARGE_TILE, LARGE_TILE);
	let first = LanguageSourceDeps::from_keep(
		SourceRevisions { forest: 1, urban: 1, terrain: 0, places: 0 },
		SEED,
		region,
	);
	let second = LanguageSourceDeps::from_keep(
		SourceRevisions { forest: 2, urban: 2, terrain: 0, places: 0 },
		SEED,
		region,
	);
	anyhow::ensure!(first != second, "paired revision bumps must stay distinct");
	let moved = LanguageSourceDeps::from_keep(
		first.revisions,
		SEED,
		feature_aabb(LARGE_TILE, LARGE_TILE, LARGE_TILE * 2.0, LARGE_TILE * 2.0),
	);
	anyhow::ensure!(first != moved, "keep-tile range is part of the dependency tuple");
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
fn place_deps_ignore_sub_quantum_transform_jitter() -> anyhow::Result<()> {
	let host = origin_cell(0.0, 0.0, 50.0, 50.0);
	let place =
		DiscoverablePlace::host(DiscoverablePlaceLabel::House, 8.0, 1.1).with_identity(host, 4);
	let a = places_signature_from_world_xz([(place.clone(), Vec3::new(12.0, 40.0, 18.0))]);
	let b = places_signature_from_world_xz([(place, Vec3::new(12.4, 41.0, 18.3))]);
	anyhow::ensure!(a == b, "host-keyed places must not dirty keep on pose jitter");

	let provisional = DiscoverablePlace::high(DiscoverablePlaceLabel::Room, 2.0, 1.0);
	let near = places_signature_from_world_xz([(provisional.clone(), Vec3::new(16.1, 0.0, 24.2))]);
	let still = places_signature_from_world_xz([(provisional.clone(), Vec3::new(19.4, 0.0, 27.6))]);
	let moved = places_signature_from_world_xz([(provisional, Vec3::new(40.0, 0.0, 40.0))]);
	anyhow::ensure!(near == still, "provisional places quantize XZ at 8 m");
	anyhow::ensure!(near != moved);
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
		SEED,
		region,
		crate::FeatureSnapshot {
			active: [forest, grove].into_iter().collect(),
			work: vec![NamedFeature::new(
				grove,
				feature_aabb(20.0, 20.0, 30.0, 30.0),
				vec!["oak".to_owned()],
				1,
			)],
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
