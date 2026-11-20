//! Bounded connected skeleton + foliage descriptors. CPU-authored; GPU expands surfaces.

use bevy::math::Vec3;
use vegetation_shaders::wesl::{hash_u32, hash_u32_to_unit, plant_id};

use crate::layout::{BranchDescriptor, LeafDescriptor, PlantDescriptor, PARENT_NONE};
use crate::recipe::BushRecipe;

/// Explicit capacity so shaders never walk an unbounded graph.
pub const MAX_BRANCHES_PER_PLANT: u32 = 16;
pub const MAX_LEAVES_PER_PLANT: u32 = 48;
pub const MAX_BRANCH_DEPTH: u32 = 2;

/// Compact topology for one plant. Branch parent indices are local to this plant.
#[derive(Clone, Debug, PartialEq)]
pub struct BushPlant {
	pub descriptor: PlantDescriptor,
	pub branches: Vec<BranchDescriptor>,
	pub leaves: Vec<LeafDescriptor>,
}

impl BushPlant {
	pub fn generate(
		seed: u32,
		local_index: u32,
		root: Vec3,
		up: Vec3,
		scale: f32,
		recipe: &BushRecipe,
	) -> Self {
		let id = plant_id(seed, local_index);
		let height = sample_range(&recipe.height_m, hash_u32_to_unit(id ^ 0x1)) * scale.max(1e-4);
		let shoot_count = sample_u32(&recipe.shoot_count, hash_u32(id ^ 0x2));
		let depth_budget = sample_u32(&recipe.branch_depth, hash_u32(id ^ 0x3)).min(MAX_BRANCH_DEPTH);
		let radial = sample_range(&recipe.radial_strength, hash_u32_to_unit(id ^ 0x4));
		let vertical = sample_range(&recipe.vertical_bias, hash_u32_to_unit(id ^ 0x5));
		let leaf_radius = sample_range(&recipe.leaf_radius_m, hash_u32_to_unit(id ^ 0x6)) * scale;
		let upn = up.normalize_or(Vec3::Y);

		let mut branches = Vec::new();
		let mut leaves = Vec::new();

		// Short woody stem; shoots build a rounded crown above it.
		let trunk_len = height * 0.38;
		let trunk_tip = root + upn * trunk_len;
		let trunk_mid = root + upn * (trunk_len * 0.52) + tangent_on(upn, 0.0) * (height * 0.03);
		let trunk_radius = (height * 0.032).clamp(0.016, 0.038);
		branches.push(BranchDescriptor {
			p0: root,
			parent_index: PARENT_NONE,
			p1: trunk_mid,
			parent_t: 0.0,
			p2: trunk_tip,
			plant_index: 0,
			root_radius_m: trunk_radius,
			tip_radius_m: trunk_radius * 0.62,
			stiffness: recipe.wood_stiffness,
			keep_rank: 0.0,
			twist: 0.0,
			depth: 0,
			branch_id: id,
			major_parent: PARENT_NONE,
		});

		for shoot_i in 0..shoot_count {
			if branches.len() as u32 >= MAX_BRANCHES_PER_PLANT {
				break;
			}
			let shoot_id = hash_u32(id ^ (0x100 + shoot_i));
			let attach_t = 0.42 + 0.52 * (shoot_i as f32 + 0.5) / shoot_count.max(1) as f32;
			let yaw = (shoot_i as f32 + hash_u32_to_unit(shoot_id)) * std::f32::consts::TAU
				/ shoot_count.max(1) as f32;
			let shoot_len = height * (0.52 + 0.22 * hash_u32_to_unit(shoot_id ^ 0x11));
			// Authored projection is upright; mix extra radial splay so the silhouette reads as a bush.
			let dir = shoot_direction(upn, yaw, (radial + 0.38).min(0.92), (vertical * 0.72).max(0.32));
			let parent_idx = 0u32;
			let shoot_index = branches.len() as u32;
			let local = child_bezier(shoot_len, dir, upn, hash_u32_to_unit(shoot_id ^ 0x21));
			branches.push(BranchDescriptor {
				p0: local[0],
				parent_index: parent_idx,
				p1: local[1],
				parent_t: attach_t.clamp(0.0, 1.0),
				p2: local[2],
				plant_index: 0,
				root_radius_m: trunk_radius * 0.58,
				tip_radius_m: trunk_radius * 0.28,
				stiffness: (recipe.wood_stiffness - 0.12).max(0.45),
				keep_rank: 0.12 + 0.08 * hash_u32_to_unit(shoot_id ^ 0x31),
				twist: (hash_u32_to_unit(shoot_id ^ 0x41) - 0.5) * 0.6,
				depth: 1,
				branch_id: shoot_id,
				major_parent: parent_idx,
			});

			if depth_budget >= 2 && hash_u32_to_unit(shoot_id ^ 0x51) > 0.22 {
				if branches.len() as u32 >= MAX_BRANCHES_PER_PLANT {
					break;
				}
				let twig_id = hash_u32(shoot_id ^ 0x61);
				let twig_len = shoot_len * 0.62;
				let twig_yaw = yaw + 0.95 * (hash_u32_to_unit(twig_id) - 0.5);
				let twig_dir = shoot_direction(upn, twig_yaw, (radial + 0.48).min(1.0), vertical * 0.55);
				let twig_local = child_bezier(twig_len, twig_dir, upn, hash_u32_to_unit(twig_id ^ 1));
				branches.push(BranchDescriptor {
					p0: twig_local[0],
					parent_index: shoot_index,
					p1: twig_local[1],
					parent_t: 0.58,
					p2: twig_local[2],
					plant_index: 0,
					root_radius_m: trunk_radius * 0.28,
					tip_radius_m: trunk_radius * 0.14,
					stiffness: (recipe.wood_stiffness - 0.22).max(0.38),
					keep_rank: 0.55 + 0.2 * hash_u32_to_unit(twig_id ^ 0x71),
					twist: (hash_u32_to_unit(twig_id ^ 0x81) - 0.5) * 0.8,
					depth: 2,
					branch_id: twig_id,
					major_parent: shoot_index,
				});
			}
		}

		if recipe.foliage_density > 0.0 {
			let card = (leaf_radius * 2.4 + height * 0.14).clamp(0.07, 0.28);
			for (branch_i, branch) in branches.iter().enumerate() {
				if leaves.len() as u32 >= MAX_LEAVES_PER_PLANT {
					break;
				}
				let count = match branch.depth {
					0 => 2,
					1 => 7,
					_ => 5,
				};
				for leaf_i in 0..count {
					if leaves.len() as u32 >= MAX_LEAVES_PER_PLANT {
						break;
					}
					let leaf_id = hash_u32(branch.branch_id ^ (0x200 + leaf_i));
					let rank = hash_u32_to_unit(leaf_id ^ 0x85ebca6b);
					if rank >= recipe.foliage_density {
						continue;
					}
					let attach_t = if branch.depth == 0 {
						0.72 + 0.22 * ((leaf_i as f32 + 0.5) / count as f32)
					} else {
						0.28 + 0.68 * ((leaf_i as f32 + 0.5) / count as f32)
					};
					let size = card * (0.82 + 0.36 * hash_u32_to_unit(leaf_id ^ 0x91));
					let yaw = hash_u32_to_unit(leaf_id ^ 0xa1) * std::f32::consts::TAU;
					let (s, c) = yaw.sin_cos();
					let outward = tangent_on(upn, yaw) * 0.85 + upn * 0.45;
					leaves.push(LeafDescriptor {
						plant_index: 0,
						parent_branch: branch_i as u32,
						attach_t,
						size_m: size.max(0.06),
						twist: (hash_u32_to_unit(leaf_id ^ 0xb1) - 0.5) * 1.4,
						fold: 0.45 + 0.4 * hash_u32_to_unit(leaf_id ^ 0xc1),
						flutter: 0.06 + 0.10 * hash_u32_to_unit(leaf_id ^ 0xd1),
						density_rank: rank,
						facing: outward.normalize_or(Vec3::new(c, 0.35, s)),
						leaf_id,
						palette: recipe.foliage.tip_rgb,
						shape: hash_u32_to_unit(leaf_id ^ 0xe1),
					});
				}
			}
		}

		let crown = height * (0.55 + 0.18 * radial);
		let pad = crown.max(height * 0.2);
		let descriptor = PlantDescriptor {
			root,
			height_m: height,
			up: upn,
			plant_id: id,
			aabb_min: root - Vec3::new(pad, 0.04, pad),
			branch_offset: 0,
			aabb_max: root + Vec3::new(pad, height + pad * 0.25, pad),
			branch_count: branches.len() as u32,
			leaf_offset: 0,
			leaf_count: leaves.len() as u32,
			seed,
			flags: PlantDescriptor::FLAG_ACTIVE,
			crown_radius_m: crown,
			wood_stiffness: recipe.wood_stiffness,
			foliage_density: recipe.foliage_density,
			species: 1.0,
		};

		Self { descriptor, branches, leaves }
	}

