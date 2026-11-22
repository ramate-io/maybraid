//! Persistent field: compact plant / branch / leaf descriptors, not expanded meshes.

use bevy::prelude::*;
use vegetation_shaders::wesl::{LodThresholds, WindInputs};

use crate::bounds::TreeBounds;
use crate::layout::{
	BranchDescriptor, LeafCloseStyle, LeafDescriptor, PlantDescriptor, TreePatchHeader, PARENT_NONE,
};
use crate::patch::TreePatch;
use crate::topology::{MAX_BRANCHES_PER_PLANT, MAX_LEAVES_PER_PLANT};

/// Tunables for GPU list capacities and wind. Authoring budgets live here, not in WESL.
#[derive(Resource, Clone, Debug)]
pub struct TreeFieldSettings {
	pub max_plants: u32,
	pub max_branches: u32,
	pub max_leaves: u32,
	pub max_per_lod: u32,
	pub wind: WindInputs,
	pub lod: LodThresholds,
	pub diagnostics: bool,
	pub freeze_time_s: Option<f32>,
	/// When set, every plant uses this close-up leaf style instead of the recipe.
	pub leaf_close_style: Option<LeafCloseStyle>,
}

impl TreeFieldSettings {
	pub fn from_env() -> Self {
		let mut settings = Self::default();
		settings.diagnostics = std::env::var_os("MAYBRAID_TREE_DIAGNOSTICS").is_some();
		settings
	}
}

impl Default for TreeFieldSettings {
	fn default() -> Self {
		Self {
			max_plants: 512,
			max_branches: 16_384,
			max_leaves: 65_536,
			max_per_lod: 8_192,
			wind: WindInputs::gentle_botw(0.0),
			lod: LodThresholds::tree_default(),
			diagnostics: false,
			freeze_time_s: None,
			leaf_close_style: None,
		}
	}
}

/// CPU mirror of the persistent GPU plant / branch / leaf buffers.
#[derive(Resource, Clone, Debug, Default)]
pub struct TreeField {
	pub plants: Vec<PlantDescriptor>,
	pub branches: Vec<BranchDescriptor>,
	pub leaves: Vec<LeafDescriptor>,
	pub patches: Vec<TreePatchHeader>,
	pub bounds: TreeBounds,
	pub generation: u64,
}

impl TreeField {
	/// Same-build parity receipt over the exact bytes uploaded for this owner.
	pub fn descriptor_digest(&self) -> u64 {
		use std::hash::{Hash, Hasher};
		let mut digest = std::collections::hash_map::DefaultHasher::new();
		for plant in &self.plants {
			plant.to_std430_bytes().hash(&mut digest);
		}
		for branch in &self.branches {
			branch.to_std430_bytes().hash(&mut digest);
		}
		for leaf in &self.leaves {
			leaf.to_std430_bytes().hash(&mut digest);
		}
		digest.finish()
	}

