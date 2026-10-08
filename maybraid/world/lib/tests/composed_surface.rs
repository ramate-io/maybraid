//! Parity: the retired world surface formula equals
//! [`TerrainView<Urbanization<Richmond<OnTerrain<Durham>>>>`].

use bevy::ecs::system::SystemState;
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::World;
use durham::{
	BaseTerrainNoise, Durham, HcsgStorage, TerrainCellLayout, TerrainConfig, TerrainStorage,
	WorldBaseTerrain,
};
use lod::hcsg::universal_bounds;
use richmond::{
	pad::PadStage, register_richmond_nodes, AuthoredDevelopment, AuthoredDevelopments,
	DevelopmentConfig, DevelopmentKind, RichmondDevelopment,
};
use terrain_layer_model::{OnTerrain, TerrainView};
use urbanization_layer_model::Urbanization;

type Ground = OnTerrain<Durham>;

fn old_ground_height(
	store: &HcsgStorage,
	layout: &TerrainCellLayout,
	base: &WorldBaseTerrain,
	with_pads: bool,
	xz: Vec2,
) -> f32 {
	let raw = store
		.composed_height_at(layout, xz.x, xz.y)
		.unwrap_or_else(|| base.0.height_at(xz.x, xz.y));
	if !with_pads {
		return raw;
	}
	let probe = Aabb3d::from_min_max(
		Vec3::new(xz.x - 0.5, -10_000.0, xz.y - 0.5),
		Vec3::new(xz.x + 0.5, 10_000.0, xz.y + 0.5),
	);
	RichmondDevelopment::<Ground>::merged_pads(store, probe).modify_elevation(raw, xz.x, xz.y)
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
	let mut store = HcsgStorage::default();
	store.insert_base_terrain_for_test(&layout, 0, 0, base_noise.clone());

	let pad_bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(100.0, 100.0, 100.0));
	let authored = AuthoredDevelopment {
		cell: pad_bounds,
		kinds: vec![DevelopmentKind::LesHalles],
		height: 18.0,
		config: DevelopmentConfig::default(),
		courtyard: None,
	};
	let id = authored.id();
	register_richmond_nodes::<Ground>(&mut store);
	store.seed(DevelopmentConfig::default(), universal_bounds());
	store.seed(AuthoredDevelopments(vec![authored]), universal_bounds());
	let pad = store
		.get_one_or_generate::<RichmondDevelopment<Ground>>(id)
		.and_then(|development| development.pad_complexes().next().cloned())
		.ok_or_else(|| anyhow::anyhow!("les halles cell should carry a pad"))?;

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

	let (expected, pad_free) = {
		let store = world.resource::<HcsgStorage>();
		let layout = world.resource::<TerrainCellLayout>();
		let base = world.resource::<WorldBaseTerrain>();
		let expected: Vec<(Vec2, f32)> = [inside, skirt, outside, ungenerated]
			.into_iter()
			.map(|xz| (xz, old_ground_height(store, layout, base, true, xz)))
			.collect();
		let pad_free = old_ground_height(store, layout, base, false, inside);
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
