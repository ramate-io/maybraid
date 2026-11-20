//! CPU patch ownership: plants as compact descriptors, never per-branch ECS.

use bevy::prelude::*;

use crate::bounds::TreeBounds;
use crate::field::TreeFieldSettings;
use crate::host::TreePatchLod;
use crate::layout::{TreePatchHeader, PARENT_NONE};
use crate::recipe::TreeRecipe;
use crate::topology::TreePlant;

/// World-space surface sample for a plant root.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeSample {
	pub root: Vec3,
	pub up: Vec3,
	pub scale: f32,
}

impl TreeSample {
	pub fn planar(root: Vec3) -> Self {
		Self { root, up: Vec3::Y, scale: 1.0 }
	}

	pub fn on_slope(root: Vec3, up: Vec3, scale: f32) -> Self {
		Self { root, up: up.normalize_or(Vec3::Y), scale: scale.max(1e-4) }
	}
}

/// Bounded spatial patch. Upload compact plants when this changes.
#[derive(Component, Clone, Debug)]
pub struct TreePatch {
	pub seed: u32,
	pub recipe: TreeRecipe,
	pub coverage: f32,
	pub samples: Vec<TreeSample>,
	pub lod: TreePatchLod,
}

impl TreePatch {
	pub fn new(seed: u32, recipe: TreeRecipe, coverage: f32, samples: Vec<TreeSample>) -> Self {
		Self { seed, recipe, coverage: coverage.clamp(0.0, 1.0), samples, lod: TreePatchLod::High }
	}

	pub fn isolated(seed: u32, center: Vec3, recipe: TreeRecipe) -> Self {
		Self::new(seed, recipe, 1.0, vec![TreeSample::planar(center)])
	}

	pub fn grove(seed: u32, origin: Vec3, extent_xz: Vec2, spacing_m: f32) -> Self {
		Self::new(
			seed,
			TreeRecipe::storybook(),
			1.0,
			grid_samples(origin, extent_xz, spacing_m, seed, Vec3::Y),
		)
	}

	pub fn sloped_grove(
		seed: u32,
		origin: Vec3,
		extent_xz: Vec2,
		spacing_m: f32,
		normal: Vec3,
	) -> Self {
		Self::new(
			seed,
			TreeRecipe::storybook(),
			1.0,
			grid_samples(origin, extent_xz, spacing_m, seed, normal),
		)
	}

	pub fn sparse_wood(seed: u32, origin: Vec3, extent_xz: Vec2, spacing_m: f32) -> Self {
		Self::new(
			seed,
			TreeRecipe::sparse_storybook(),
			1.0,
			grid_samples(origin, extent_xz, spacing_m, seed, Vec3::Y),
		)
	}

	pub fn leafless_wood(seed: u32, center: Vec3) -> Self {
		Self::isolated(seed, center, TreeRecipe::leafless_storybook())
	}

	/// Grove-style placements: world root + scale, same recipe as Temperate Lower Massives storybook.
	pub fn from_grove_placements(seed: u32, placements: &[(Vec3, f32)], recipe: TreeRecipe) -> Self {
		let samples = placements
			.iter()
			.map(|(root, scale)| TreeSample::planar(*root).with_scale(*scale))
			.collect();
		Self::new(seed, recipe, 1.0, samples)
	}

	pub fn with_lod(mut self, lod: TreePatchLod) -> Self {
		self.lod = lod;
		self
	}

	pub fn with_coverage(mut self, coverage: f32) -> Self {
		self.coverage = coverage.clamp(0.0, 1.0);
		self
	}

	pub fn with_recipe(mut self, recipe: TreeRecipe) -> Self {
		self.recipe = recipe;
		self
	}

	pub fn is_empty(&self) -> bool {
		self.samples.is_empty() || self.coverage <= 0.0
	}

