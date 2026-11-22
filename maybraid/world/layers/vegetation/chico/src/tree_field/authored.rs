//! One GPU grove patch from existing plant roots. One seed, one recipe.
//!
//! This trial does not import authored wood or canopy topology. Physics still
//! reads the original Storybook chain through the metadata-only host.

use bevy::prelude::*;
use vegetation_groves::temperate_lower_massives::StorybookFieldSource;
use vegetation_tree_field::{TreePatch, TreeRecipe};

impl StorybookGrovePatch {
	pub(super) fn from_sources(seed: u32, sources: &[StorybookFieldSource]) -> TreePatch {
		TreePatch::from_roots(
			seed,
			sources.iter().map(|source| source.placement.translation),
			TreeRecipe::storybook(),
		)
	}
}

/// Adapter so groves can build a [`TreePatch`] without depending on tree-field types.
pub(super) struct StorybookGrovePatch;

#[cfg(test)]
mod tests {
	use super::*;
	use sbs_trees::StorybookTree;
	use std::sync::Arc;
	use vegetation_components::{Placement, leaf_material_ref};

	#[test]
	fn patch_keeps_authored_roots_and_one_recipe() -> anyhow::Result<()> {
		let tree = Arc::new(StorybookTree::unit_from_num(7));
		let sources = [
			StorybookFieldSource {
				tree: Arc::clone(&tree),
				canopy_material: leaf_material_ref(),
				placement: Placement::new(Vec3::new(12.0, 3.0, -4.0), 0.0)
					.with_scale(Vec3::splat(15.0)),
			},
			StorybookFieldSource {
				tree: Arc::clone(&tree),
				canopy_material: leaf_material_ref(),
				placement: Placement::new(Vec3::new(-8.0, 1.0, 6.0), 0.0)
					.with_scale(Vec3::splat(12.0)),
			},
		];
		let patch = StorybookGrovePatch::from_sources(11, &sources);
		assert_eq!(patch.seed, 11);
		assert_eq!(patch.recipe, TreeRecipe::storybook());
		assert_eq!(patch.samples.len(), 2);
		assert_eq!(patch.samples[0].root, sources[0].placement.translation);
		assert_eq!(patch.samples[1].root, sources[1].placement.translation);
		let plants = patch.plants();
		assert_eq!(plants.len(), 2);
		assert!((plants[0].descriptor.root - sources[0].placement.translation).length() < 1e-4);
		assert!(plants.iter().all(|plant| plant.attachment_is_bounded()));
		let field =
			vegetation_tree_field::TreeField::from_plants(plants, &super::super::owner_settings())
				.map_err(anyhow::Error::msg)?;
		assert_eq!(field.plants.len(), 2);
		Ok(())
	}

	#[test]
	fn all_storybooks_in_nine_authored_tiles_fit_without_drops() -> anyhow::Result<()> {
		use vegetation_groves::{GroveExtent, TemperateLowerMassivesParams};
		let mut total = 0;
		for x in -1..=1 {
			for z in -1..=1 {
				let origin = Vec3::new(x as f32 * 100.0, 0.0, z as f32 * 100.0);
				let grove = TemperateLowerMassivesParams::default()
					.with_extent(GroveExtent::new(origin, origin + Vec3::new(100.0, 1.0, 100.0)))
					.build();
				let sources = grove.authored_storybooks();
				let patch = StorybookGrovePatch::from_sources(
					TreePatch::seed_from_origin(origin),
					&sources,
				);
				let plants = patch.plants();
				let field = vegetation_tree_field::TreeField::from_plants(
					plants,
					&super::super::owner_settings(),
				)
				.map_err(anyhow::Error::msg)?;
				assert_eq!(field.plants.len(), sources.len());
				for (plant, source) in field.plants.iter().zip(&sources) {
					assert_eq!(plant.root, source.placement.translation);
				}
				assert!(patch.plants().iter().all(|plant| plant.attachment_is_bounded()));
				total += field.plants.len();
			}
		}
		assert!(total > 0);
		Ok(())
	}

