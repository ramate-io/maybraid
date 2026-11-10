use super::*;
use crate::{
	ChicoGrove, ChicoGroveHost, ForestGroveKind, ForestGroveRecipe, ForestLayer, ForestSelection,
	LayeringKind, grove_id,
};
use bevy::scene::{ScenePlugin, WorldSceneExt};
use bevy::shape::Aabb3d;
use lod::LodScene;
use lod::hcsg::{GenerationContext, HcsgStorage, universal_bounds};
use std::sync::Arc;
use vegetation_components::{
	FlattenedComponentsOnly, PlacedVegetation, VegetationComponents, VegetationInstance,
};
use vegetation_groves::{FlatTerrainSample, GroveExtent};

type Source = FlattenedComponentsOnly<PlacedVegetation<Arc<sbs_trees::StorybookTree>>>;

fn fixture(mode: &str) -> anyhow::Result<VegetationInstanceFixture> {
	VegetationInstanceFixture::parse(Some("0,0"), Some(mode)).map_err(anyhow::Error::msg)
}
fn storage(fixture: VegetationInstanceFixture) -> HcsgStorage {
	let storage = HcsgStorage::default();
	storage.seed(
		ForestSelection { layering: Some(LayeringKind::AgTown), ..default() },
		universal_bounds(),
	);
	storage.seed(fixture, universal_bounds());
	storage
}

#[test]
fn exact_recipe_override_is_independent_of_renderer_and_leaves_other_ids_unchanged()
-> anyhow::Result<()> {
	let ordinary = fixture("ordinary")?;
	let persistent = fixture("persistent")?;
	let natural = storage(VegetationInstanceFixture::default());
	let a = storage(ordinary);
	let b = storage(persistent);
	let selected = ordinary.extent().ok_or_else(|| anyhow::anyhow!("extent absent"))?;
	let neighbor = GroveExtent::new(Vec3::new(100.0, 0.0, 0.0), Vec3::new(200.0, 1.0, 100.0));
	for extent in [selected, neighbor] {
		for layer in ForestLayer::ALL {
			let id = grove_id(extent, layer);
			let get = |storage: &HcsgStorage| {
				GenerationContext::new(storage)
					.get_or_generate::<ChicoGrove>(id)
					.ok_or_else(|| anyhow::anyhow!("grove absent"))
			};
			let before = get(&natural)?;
			let after = get(&a)?;
			let packed = get(&b)?;
			let kinds = |grove: &ChicoGrove| {
				grove
					.recipes
					.iter()
					.map(|recipe| (recipe.kind, recipe.cells.as_ref().map(Vec::len)))
					.collect::<Vec<_>>()
			};
			assert_eq!(kinds(&after), kinds(&packed));
			if ordinary.selects(extent, layer) {
				assert_eq!(after.recipes.len(), 1);
				assert_eq!(after.recipes[0].kind, ForestGroveKind::Orchard);
				assert!(after.recipes[0].cells.is_none());
			} else {
				assert_eq!(kinds(&before), kinds(&after));
			}
			assert_eq!(after.id(), packed.id());
		}
	}
	let outside = Aabb3d::from_min_max(Vec3::new(100., 0., 0.), Vec3::new(200., 1., 100.));
	let natural_ids = GenerationContext::new(&natural).original_ids_for::<ChicoGrove>(outside);
	let fixture_ids = GenerationContext::new(&a).original_ids_for::<ChicoGrove>(outside);
	assert_eq!(natural_ids, fixture_ids);
	let first = GenerationContext::new(&a)
		.get_or_generate::<ChicoGrove>(
			ordinary.id().ok_or_else(|| anyhow::anyhow!("fixture id absent"))?,
		)
		.ok_or_else(|| anyhow::anyhow!("fixture absent"))?;
	let warm = GenerationContext::new(&a)
		.get::<ChicoGrove>(first.id())
		.ok_or_else(|| anyhow::anyhow!("warm fixture absent"))?;
	assert!(Arc::ptr_eq(&first, &warm));
	Ok(())
}