	pub fn is_connected(&self) -> bool {
		self.branches.iter().any(|branch| branch.is_trunk())
			&& self.branches.iter().all(|branch| {
				branch.is_trunk() || (branch.parent_index as usize) < self.branches.len()
			})
	}
}

fn sample_range(range: &std::ops::RangeInclusive<f32>, unit: f32) -> f32 {
	let lo = *range.start();
	let hi = *range.end();
	lo + (hi - lo) * unit.clamp(0.0, 1.0)
}

fn sample_u32(range: &std::ops::RangeInclusive<u32>, hash: u32) -> u32 {
	let lo = *range.start();
	let hi = *range.end();
	if hi <= lo {
		return lo;
	}
	lo + hash % (hi - lo + 1)
}

fn tangent_on(up: Vec3, yaw: f32) -> Vec3 {
	let (s, c) = yaw.sin_cos();
	let a = up.any_orthonormal_vector();
	let b = up.cross(a);
	(a * c + b * s).normalize_or(Vec3::X)
}

fn shoot_direction(up: Vec3, yaw: f32, radial: f32, vertical: f32) -> Vec3 {
	let side = tangent_on(up, yaw);
	(up * vertical + side * radial).normalize_or(up)
}

fn child_bezier(length: f32, dir: Vec3, up: Vec3, bow: f32) -> [Vec3; 3] {
	let along = dir.normalize_or(up) * length.max(0.05);
	let side = dir.cross(up).normalize_or(Vec3::X) * (length * (0.08 + 0.12 * bow));
	// `p0` is the attachment. Junction overlap uses a matching root radius, not a
	// second mesh; the wood shader evaluates a small negative `t` collar.
	let p0 = Vec3::ZERO;
	let p2 = along;
	let p1 = along * 0.5 + side;
	[p0, p1, p2]
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::recipe::BushRecipe;

	#[test]
	fn generated_plant_stays_within_capacity() -> anyhow::Result<()> {
		let plant = BushPlant::generate(
			11,
			0,
			Vec3::ZERO,
			Vec3::Y,
			1.0,
			&BushRecipe::small_scrub(),
		);
		assert!(plant.branches.len() as u32 <= MAX_BRANCHES_PER_PLANT);
		assert!(plant.leaves.len() as u32 <= MAX_LEAVES_PER_PLANT);
		assert!(plant.branches.iter().all(|b| b.depth <= MAX_BRANCH_DEPTH));
		assert!(plant.is_connected());
		assert!(plant.branches.iter().any(|b| b.is_trunk()));
		Ok(())
	}

	#[test]
	fn leafless_recipe_emits_wood_only() -> anyhow::Result<()> {
		let plant = BushPlant::generate(
			3,
			1,
			Vec3::new(2.0, 0.0, -1.0),
			Vec3::Y,
			1.0,
			&BushRecipe::leafless_scrub(),
		);
		assert!(!plant.branches.is_empty());
		assert!(plant.leaves.is_empty());
		Ok(())
	}

	#[test]
	fn identities_are_stable() -> anyhow::Result<()> {
		let a = BushPlant::generate(9, 4, Vec3::ZERO, Vec3::Y, 1.0, &BushRecipe::small_scrub());
		let b = BushPlant::generate(9, 4, Vec3::ZERO, Vec3::Y, 1.0, &BushRecipe::small_scrub());
		assert_eq!(a.descriptor.plant_id, b.descriptor.plant_id);
		assert_eq!(
			a.branches.iter().map(|br| br.branch_id).collect::<Vec<_>>(),
			b.branches.iter().map(|br| br.branch_id).collect::<Vec<_>>()
		);
		Ok(())
	}
}