	#[test]
	fn nine_tile_swap_reports_legacy_and_gpu_work() -> anyhow::Result<()> {
		use lod::scene::LodSceneLevel;
		use std::time::Instant;
		use vegetation_components::VegetationComponents;
		use vegetation_groves::{GroveExtent, TemperateLowerMassivesParams};
		use vegetation_tree_field::{
			BRANCH_DESCRIPTOR_STRIDE, LEAF_DESCRIPTOR_STRIDE, PLANT_DESCRIPTOR_STRIDE, TreeField,
			TreeFieldSettings, TreeSelector,
		};
		let mut storybooks = 0usize;
		let mut woody = 0usize;
		let mut kit_sticks = 0usize;
		let mut kit_foliage = 0usize;
		let mut gpu_plants = 0usize;
		let mut gpu_branches = 0usize;
		let mut gpu_leaves = 0usize;
		let mut upload = 0u64;
		let mut select_ns = 0u128;
		let mut compact_wood = 0u32;
		let mut compact_foliage = 0u32;
		for x in -1..=1 {
			for z in -1..=1 {
				let origin = Vec3::new(x as f32 * 100.0, 0.0, z as f32 * 100.0);
				let grove = TemperateLowerMassivesParams::default()
					.with_extent(GroveExtent::new(origin, origin + Vec3::new(100.0, 1.0, 100.0)))
					.build();
				woody += grove.plants.len();
				let sources = grove.authored_storybooks();
				storybooks += sources.len();
				for source in &sources {
					kit_sticks +=
						source.tree.stick_nodes_for_level(LodSceneLevel::High).flatten().len();
					kit_foliage +=
						source.tree.foliage_nodes_for_level(LodSceneLevel::High).flatten().len();
				}
				let patch = StorybookGrovePatch::from_sources(
					TreePatch::seed_from_origin(origin),
					&sources,
				);
				let field = TreeField::from_plants(patch.plants(), &TreeFieldSettings::default())
					.map_err(anyhow::Error::msg)?;
				assert_eq!(field.plants.len(), sources.len());
				gpu_plants += field.plants.len();
				gpu_branches += field.branches.len();
				gpu_leaves += field.leaves.len();
				upload += field.upload_bytes();
				let started = Instant::now();
				let lists = TreeSelector::from_view(
					look_view(
						origin + Vec3::new(50.0, 12.0, 80.0),
						origin + Vec3::new(50.0, 8.0, 50.0),
					),
					&TreeFieldSettings::default(),
				)
				.compact(&field.plants, &field.branches, &field.leaves);
				select_ns += started.elapsed().as_nanos();
				compact_wood += lists.submitted_wood;
				compact_foliage += lists.submitted_foliage;
			}
		}
		assert!(storybooks > 0);
		eprintln!(
			"legacy-vs-gpu nine-tile: woody={woody} storybooks={storybooks} kit_stick_nodes={kit_sticks} kit_foliage_nodes={kit_foliage} gpu_plants={gpu_plants} gpu_branches={gpu_branches} gpu_leaves={gpu_leaves} upload_B={upload} plant_stride={PLANT_DESCRIPTOR_STRIDE} branch_stride={BRANCH_DESCRIPTOR_STRIDE} leaf_stride={LEAF_DESCRIPTOR_STRIDE} cpu_select_ns={select_ns} submitted_wood={compact_wood} submitted_foliage={compact_foliage}"
		);
		assert_eq!(gpu_plants, storybooks);
		assert!(upload > 0);
		Ok(())
	}

	fn look_view(origin: Vec3, target: Vec3) -> vegetation_tree_field::GrassViewUniform {
		use bevy::camera::CameraProjection;
		use bevy::prelude::PerspectiveProjection;
		use vegetation_shaders::wesl::LodThresholds;
		use vegetation_tree_field::GrassViewUniform;
		let view_from_world = Transform::from_translation(origin)
			.looking_at(target, Vec3::Y)
			.to_matrix()
			.inverse();
		let projection = PerspectiveProjection {
			fov: 1.0,
			aspect_ratio: 16.0 / 9.0,
			near: 0.1,
			far: 400.0,
			..Default::default()
		};
		let clip = projection.get_clip_from_view() * view_from_world;
		let t = LodThresholds::tree_default();
		GrassViewUniform {
			clip_from_world: clip,
			world_from_clip: clip.inverse(),
			world_position: origin.extend(0.0),
			prev_world_position: origin.extend(0.0),
			resolution: Vec4::new(1280.0, 1080.0, 1.0, 0.0),
			wind: Vec4::new(0.92, 0.38, 0.0, 0.22),
			lod_distances: Vec4::new(0.85, t.near_m, t.mid_m, t.card_m),
			lod_density: Vec4::new(t.near_density, t.mid_density, t.card_density, t.min_height_px),
			counts: Vec4::new(0.0, 8192.0, 0.0, 0.0),
			light_direction: Vec4::new(0.4, 0.8, 0.3, 0.0),
		}
	}
}