fn grown() -> ForestGroveTile {
	ForestGroveRecipe::uniform(
		ForestGroveKind::Orchard,
		GroveExtent::new(Vec3::ZERO, Vec3::new(100., 1., 100.)),
	)
	.grow(&FlatTerrainSample::default())
}
fn scene_app(tile: &ForestGroveTile, mode: FixtureMode) -> anyhow::Result<App> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		ScenePlugin,
		crate::stick_physics::StickPhysicsPlugin,
	));
	crate::stick_physics::register_vegetation_stick_colliders::<Source>(&mut app);
	let camera = Transform::from_translation(Vec3::new(50., 15., -30.));
	let bounds = tile.scene_bounds();
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &camera,
		current_transform: &camera,
		bounds: &bounds,
	};
	for (_, scene) in
		fixture_chunks(tile, &lod_ref, mode, CanopyShadowMode::Proxy).into_primitives()
	{
		app.world_mut()
			.spawn_scene(scene)
			.map_err(|error| anyhow::anyhow!("scene spawn: {error}"))?;
	}
	for _ in 0..32 {
		app.update();
	}
	Ok(app)
}

#[test]
fn construction_split_preserves_exact_semantic_and_collision_sources_without_ordinary_visual_roots()
-> anyhow::Result<()> {
	let tile = grown();
	let mut a = scene_app(&tile, FixtureMode::Ordinary)?;
	let mut b = scene_app(&tile, FixtureMode::Persistent)?;
	let sources = |app: &mut App| {
		let mut query = app.world_mut().query::<(&Source, &VegetationInstance)>();
		let mut sources = query
			.iter(app.world())
			.map(|(source, instance)| (source.clone(), *instance))
			.collect::<Vec<_>>();
		sources.sort_by(|(_, a), (_, b)| {
			a.anchor.x.total_cmp(&b.anchor.x).then(a.anchor.z.total_cmp(&b.anchor.z))
		});
		sources
	};
	let ordinary = sources(&mut a);
	let persistent = sources(&mut b);
	assert!(!ordinary.is_empty());
	assert_eq!(ordinary.len(), persistent.len());
	for ((a, anchor_a), (b, anchor_b)) in ordinary.iter().zip(&persistent) {
		assert_eq!(anchor_a, anchor_b);
		assert!(Arc::ptr_eq(&a.0.vegetation, &b.0.vegetation));
		assert_eq!(
			a.stick_nodes_for_level(LodSceneLevel::High).flatten(),
			b.stick_nodes_for_level(LodSceneLevel::High).flatten()
		);
		assert_eq!(
			a.foliage_nodes_for_level(LodSceneLevel::High).flatten(),
			b.foliage_nodes_for_level(LodSceneLevel::High).flatten()
		);
	}
	let collider_count = |app: &mut App| {
		app.world_mut()
			.query_filtered::<Entity, With<crate::stick_physics::StickPhysicsCompound>>()
			.iter(app.world())
			.count()
	};
	assert_eq!(collider_count(&mut a), ordinary.len());
	assert_eq!(collider_count(&mut b), ordinary.len());
	let ordinary_visual_roots = |app: &mut App| {
		app.world_mut()
			.query_filtered::<Entity, With<scene_ref::MultiSceneMergeRoot>>()
			.iter(app.world())
			.count()
	};
	assert!(ordinary_visual_roots(&mut a) > 0);
	assert_eq!(ordinary_visual_roots(&mut b), 0);
	let statuses = |app: &mut App| {
		app.world_mut()
			.query::<&FixtureStatus>()
			.iter(app.world())
			.cloned()
			.collect::<Vec<_>>()
	};
	let sa = statuses(&mut a);
	let sb = statuses(&mut b);
	assert_eq!(sa.len(), 1);
	assert_eq!(sb.len(), 1);
	assert_eq!(sa[0].source_digest, sb[0].source_digest);
	assert_eq!(sa[0].batches, sb[0].batches);
	assert_eq!(sa[0].instances, sb[0].instances);
	Ok(())
}

