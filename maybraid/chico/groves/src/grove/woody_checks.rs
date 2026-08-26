//! Shared woody grove render checks. Per-grove tests supply the built grove and
//! plant accessors; High/Medium nesting and archetype quantization live here.

use std::collections::HashSet;

use anyhow::Result;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Entity, Transform, Vec3};
use chico_vegetation_components::VegetationComponents;
use lod::gen::{LodScene, LodSceneLevel};
use lod::lod_ref::LodRef;

/// Camera used by the copied High/Medium nest checks.
pub fn preview_lod_ref<'a>(camera: &'a Transform, bounds: &'a Aabb3d) -> LodRef<'a> {
	LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: camera,
		current_transform: camera,
		bounds,
	}
}

fn preview_camera_and_bounds() -> (Transform, Aabb3d) {
	(
		Transform::from_translation(Vec3::new(40.0, 2.0, 40.0)),
		Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE),
	)
}

/// High/Medium must nest one lazy flattened host per plant, not per kit node.
pub fn assert_high_medium_nests_plants<G>(grove: &G, plant_count: usize, label: &str) -> Result<()>
where
	G: VegetationComponents + LodScene,
{
	if plant_count == 0 {
		anyhow::bail!("expected placed {label} plants");
	}

	anyhow::ensure!(grove.stick_nodes_for_level(LodSceneLevel::High).len() == 0);
	anyhow::ensure!(grove.foliage_nodes_for_level(LodSceneLevel::High).len() == 0);
	anyhow::ensure!(grove.stick_nodes_for_level(LodSceneLevel::Medium).len() == 0);
	anyhow::ensure!(grove.foliage_nodes_for_level(LodSceneLevel::Medium).len() == 0);

	let (camera, bounds) = preview_camera_and_bounds();
	let lod_ref = preview_lod_ref(&camera, &bounds);
	let high = grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::High);
	let lod::SceneChunk::SubChunks(parts) = high else {
		anyhow::bail!("High {label} should wrap plant chunks");
	};
	anyhow::ensure!(parts.len() == 1, "expected one lazy plant producer");
	let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0] else {
		anyhow::bail!("High {label} plants should be SceneChunk::Lazy");
	};
	anyhow::ensure!(*remaining_primitives == plant_count);
	anyhow::ensure!(*remaining_weight as usize == plant_count);
	Ok(())
}

/// Same cell positions + `tree_variants = n` share archetypal unit meshes.
pub fn assert_quantized_archetypes<P>(
	plants: &[P],
	max_variants: usize,
	label: &str,
	unit_height: impl Fn(&P) -> f32,
	seed: impl Fn(&P) -> i32,
) -> Result<()> {
	if plants.is_empty() {
		anyhow::bail!("expected placed {label} plants");
	}
	for plant in plants {
		let height = unit_height(plant);
		anyhow::ensure!((height - 1.0).abs() < 1e-4, "expected unit height, got {height}");
	}
	let seeds: HashSet<i32> = plants.iter().map(seed).collect();
	anyhow::ensure!(
		seeds.len() <= max_variants,
		"expected ≤{max_variants} unique unit seeds, got {}",
		seeds.len()
	);
	Ok(())
}

/// Repeated variants must share one unit `Arc` (same type + num).
pub fn assert_shared_unit_arcs<P>(plants: &[P], arc_ptr: impl Fn(&P) -> *const ()) -> Result<()> {
	let ptrs: HashSet<_> = plants.iter().map(arc_ptr).collect();
	anyhow::ensure!(plants.len() > ptrs.len(), "expected repeated variants to share one unit Arc");
	Ok(())
}