	pub fn plants(&self) -> Vec<TreePlant> {
		if self.is_empty() {
			return Vec::new();
		}
		let density = (self.coverage * self.lod.density_scale()).clamp(0.0, 1.0);
		let mut recipe = self.recipe.clone();
		recipe.foliage_density *= density;
		self.samples
			.iter()
			.enumerate()
			.filter_map(|(i, sample)| {
				let rank = vegetation_shaders::wesl::density_rank(vegetation_shaders::wesl::plant_id(
					self.seed,
					i as u32,
				));
				if rank >= density && density < 1.0 && self.samples.len() > 1 {
					return None;
				}
				Some(TreePlant::generate(
					self.seed,
					i as u32,
					sample.root,
					sample.up,
					sample.scale,
					&recipe,
				))
			})
			.collect()
	}

	pub fn bounds(&self, settings: &TreeFieldSettings) -> TreeBounds {
		let mut bounds = TreeBounds::empty();
		for plant in self.plants() {
			bounds = bounds.union(TreeBounds::from_plant(&plant.descriptor, settings.wind.strength_m));
		}
		if bounds.is_empty() {
			TreeBounds { min: Vec3::ZERO, max: Vec3::ZERO }
		} else {
			bounds
		}
	}

	pub fn header(
		&self,
		plant_offset: u32,
		plant_count: u32,
		settings: &TreeFieldSettings,
	) -> TreePatchHeader {
		let bounds = self.bounds(settings);
		TreePatchHeader {
			aabb_min: bounds.min,
			plant_offset,
			aabb_max: bounds.max,
			plant_count,
			seed: self.seed,
			coverage: self.coverage * self.lod.density_scale(),
			species: 1,
			flags: if plant_count == 0 { 0 } else { TreePatchHeader::FLAG_ACTIVE },
		}
	}

	pub fn has_connected_wood(&self) -> bool {
		self.plants().iter().all(|plant| {
			plant.is_connected()
				&& plant.branches.iter().any(|branch| branch.parent_index == PARENT_NONE)
		})
	}
}

impl TreeSample {
	pub fn with_scale(mut self, scale: f32) -> Self {
		self.scale = scale.max(1e-4);
		self
	}
}

fn grid_samples(
	origin: Vec3,
	extent_xz: Vec2,
	spacing_m: f32,
	seed: u32,
	normal: Vec3,
) -> Vec<TreeSample> {
	let spacing = spacing_m.max(2.0);
	let nx = (extent_xz.x / spacing).ceil().max(1.0) as u32;
	let nz = (extent_xz.y / spacing).ceil().max(1.0) as u32;
	let mut samples = Vec::with_capacity((nx * nz) as usize);
	for z in 0..nz {
		for x in 0..nx {
			let i = z * nx + x;
			let jitter = vegetation_shaders::wesl::hash_u32(seed ^ i);
			let jx = ((jitter & 0xff) as f32 / 255.0 - 0.5) * spacing * 0.45;
			let jz = (((jitter >> 8) & 0xff) as f32 / 255.0 - 0.5) * spacing * 0.45;
			let root = origin
				+ Vec3::new((x as f32 + 0.5) * spacing + jx, 0.0, (z as f32 + 0.5) * spacing + jz);
			samples.push(TreeSample::on_slope(root, normal, 1.0));
		}
	}
	samples
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn zero_coverage_emits_no_plants() -> anyhow::Result<()> {
		let patch = TreePatch::grove(1, Vec3::ZERO, Vec2::splat(4.0), 1.2).with_coverage(0.0);
		assert!(patch.plants().is_empty());
		Ok(())
	}

	#[test]
	fn grove_placements_keep_roots() -> anyhow::Result<()> {
		let placements = [(Vec3::new(-2.0, 0.4, 3.0), 1.1), (Vec3::new(4.0, 0.2, -1.0), 0.9)];
		let patch = TreePatch::from_grove_placements(4, &placements, TreeRecipe::storybook());
		let plants = patch.plants();
		assert_eq!(plants.len(), 2);
		assert!((plants[0].descriptor.root - placements[0].0).length() < 1e-4);
		assert!((plants[1].descriptor.root - placements[1].0).length() < 1e-4);
		Ok(())
	}
}