#[test]
fn only_tagged_host_pins_high_and_shadow_instances_match_structural_proxies() -> anyhow::Result<()>
{
	let tile = grown();
	let bounds = tile.scene_bounds();
	let far = Transform::from_translation(Vec3::splat(10000.));
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &far,
		current_transform: &far,
		bounds: &bounds,
	};
	let natural = ChicoGroveHost::new(tile.clone(), ForestLayer::UpperCanopy);
	assert_ne!(natural.scene_lod_level(&lod_ref), LodSceneLevel::High);
	for mode in [FixtureMode::Ordinary, FixtureMode::Persistent] {
		assert_eq!(
			natural.clone().with_fixture(Some(mode)).scene_lod_level(&lod_ref),
			LodSceneLevel::High
		);
	}
	let ForestGroveTile::Orchard(orchard) = tile else { anyhow::bail!("fixture not Orchard") };
	let band = payload::SelectedBand::from_orchard(&orchard, CanopyShadowMode::Proxy)
		.map_err(anyhow::Error::msg)?;
	let shadows = band
		.batches
		.iter()
		.find(|batch| batch.key.material_kind == payload::MaterialKind::ShadowProxy)
		.ok_or_else(|| anyhow::anyhow!("shadow batch absent"))?;
	assert_eq!(shadows.instances.len(), orchard.plants.len());
	for (plant, shadow) in orchard.placed_plants().zip(shadows.instances.iter()) {
		let structural = plant
			.structural_lod()
			.ok_or_else(|| anyhow::anyhow!("structural source absent"))?;
		let expected = Mat4::from_scale_rotation_translation(
			Vec3::splat(structural.tree_radius.max(0.25)),
			Quat::IDENTITY,
			structural.center,
		);
		assert_eq!(shadow.world_from_local, expected);
	}
	assert!(payload::SelectedBand::from_orchard(&orchard, CanopyShadowMode::Full).is_err());
	Ok(())
}

#[test]
fn loaded_band_has_geometry_bounds_resident_proxies_shared_materials_and_complete_teardown()
-> anyhow::Result<()> {
	use super::{
		kit::{KitState, PackedKitMeshes},
		material::*,
		payload::{MaterialKind, SelectedBand},
		spawn::{PendingBand, SharedMaterials},
	};
	use bevy::asset::RenderAssetUsages;
	use bevy::camera::visibility::NoAutoAabb;
	use bevy::mesh::PrimitiveTopology;
	let tile = grown();
	let ForestGroveTile::Orchard(orchard) = &tile else { anyhow::bail!("fixture not Orchard") };
	let band =
		SelectedBand::from_orchard(orchard, CanopyShadowMode::Proxy).map_err(anyhow::Error::msg)?;
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()))
		.init_asset::<Mesh>()
		.init_asset::<PackedLeafMaterial>()
		.init_asset::<PackedStickMaterial>()
		.init_asset::<PackedShadowMaterial>()
		.init_resource::<PackedKitMeshes>()
		.init_resource::<SharedMaterials>();
	app.world_mut().spawn((Camera3d::default(), Camera::default()));
	let positions = [[-0.7_f32, -0.2, -0.4], [0.9, 0.8, 0.6], [0.3, 1.2, -0.1]];
	for batch in band.batches.iter() {
		let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
		mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.to_vec());
		mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 3]);
		let bounds = PackedKitMeshes::bounds_for_mesh(&mesh)
			.ok_or_else(|| anyhow::anyhow!("local bounds absent"))?;
		let handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
		let mut kits = app.world_mut().resource_mut::<PackedKitMeshes>();
		kits.states.insert(batch.key.kit, KitState::Ready(handle));
		kits.local_bounds.insert(batch.key.kit, bounds);
	}
	let bounds = band
		.geometric_bounds(app.world().resource::<PackedKitMeshes>())
		.ok_or_else(|| anyhow::anyhow!("geometric bounds absent"))?;
	for batch in band.batches.iter() {
		for instance in batch.instances.iter() {
			for position in positions {
				let point = instance.world_from_local.transform_point3(Vec3::from_array(position));
				assert!(point.cmpge(Vec3::from(bounds.min) - Vec3::splat(1e-4)).all());
				assert!(point.cmple(Vec3::from(bounds.max) + Vec3::splat(1e-4)).all());
			}
		}
	}
	let root = app
		.world_mut()
		.spawn((
			PendingBand(band.clone()),
			FixtureStatus::pending(FixtureMode::Persistent, tile.scene_bounds(), &band),
		))
		.id();
	spawn::load_selected_band(app.world_mut());
	let status = app
		.world()
		.get::<FixtureStatus>(root)
		.ok_or_else(|| anyhow::anyhow!("status absent"))?;
	assert_eq!(status.state, FixtureState::Ready);
	assert_eq!(status.selected_proxies, band.batches.len());
	assert_eq!(
		status.resident_bytes,
		band.batches.iter().map(|batch| batch.instances.len() as u64 * 176).sum::<u64>()
	);
	assert!(app.world().get::<PendingBand>(root).is_none());
	let entities = app
		.world()
		.get::<Children>(root)
		.ok_or_else(|| anyhow::anyhow!("proxy children absent"))?
		.iter()
		.collect::<Vec<_>>();
	assert_eq!(entities.len(), band.batches.len());
	let mut shared = std::collections::HashMap::new();
	for (&entity, batch) in entities.iter().zip(band.batches.iter()) {
		assert!(app.world().get::<NoAutoAabb>(entity).is_some());
		let material = match batch.key.material_kind {
			MaterialKind::Leaf => app
				.world()
				.get::<MeshMaterial3d<PackedLeafMaterial>>(entity)
				.map(|material| material.0.id().untyped()),
			MaterialKind::Stick => app
				.world()
				.get::<MeshMaterial3d<PackedStickMaterial>>(entity)
				.map(|material| material.0.id().untyped()),
			MaterialKind::ShadowProxy => app
				.world()
				.get::<MeshMaterial3d<PackedShadowMaterial>>(entity)
				.map(|material| material.0.id().untyped()),
		}
		.ok_or_else(|| anyhow::anyhow!("typed material missing"))?;
		if let Some(previous) = shared.insert((batch.key.material_kind, batch.key.color), material)
		{
			assert_eq!(previous, material);
		}
	}
	spawn::load_selected_band(app.world_mut());
	assert_eq!(
		app.world()
			.get::<Children>(root)
			.ok_or_else(|| anyhow::anyhow!("children lost"))?
			.iter()
			.collect::<Vec<_>>(),
		entities
	);
	app.world_mut().despawn(root);
	spawn::load_selected_band(app.world_mut());
	assert!(entities.iter().all(|&entity| app.world().get_entity(entity).is_err()));
	assert_eq!(app.world().resource::<SharedMaterials>().len(), 0);
	assert!(app.world().resource::<PackedKitMeshes>().states.is_empty());
	assert!(app.world().resource::<PackedKitMeshes>().local_bounds.is_empty());
	Ok(())
}

