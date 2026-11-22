use super::*;
use anyhow::Result;

fn small_grove() -> TemperateLowerMassives {
	TemperateLowerMassivesParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0)))
		.build()
}

fn plant_height(plant: &TemperateLowerMassivesPlant) -> f32 {
	match &plant.kind {
		TemperateLowerMassivesKind::Oak(t) => t.geometry.height(),
		TemperateLowerMassivesKind::Storybook(t) => t.geometry.height(),
		TemperateLowerMassivesKind::Rory(t) => t.geometry.height(),
	}
}

fn plant_seed(plant: &TemperateLowerMassivesPlant) -> i32 {
	match &plant.kind {
		TemperateLowerMassivesKind::Oak(t) => t.geometry.canopy_noise.seed,
		TemperateLowerMassivesKind::Storybook(t) => t.geometry.canopy_noise.seed,
		TemperateLowerMassivesKind::Rory(t) => t.geometry.canopy_noise.seed,
	}
}

#[test]
fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
	let grove = small_grove();
	crate::grove::woody_checks::assert_high_medium_nests_plants(
		&grove,
		grove.plants.len(),
		"temperate lower massives",
	)?;

	let camera = Transform::from_translation(Vec3::new(40.0, 2.0, 40.0));
	let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &camera,
		current_transform: &camera,
		bounds: &bounds,
	};

	assert!(grove.stick_nodes_for_level(LodSceneLevel::Low).len() <= 1);
	let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).len();
	assert_eq!(low_foliage, grove.canopy_sites().len());
	assert!(low_foliage >= grove.plants.len());
	assert!(grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len() <= low_foliage);
	match grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low) {
		lod::SceneChunk::Primitive { weight, .. } => {
			assert_eq!(weight, vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
		}
		lod::SceneChunk::SubChunks(parts) => assert!(!parts.is_empty()),
		_ => {
			anyhow::bail!("Low temperate lower massives should emit flattened canopy kits")
		}
	}
	Ok(())
}

#[test]
fn tree_variants_quantize_archetypes() -> Result<()> {
	let mut params = TemperateLowerMassivesParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0)));
	params.tree_variants = 4;
	let grove = params.build();
	crate::grove::woody_checks::assert_quantized_archetypes(
		&grove.plants,
		4,
		"temperate lower massives",
		plant_height,
		plant_seed,
	)?;
	Ok(())
}

#[test]
fn claiming_visuals_preserves_sources_and_retains_only_physics_for_storybooks() -> Result<()> {
	let mut grove = small_grove();
	let original = Arc::clone(&grove.plants);
	let storybooks = grove
		.plants
		.iter()
		.filter(|plant| matches!(plant.kind, TemperateLowerMassivesKind::Storybook(_)))
		.count();
	assert!(storybooks > 0);
	let sites = grove.canopy_sites().len();
	grove.claim_storybook_field_visuals();
	assert!(Arc::ptr_eq(&original, &grove.plants));
	assert_eq!(grove.canopy_sites().len(), sites - storybooks);
	crate::grove::woody_checks::assert_high_medium_nests_plants(
		&grove,
		original.len(),
		"field-owned physics metadata",
	)?;
	let plant = grove
		.plants
		.iter()
		.find(|plant| matches!(plant.kind, TemperateLowerMassivesKind::Storybook(_)))
		.ok_or_else(|| anyhow::anyhow!("missing Storybook"))?;
	let TemperateLowerMassivesKind::Storybook(tree) = &plant.kind else {
		anyhow::bail!("wrong species")
	};
	use bevy::scene::{ScenePlugin, WorldSceneExt};
	use bevy::transform::TransformPlugin;
	let mut app = App::new();
	app.add_plugins(MinimalPlugins).add_plugins((
		TransformPlugin,
		AssetPlugin::default(),
		ScenePlugin,
	));
	let (entity, child) = {
		let world = app.world_mut();
		let entity = world.spawn_scene(storybook_physics_metadata(tree, plant))?.id();
		assert!(
			world
				.get::<FlattenedComponentsOnly<PlacedVegetation<Arc<StorybookTree>>>>(entity)
				.is_some()
		);
		assert!(world.get::<Transform>(entity).is_some());
		assert!(world.get::<GlobalTransform>(entity).is_some());
		assert!(world.get::<Children>(entity).is_none());
		let child = world
			.spawn((Transform::from_translation(Vec3::new(4.0, 1.5, -2.0)), ChildOf(entity)))
			.id();
		(entity, child)
	};
	let _ = entity;
	app.update();
	let child_translation = app
		.world()
		.get::<GlobalTransform>(child)
		.ok_or_else(|| anyhow::anyhow!("collider child missing GlobalTransform"))?
		.translation();
	assert!(
		(child_translation - Vec3::new(4.0, 1.5, -2.0)).length() < 1e-4,
		"physics child drifted to {child_translation:?}"
	);
	for (before, after) in original.iter().zip(grove.plants.iter()) {
		assert_eq!(before.placement, after.placement);
		if let (
			TemperateLowerMassivesKind::Storybook(a),
			TemperateLowerMassivesKind::Storybook(b),
		) = (&before.kind, &after.kind)
		{
			assert!(Arc::ptr_eq(a, b));
		}
	}
	Ok(())
}
