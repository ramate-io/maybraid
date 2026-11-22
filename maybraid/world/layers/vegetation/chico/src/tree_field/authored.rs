//! Import the existing Storybook chain; never call the field recipe generator.
//!
//! Every authored wood edge remains present. High canopy-ball sites become three
//! crossed cards, so legacy kits and fields have explicitly different surfaces.
//! Edges are independent rigid world curves (stiffness 1), avoiding a second
//! skeleton and the playground's bounded hierarchical attachment walk.

use bevy::prelude::*;
use lod::LodSceneLevel;
use vegetation_components::{FoliageGeometry, VegetationComponents};
use vegetation_groves::temperate_lower_massives::StorybookFieldSource;
use vegetation_tree_field::{
	BranchDescriptor, LeafDescriptor, PlantDescriptor, TreePlant, PARENT_NONE,
};

pub(super) fn from_source(source: &StorybookFieldSource) -> Result<TreePlant, &'static str> {
	let placement = source.placement;
	if !placement.translation.is_finite()
		|| !placement.scale.is_finite()
		|| placement.scale.min_element() <= 0.0
	{
		return Err("nonfinite or nonpositive authored placement");
	}
	let point = |p: Vec3| placement.translation + placement.rotation() * (p * placement.scale);
	let seed = source.tree.geometry.canopy_noise.seed as u32;
	let id = vegetation_shaders::wesl::hash_u32(
		seed ^ placement.translation.x.to_bits()
			^ placement.translation.z.to_bits().rotate_left(13),
	);
	let branches: Vec<_> = source
		.tree
		.chain
		.segments()
		.enumerate()
		.map(|(index, edge)| {
			let p0 = point(edge.start.position);
			let p2 = point(edge.end.position);
			BranchDescriptor {
				p0,
				p1: (p0 + p2) * 0.5,
				p2,
				parent_index: PARENT_NONE,
				parent_t: 0.0,
				plant_index: 0,
				root_radius_m: edge.start.radius * placement.scale.max_element(),
				tip_radius_m: edge.end.radius * placement.scale.max_element(),
				stiffness: 1.0,
				keep_rank: 0.0,
				twist: 0.0,
				depth: 0,
				branch_id: id.wrapping_add(index as u32),
				major_parent: PARENT_NONE,
			}
		})
		.collect();
	if branches.is_empty() {
		return Err("authored tree has no wood edges");
	}
	let color = source
		.canopy_material
		.palette
		.first()
		.copied()
		.unwrap_or(Color::srgb(0.22, 0.62, 0.28))
		.to_linear();
	let palette = Vec3::new(color.red, color.green, color.blue);
	let mut leaves = Vec::new();
	for node in source.tree.foliage_nodes_for_level(LodSceneLevel::High).flatten() {
		let sites = match node.geometry {
			FoliageGeometry::CheapBallCollection(collection) => collection.placements,
			FoliageGeometry::CheapBall => vec![vegetation_components::Placement::IDENTITY],
			_ => return Err("authored canopy uses unsupported foliage geometry"),
		};
		for site in sites {
			let site = placement.compose_child(node.placement.compose_child(site));
			let nearest = branches
				.iter()
				.enumerate()
				.min_by(|(_, a), (_, b)| {
					a.p2.distance_squared(site.translation)
						.total_cmp(&b.p2.distance_squared(site.translation))
				})
				.ok_or("authored canopy has no supporting branch")?;
			// High Storybook balls are placed on exact authored branch endpoints.
			// Reject a future construction that does not satisfy this contract.
			if nearest.1.p2.distance(site.translation) > 0.01 {
				return Err("authored canopy site is not a branch endpoint");
			}
			for card in 0..3 {
				let leaf_id = id.wrapping_add(leaves.len() as u32);
				let yaw = card as f32 * std::f32::consts::TAU / 3.0;
				leaves.push(LeafDescriptor {
					plant_index: 0,
					parent_branch: nearest.0 as u32,
					attach_t: 1.0,
					size_m: site.scale.max_element() * 2.0,
					twist: yaw,
					fold: 0.35,
					flutter: 0.0,
					density_rank: if card == 0 {
						0.0
					} else {
						vegetation_shaders::wesl::hash_u32_to_unit(leaf_id)
					},
					facing: Vec3::new(yaw.cos(), 0.3, yaw.sin()).normalize(),
					leaf_id,
					palette,
					shape: 0.5,
				});
			}
		}
	}
	if leaves.is_empty() {
		return Err("authored Storybook canopy has no supported sites");
	}
	let padding = leaves.iter().map(|leaf| leaf.size_m).fold(0.0_f32, f32::max);
	let mut min = placement.translation;
	let mut max = placement.translation;
	for branch in &branches {
		let radius = Vec3::splat(branch.root_radius_m.max(branch.tip_radius_m) + padding);
		min = min.min(branch.p0 - radius).min(branch.p2 - radius);
		max = max.max(branch.p0 + radius).max(branch.p2 + radius);
	}
	let height = source.tree.geometry.height() * placement.scale.y;
	Ok(TreePlant {
		descriptor: PlantDescriptor {
			root: placement.translation,
			height_m: height,
			up: placement.rotation() * Vec3::Y,
			plant_id: id,
			aabb_min: min,
			aabb_max: max,
			branch_offset: 0,
			branch_count: branches.len() as u32,
			leaf_offset: 0,
			leaf_count: leaves.len() as u32,
			seed,
			flags: PlantDescriptor::FLAG_ACTIVE,
			crown_radius_m: (max - min).xz().max_element() * 0.5,
			wood_stiffness: 1.0,
			foliage_density: 1.0,
			species: 2.0,
		},
		branches,
		leaves,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use sbs_trees::StorybookTree;
	use std::sync::Arc;
	use vegetation_components::{leaf_material_ref, Placement};

	#[test]
	fn imports_all_authored_edges_and_keeps_shared_source() -> anyhow::Result<()> {
		let tree = Arc::new(StorybookTree::unit_from_num(7));
		let source = StorybookFieldSource {
			tree: Arc::clone(&tree),
			canopy_material: leaf_material_ref(),
			placement: Placement::new(Vec3::new(12.0, 3.0, -4.0), 0.0)
				.with_scale(Vec3::splat(15.0)),
		};
		let imported = from_source(&source).map_err(anyhow::Error::msg)?;
		assert_eq!(imported.branches.len(), tree.chain.segments().count());
		assert_eq!(imported.descriptor.root, source.placement.translation);
		assert!(Arc::ptr_eq(&source.tree, &tree));
		assert!(!imported.leaves.is_empty());
		assert!(imported
			.leaves
			.iter()
			.all(|leaf| (leaf.parent_branch as usize) < imported.branches.len()));
		for (edge, branch) in tree.chain.segments().zip(&imported.branches) {
			assert_eq!(branch.p0, source.placement.translation + edge.start.position * 15.0);
			assert_eq!(branch.p2, source.placement.translation + edge.end.position * 15.0);
		}
		let field = vegetation_tree_field::TreeField::from_plants(
			vec![imported],
			&super::super::owner_settings(),
		)
		.map_err(anyhow::Error::msg)?;
		assert_eq!(field.plants.len(), 1);
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
				let plants = sources
					.iter()
					.map(from_source)
					.collect::<Result<Vec<_>, _>>()
					.map_err(anyhow::Error::msg)?;
				let field = vegetation_tree_field::TreeField::from_plants(
					plants,
					&super::super::owner_settings(),
				)
				.map_err(anyhow::Error::msg)?;
				assert_eq!(field.plants.len(), sources.len());
				for (plant, source) in field.plants.iter().zip(&sources) {
					assert_eq!(plant.root, source.placement.translation);
					assert_eq!(plant.branch_count as usize, source.tree.chain.segments().count());
				}
				total += field.plants.len();
			}
		}
		assert!(total > 0);
		Ok(())
	}
}