#[test]
fn fixed_view_contract_waits_for_camera_and_excludes_preview_lights_by_layers() {
	use bevy::camera::visibility::RenderLayers;
	use bevy::core_pipeline::prepass::DeferredPrepass;
	let mut world = World::new();
	assert_eq!(spawn::FixtureViewContract::validate(&mut world), Ok(false));
	let camera = world.spawn((Camera3d::default(), Camera::default())).id();
	world.spawn(DirectionalLight { shadow_maps_enabled: true, ..default() });
	let preview = world
		.spawn((
			DirectionalLight { shadow_maps_enabled: true, ..default() },
			RenderLayers::layer(1).with(30),
		))
		.id();
	assert_eq!(spawn::FixtureViewContract::validate(&mut world), Ok(true));
	world.entity_mut(preview).insert(RenderLayers::default());
	assert!(spawn::FixtureViewContract::validate(&mut world).is_err());
	world.entity_mut(preview).insert(RenderLayers::layer(1).with(30));
	world.entity_mut(camera).insert(DeferredPrepass);
	assert!(spawn::FixtureViewContract::validate(&mut world).is_err());
}

#[test]
fn explicit_fixture_failure_exits_once_without_diagnostic_subscriber() {
	let mut app = App::new();
	app.add_systems(Update, spawn::fail_invalid_fixture);
	app.world_mut().spawn(FixtureStatus::failed(
		FixtureMode::Persistent,
		Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE),
		"test unsupported fixture",
	));
	app.update();
	assert_eq!(app.world().resource::<Messages<AppExit>>().len(), 1);
	app.update();
	assert_eq!(app.world().resource::<Messages<AppExit>>().len(), 1);
}