	/// Pack already-authored plants without generating a second skeleton or dropping content.
	/// Callers retain their legacy representation when admission fails.
	pub fn from_plants(
		plants: Vec<crate::topology::TreePlant>,
		settings: &TreeFieldSettings,
	) -> Result<Self, &'static str> {
		let branch_count: usize = plants.iter().map(|p| p.branches.len()).sum();
		let leaf_count: usize = plants.iter().map(|p| p.leaves.len()).sum();
		if plants.len() > settings.max_plants as usize
			|| branch_count > settings.max_branches as usize
			|| leaf_count > settings.max_leaves as usize
			|| branch_count.max(leaf_count) > settings.max_per_lod as usize
		{
			return Err("grove exceeds descriptor or selection capacity");
		}
		let mut field = Self::default();
		for mut plant in plants {
			for (index, branch) in plant.branches.iter().enumerate() {
				if (branch.parent_index != PARENT_NONE && branch.parent_index as usize >= index)
					|| (branch.major_parent != PARENT_NONE && branch.major_parent as usize > index)
				{
					return Err("authored branch references invalid parent");
				}
			}
			if plant
				.leaves
				.iter()
				.any(|leaf| leaf.parent_branch as usize >= plant.branches.len())
			{
				return Err("authored leaf references invalid branch");
			}
			let branch_offset = field.branches.len() as u32;
			let leaf_offset = field.leaves.len() as u32;
			let plant_index = field.plants.len() as u32;
			for mut branch in plant.branches {
				if branch.parent_index != PARENT_NONE {
					branch.parent_index += branch_offset;
				}
				if branch.major_parent != PARENT_NONE {
					branch.major_parent += branch_offset;
				}
				branch.plant_index = plant_index;
				field.branches.push(branch);
			}
			for mut leaf in plant.leaves {
				leaf.plant_index = plant_index;
				leaf.parent_branch += branch_offset;
				field.leaves.push(leaf);
			}
			plant.descriptor.branch_offset = branch_offset;
			plant.descriptor.branch_count = field.branches.len() as u32 - branch_offset;
			plant.descriptor.leaf_offset = leaf_offset;
			plant.descriptor.leaf_count = field.leaves.len() as u32 - leaf_offset;
			if let Some(style) = settings.leaf_close_style {
				plant.descriptor = plant.descriptor.with_leaf_close_style(style);
			}
			field.bounds = field
				.bounds
				.union(TreeBounds::from_plant(&plant.descriptor, settings.wind.strength_m));
			// Imported canopies may lean beyond the recipe's symmetric envelope.
			field.bounds = field.bounds.union(TreeBounds {
				min: plant.descriptor.aabb_min,
				max: plant.descriptor.aabb_max,
			});
			field.plants.push(plant.descriptor);
		}
		Ok(field.with_generation(1))
	}

	pub fn rebuild<'a>(
		patches: impl IntoIterator<Item = &'a TreePatch>,
		settings: &TreeFieldSettings,
	) -> Self {
		let mut plants = Vec::new();
		let mut branches = Vec::new();
		let mut leaves = Vec::new();
		let mut headers = Vec::new();
		let mut bounds = TreeBounds::empty();
		for patch in patches {
			if plants.len() as u32 >= settings.max_plants {
				break;
			}
			let generated = patch.plants();
			let plant_offset = plants.len() as u32;
			let mut kept = 0u32;
			for mut plant in generated {
				if plants.len() as u32 >= settings.max_plants {
					break;
				}
				if branches.len() as u32 + plant.branches.len() as u32 > settings.max_branches {
					break;
				}
				if leaves.len() as u32 + plant.leaves.len() as u32 > settings.max_leaves {
					plant.leaves.clear();
					plant.descriptor.leaf_count = 0;
				}
				if plant.branches.len() as u32 > MAX_BRANCHES_PER_PLANT
					|| plant.leaves.len() as u32 > MAX_LEAVES_PER_PLANT
				{
					continue;
				}
				let branch_offset = branches.len() as u32;
				let leaf_offset = leaves.len() as u32;
				let plant_index = plants.len() as u32;
				for mut branch in plant.branches {
					if branch.parent_index != PARENT_NONE {
						branch.parent_index = branch.parent_index.saturating_add(branch_offset);
					}
					if branch.major_parent != PARENT_NONE {
						branch.major_parent = branch.major_parent.saturating_add(branch_offset);
					}
					branch.plant_index = plant_index;
					branches.push(branch);
				}
				for mut leaf in plant.leaves {
					leaf.plant_index = plant_index;
					leaf.parent_branch = leaf.parent_branch.saturating_add(branch_offset);
					leaves.push(leaf);
				}
				plant.descriptor.branch_offset = branch_offset;
				plant.descriptor.branch_count = branches.len() as u32 - branch_offset;
				plant.descriptor.leaf_offset = leaf_offset;
				plant.descriptor.leaf_count = leaves.len() as u32 - leaf_offset;
				if let Some(style) = settings.leaf_close_style {
					plant.descriptor = plant.descriptor.with_leaf_close_style(style);
				}
				bounds = bounds
					.union(TreeBounds::from_plant(&plant.descriptor, settings.wind.strength_m));
				plants.push(plant.descriptor);
				kept += 1;
			}
			headers.push(patch.header(plant_offset, kept, settings));
		}
		Self { plants, branches, leaves, patches: headers, bounds, generation: 0 }
	}

	pub fn with_generation(mut self, generation: u64) -> Self {
		self.generation = generation;
		self
	}

	pub fn is_empty(&self) -> bool {
		self.plants.is_empty()
	}

	pub fn branch_count(&self) -> u32 {
		self.branches.len() as u32
	}

	pub fn leaf_count(&self) -> u32 {
		self.leaves.len() as u32
	}
}

#[cfg(test)]
mod authored_tests {
	use super::*;
	use crate::{TreePlant, TreeRecipe};

	#[test]
	fn authored_admission_is_complete_or_rejected() -> anyhow::Result<()> {
		let source = TreePlant::generate(1, 0, Vec3::ZERO, Vec3::Y, 1., &TreeRecipe::storybook());
		let settings = TreeFieldSettings::default();
		let field = TreeField::from_plants(vec![source.clone(), source.clone()], &settings)
			.map_err(anyhow::Error::msg)?;
		assert_eq!(field.plants.len(), 2);
		assert_eq!(field.branches.len(), source.branches.len() * 2);
		assert_eq!(field.leaves.len(), source.leaves.len() * 2);
		assert_eq!(field.branches[source.branches.len()].plant_index, 1);
		assert!(field.leaves[source.leaves.len()].parent_branch >= source.branches.len() as u32);
		let limited = TreeFieldSettings { max_leaves: 1, ..settings };
		assert!(TreeField::from_plants(vec![source], &limited).is_err());
		Ok(())
	}
}
