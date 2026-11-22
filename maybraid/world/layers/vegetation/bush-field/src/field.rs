//! Persistent field: compact plant / branch / leaf descriptors, not expanded meshes.

use bevy::prelude::*;
use vegetation_shaders::wesl::{LodThresholds, WindInputs};

use crate::bounds::BushBounds;
use crate::layout::{
	BranchDescriptor, BushPatchHeader, LeafDescriptor, PlantDescriptor, PARENT_NONE,
};
use crate::patch::BushPatch;
use crate::topology::{MAX_BRANCHES_PER_PLANT, MAX_LEAVES_PER_PLANT};

/// Tunables for GPU list capacities and wind. Authoring budgets live here, not in WESL.
#[derive(Resource, Clone, Debug)]
pub struct BushFieldSettings {
	pub max_plants: u32,
	pub max_branches: u32,
	pub max_leaves: u32,
	pub max_per_lod: u32,
	pub wind: WindInputs,
	pub lod: LodThresholds,
	pub diagnostics: bool,
	pub freeze_time_s: Option<f32>,
}

impl BushFieldSettings {
	pub fn from_env() -> Self {
		let mut settings = Self::default();
		settings.diagnostics = std::env::var_os("MAYBRAID_BUSH_DIAGNOSTICS").is_some();
		settings
	}
}

impl Default for BushFieldSettings {
	fn default() -> Self {
		Self {
			max_plants: 262_144,
			max_branches: 262_144,
			max_leaves: 262_144,
			max_per_lod: 262_144,
			wind: WindInputs::gentle_botw(0.0),
			lod: LodThresholds::bush_default(),
			diagnostics: false,
			freeze_time_s: None,
		}
	}
}

/// CPU mirror of the persistent GPU plant / branch / leaf buffers.
#[derive(Resource, Clone, Debug, Default)]
pub struct BushField {
	pub plants: Vec<PlantDescriptor>,
	pub branches: Vec<BranchDescriptor>,
	pub leaves: Vec<LeafDescriptor>,
	pub patches: Vec<BushPatchHeader>,
	pub bounds: BushBounds,
	pub generation: u64,
	pub grove_residency: bool,
}

impl BushField {
	pub fn rebuild<'a>(
		patches: impl IntoIterator<Item = &'a BushPatch>,
		settings: &BushFieldSettings,
	) -> Self {
		let patches: Vec<_> = patches.into_iter().collect();
		let grove_residency = patches.iter().any(|patch| patch.grove_residency);
		let mut plants = Vec::new();
		let mut branches = Vec::new();
		let mut leaves = Vec::new();
		let mut headers = Vec::new();
		let mut bounds = BushBounds::empty();
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
				bounds = bounds
					.union(BushBounds::from_plant(&plant.descriptor, settings.wind.strength_m));
				plants.push(plant.descriptor);
				kept += 1;
			}
			headers.push(patch.header(plant_offset, kept, settings));
		}
		Self { plants, branches, leaves, patches: headers, bounds, generation: 0, grove_residency }
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
