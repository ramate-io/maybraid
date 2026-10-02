//! Parity: the retired world surface formula equals
//! [`TerrainView<Urbanization<Richmond<OnTerrain<Durham>>>>`].

use bevy::ecs::system::SystemState;
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::World;
use durham::{
	BaseTerrainNoise, Durham, TerrainCellLayout, TerrainConfig, TerrainEntryStore, WorldBaseTerrain,
};
use richmond::{pad::PadStage, DevelopmentCell, DevelopmentConfig, DevelopmentEntryStore};
use terrain_layer_model::{OnTerrain, TerrainView};
use urbanization_cells::UrbanizationIndex;
use urbanization_layer_model::Urbanization;

fn old_ground_height(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	base: &WorldBaseTerrain,
	developments: &DevelopmentEntryStore,
	xz: Vec2,
) -> f32 {
	let raw = store
		.composed_height_at(layout, xz.x, xz.y)
		.unwrap_or_else(|| base.0.height_at(xz.x, xz.y));
	let probe = Aabb3d::from_min_max(
		Vec3::new(xz.x - 0.5, -10_000.0, xz.y - 0.5),
		Vec3::new(xz.x + 0.5, 10_000.0, xz.y + 0.5),
	);
	developments.merged_pad_complex(probe).modify_elevation(raw, xz.x, xz.y)
}

fn first_classified(pad: &richmond::PadComplex, stage: PadStage) -> Option<Vec2> {
	let mid = (pad.bounds.min + pad.bounds.max) * 0.5;
	let span = (pad.bounds.max.x - pad.bounds.min.x).max(1.0);
	(0..=80).map(|i| mid.x - span * 0.5 + span * i as f32 / 80.0).find_map(|x| {
		let xz = Vec2::new(x, mid.y);
		(pad.classification_at(xz.x, xz.y) == Some(stage)).then_some(xz)
	})
}

fn first_unclassified(pad: &richmond::PadComplex) -> Option<Vec2> {
	let mid_y = (pad.bounds.min.y + pad.bounds.max.y) * 0.5;
	(1..=40).find_map(|i| {
		let xz = Vec2::new(pad.bounds.max.x + i as f32, mid_y);
		pad.classification_at(xz.x, xz.y).is_none().then_some(xz)
	})
}

#[test]
fn terrain_view_ground_matches_the_retired_world_formula() -> anyhow::Result<()> {
	let layout = TerrainCellLayout::default();
	let base_noise = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
	let mut store = TerrainEntryStore::default();
	store.insert_base_terrain_for_test(&layout, 0, 0, base_noise.clone());

	let pad_bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(100.0, 100.0, 100.0));
	let development =
		DevelopmentCell::with_les_halles(pad_bounds, 18.0, &DevelopmentConfig::default());
	let pad = development
		.pad_complex()
		.cloned()
		.ok_or_else(|| anyhow::anyhow!("les halles cell should carry a pad"))?;
	let mut developments = DevelopmentEntryStore::default();
	developments.insert_cell(lod::gen::Id::from_cell(pad_bounds), development);

	let inside = first_classified(&pad, PadStage::Flatten)
		.ok_or_else(|| anyhow::anyhow!("pad should have a flatten terrace"))?;
	let skirt = first_classified(&pad, PadStage::Ease)
		.ok_or_else(|| anyhow::anyhow!("pad should have an ease skirt"))?;
	let outside = first_unclassified(&pad)
		.ok_or_else(|| anyhow::anyhow!("should find a point past the pad"))?;
	let ungenerated = Vec2::new(layout.cell_size * 8.0, layout.cell_size * 8.0);

	let mut world = World::new();
	world.insert_resource(store);
	world.insert_resource(layout.clone());
	world.insert_resource(WorldBaseTerrain(base_noise));
	world.insert_resource(developments);
	world.insert_resource(UrbanizationIndex::default());

	let (expected, pad_free) = {
		let store = world.resource::<TerrainEntryStore>();
		let layout = world.resource::<TerrainCellLayout>();
		let base = world.resource::<WorldBaseTerrain>();
		let developments = world.resource::<DevelopmentEntryStore>();
		let expected: Vec<(Vec2, f32)> = [inside, skirt, outside, ungenerated]
			.into_iter()
			.map(|xz| (xz, old_ground_height(store, layout, base, developments, xz)))
			.collect();
		let pad_free =
			old_ground_height(store, layout, base, &DevelopmentEntryStore::default(), inside);
		(expected, pad_free)
	};

	{
		let mut ground = SystemState::<
			TerrainView<Urbanization<richmond::Richmond<OnTerrain<Durham>>>>,
		>::new(&mut world);
		let view = ground.get(&world)?;
		for (xz, want) in expected {
			anyhow::ensure!(
				view.height_or_fallback(xz) == want,
				"composed surface at {xz:?}: view {} != old {want}",
				view.height_or_fallback(xz)
			);
		}
	}

	let mut durham = SystemState::<TerrainView<OnTerrain<Durham>>>::new(&mut world);
	let durham_view = durham.get(&world)?;
	anyhow::ensure!(
		durham_view.height_or_fallback(inside) == pad_free,
		"OnTerrain must stay pad-free at the terrace"
	);
	Ok(())
}
