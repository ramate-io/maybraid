//! Bounded connected skeleton + layered canopy descriptors. CPU-authored; GPU expands surfaces.

use bevy::math::Vec3;
use vegetation_shaders::wesl::{hash_u32, hash_u32_to_unit, plant_id};

use crate::layout::{BranchDescriptor, LeafDescriptor, PlantDescriptor, PARENT_NONE};
use crate::recipe::TreeRecipe;

/// Explicit capacity so shaders never walk an unbounded graph.
pub const MAX_BRANCHES_PER_PLANT: u32 = 40;
pub const MAX_LEAVES_PER_PLANT: u32 = 256;
pub const MAX_BRANCH_DEPTH: u32 = 3;

/// Compact topology for one tree. Branch parent indices are local to this plant.
#[derive(Clone, Debug, PartialEq)]
pub struct TreePlant {
	pub descriptor: PlantDescriptor,
	pub branches: Vec<BranchDescriptor>,
	pub leaves: Vec<LeafDescriptor>,
}

impl TreePlant {
	pub fn generate(
		seed: u32,
		local_index: u32,
		root: Vec3,
		up: Vec3,
		scale: f32,
		recipe: &TreeRecipe,
	) -> Self {
		let id = plant_id(seed, local_index);
		let height = sample_range(&recipe.height_m, hash_u32_to_unit(id ^ 0x1)) * scale.max(1e-4);
		let limb_count = sample_u32(&recipe.limb_count, hash_u32(id ^ 0x2));
		let depth_budget = sample_u32(&recipe.branch_depth, hash_u32(id ^ 0x3)).min(MAX_BRANCH_DEPTH);
		let spread = sample_range(&recipe.canopy_spread_m, hash_u32_to_unit(id ^ 0x4)) * scale;
		let stalk_radius = sample_range(&recipe.stalk_radius_m, hash_u32_to_unit(id ^ 0x5)) * scale;
		let leaf_size = sample_range(&recipe.leaf_size_m, hash_u32_to_unit(id ^ 0x6)) * scale;
		let upn = up.normalize_or(Vec3::Y);

		let mut branches = Vec::new();
		let mut leaves = Vec::new();

		let trunk_len = height * recipe.stalk_height_fraction.clamp(0.62, 0.88);
		let lean = tangent_on(upn, hash_u32_to_unit(id ^ 0x7) * std::f32::consts::TAU)
			* (height * 0.035);
		let trunk_tip = root + upn * trunk_len + lean;
		let trunk_mid = root + upn * (trunk_len * 0.48) + lean * 0.42;
		branches.push(BranchDescriptor {
			p0: root,
			parent_index: PARENT_NONE,
			p1: trunk_mid,
			parent_t: 0.0,
			p2: trunk_tip,
			plant_index: 0,
			root_radius_m: stalk_radius,
			tip_radius_m: stalk_radius * 0.55,
			stiffness: recipe.wood_stiffness,
			keep_rank: 0.0,
			twist: 0.0,
			depth: 0,
			branch_id: id,
			major_parent: PARENT_NONE,
		});

		for limb_i in 0..limb_count {
			if branches.len() as u32 >= MAX_BRANCHES_PER_PLANT {
				break;
			}
			let limb_id = hash_u32(id ^ (0x100 + limb_i));
			let ring_u = 0.34 + 0.62 * ((limb_i as f32 + 0.35) / limb_count.max(1) as f32);
			let attach_t = ring_u.clamp(0.32, 0.97);
			let yaw = (limb_i as f32 + hash_u32_to_unit(limb_id)) * std::f32::consts::TAU
				/ limb_count.max(1) as f32;
			let dome = (std::f32::consts::PI * ring_u).sin();
			let limb_len = spread * (0.55 + 0.42 * dome)
				+ height * (0.06 + 0.08 * hash_u32_to_unit(limb_id ^ 0x11));
			let up_bias = (ring_u - 0.42) * 0.55;
			let dir = shoot_direction(upn, yaw, 0.82 + 0.14 * dome, (0.28 + up_bias).clamp(0.12, 0.78));
			let limb_index = branches.len() as u32;
			let local = child_bezier(limb_len, dir, upn, hash_u32_to_unit(limb_id ^ 0x21));
			let limb_root_r = stalk_radius * 0.34;
			branches.push(BranchDescriptor {
				p0: local[0],
				parent_index: 0,
				p1: local[1],
				parent_t: attach_t,
				p2: local[2],
				plant_index: 0,
				root_radius_m: limb_root_r,
				tip_radius_m: limb_root_r * 0.42,
				stiffness: (recipe.wood_stiffness - 0.08).max(0.58),
				keep_rank: 0.08 + 0.06 * hash_u32_to_unit(limb_id ^ 0x31),
				twist: (hash_u32_to_unit(limb_id ^ 0x41) - 0.5) * 0.45,
				depth: 1,
				branch_id: limb_id,
				major_parent: 0,
			});

			if recipe.foliage_density > 0.0 {
				emit_clump(
					&mut leaves,
					limb_index,
					limb_id,
					0.78,
					leaf_size,
					upn,
					dir,
					14,
					recipe,
					true,
				);
				emit_clump(
					&mut leaves,
					limb_index,
					limb_id ^ 0x9a,
					0.52,
					leaf_size * 0.82,
					upn,
					dir,
					8,
					recipe,
					false,
				);
			}

			if depth_budget < 2 {
				continue;
			}
			let forks = 1 + (hash_u32(limb_id ^ 0x51) % 2);
			for fork_i in 0..forks {
				if branches.len() as u32 >= MAX_BRANCHES_PER_PLANT {
					break;
				}
				let fork_id = hash_u32(limb_id ^ (0x61 + fork_i));
				let fork_len = limb_len * (0.42 + 0.22 * hash_u32_to_unit(fork_id));
				let fork_yaw = yaw + (0.55 + 0.7 * hash_u32_to_unit(fork_id ^ 1))
					* if fork_i == 0 { 1.0 } else { -1.0 };
				let fork_dir = shoot_direction(
					upn,
					fork_yaw,
					0.88,
					(0.18 + 0.42 * ring_u).clamp(0.12, 0.7),
				);
				let fork_local = child_bezier(fork_len, fork_dir, upn, hash_u32_to_unit(fork_id ^ 2));
				let fork_index = branches.len() as u32;
				branches.push(BranchDescriptor {
					p0: fork_local[0],
					parent_index: limb_index,
					p1: fork_local[1],
					parent_t: 0.58 + 0.28 * hash_u32_to_unit(fork_id ^ 3),
					p2: fork_local[2],
					plant_index: 0,
					root_radius_m: limb_root_r * 0.48,
					tip_radius_m: limb_root_r * 0.22,
					stiffness: (recipe.wood_stiffness - 0.18).max(0.48),
					keep_rank: 0.32 + 0.18 * hash_u32_to_unit(fork_id ^ 4),
					twist: (hash_u32_to_unit(fork_id ^ 5) - 0.5) * 0.7,
					depth: 2,
					branch_id: fork_id,
					major_parent: limb_index,
				});
				if recipe.foliage_density > 0.0 {
					emit_clump(
						&mut leaves,
						fork_index,
						fork_id,
						0.82,
						leaf_size * 0.88,
						upn,
						fork_dir,
						10,
						recipe,
						true,
					);
				}

				if depth_budget < 3 || hash_u32_to_unit(fork_id ^ 0x71) < 0.38 {
					continue;
				}
				if branches.len() as u32 >= MAX_BRANCHES_PER_PLANT {
					break;
				}
				let twig_id = hash_u32(fork_id ^ 0x81);
				let twig_len = fork_len * 0.58;
				let twig_dir = shoot_direction(
					upn,
					fork_yaw + 0.8 * (hash_u32_to_unit(twig_id) - 0.5),
					0.95,
					0.22,
				);
				let twig_local = child_bezier(twig_len, twig_dir, upn, hash_u32_to_unit(twig_id ^ 1));
				let twig_index = branches.len() as u32;
				branches.push(BranchDescriptor {
					p0: twig_local[0],
					parent_index: fork_index,
					p1: twig_local[1],
					parent_t: 0.62,
					p2: twig_local[2],
					plant_index: 0,
					root_radius_m: limb_root_r * 0.22,
					tip_radius_m: limb_root_r * 0.10,
					stiffness: (recipe.wood_stiffness - 0.28).max(0.40),
					keep_rank: 0.58 + 0.22 * hash_u32_to_unit(twig_id ^ 2),
					twist: (hash_u32_to_unit(twig_id ^ 3) - 0.5) * 0.9,
					depth: 3,
					branch_id: twig_id,
					major_parent: limb_index,
				});
				if recipe.foliage_density > 0.0 {
					emit_clump(
						&mut leaves,
						twig_index,
						twig_id,
						0.72,
						leaf_size * 0.72,
						upn,
						twig_dir,
						7,
						recipe,
						true,
					);
				}
			}
		}

		let crown = spread.max(height * 0.32);
		let pad = crown.max(height * 0.12);
		let descriptor = PlantDescriptor {
			root,
			height_m: height,
			up: upn,
			plant_id: id,
			aabb_min: root - Vec3::new(pad, 0.08, pad),
			branch_offset: 0,
			aabb_max: root + Vec3::new(pad, height + pad * 0.22, pad),
			branch_count: branches.len() as u32,
			leaf_offset: 0,
			leaf_count: leaves.len() as u32,
			seed,
			flags: PlantDescriptor::FLAG_ACTIVE,
			crown_radius_m: crown,
			wood_stiffness: recipe.wood_stiffness,
			foliage_density: recipe.foliage_density,
			species: 2.0,
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

fn emit_clump(
	leaves: &mut Vec<LeafDescriptor>,
	parent_branch: u32,
	clump_id: u32,
	attach_t: f32,
	size_m: f32,
	up: Vec3,
	outward: Vec3,
	count: u32,
	recipe: &TreeRecipe,
	sunlit: bool,
) {
	for leaf_i in 0..count {
		if leaves.len() as u32 >= MAX_LEAVES_PER_PLANT {
			return;
		}
		let leaf_id = hash_u32(clump_id ^ (0x200 + leaf_i));
		let rank = hash_u32_to_unit(leaf_id ^ 0x85ebca6b);
		if rank >= recipe.foliage_density {
			continue;
		}
		let yaw = (leaf_i as f32 + 0.18) * std::f32::consts::TAU / count.max(1) as f32
			+ hash_u32_to_unit(leaf_id ^ 0xa1) * 0.7;
		let pitch = 0.22 + 0.95 * hash_u32_to_unit(leaf_id ^ 0xb1);
		let side = tangent_on(up, yaw);
		let facing = if sunlit {
			(outward * 0.55 + up * (0.35 + 0.45 * pitch) + side * 0.55).normalize_or(up)
		} else {
			(-outward * 0.22 + up * 0.12 - Vec3::Y * 0.55 + side * 0.35).normalize_or(-Vec3::Y)
		};
		let hue = hash_u32_to_unit(clump_id ^ 0xc1);
		let palette = recipe.foliage.tip_rgb.lerp(
			Vec3::new(0.58, 0.62, 0.14).lerp(Vec3::new(0.22, 0.48, 0.12), hue),
			0.28,
		);
		leaves.push(LeafDescriptor {
			plant_index: 0,
			parent_branch,
			attach_t: (attach_t + 0.08 * (hash_u32_to_unit(leaf_id ^ 0xd1) - 0.5)).clamp(0.12, 0.98),
			size_m: (size_m * (0.78 + 0.38 * hash_u32_to_unit(leaf_id ^ 0xe1))).max(0.28),
			twist: (hash_u32_to_unit(leaf_id ^ 0xf1) - 0.5) * 1.6,
			fold: 0.62 + 0.32 * hash_u32_to_unit(leaf_id ^ 0x12),
			flutter: 0.035 + 0.055 * hash_u32_to_unit(leaf_id ^ 0x13),
			density_rank: rank,
			facing,
			leaf_id,
			palette,
			shape: hash_u32_to_unit(leaf_id ^ 0x14),
		});
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
	let along = dir.normalize_or(up) * length.max(0.12);
	let side = dir.cross(up).normalize_or(Vec3::X) * (length * (0.10 + 0.16 * bow));
	let p0 = Vec3::ZERO;
	let p2 = along;
	let p1 = along * 0.52 + side + up * (length * 0.06 * bow);
	[p0, p1, p2]
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::recipe::TreeRecipe;

	#[test]
	fn generated_plant_stays_within_capacity() -> anyhow::Result<()> {
		let plant = TreePlant::generate(
			11,
			0,
			Vec3::ZERO,
			Vec3::Y,
			1.0,
			&TreeRecipe::storybook(),
		);
		assert!(plant.branches.len() as u32 <= MAX_BRANCHES_PER_PLANT);
		assert!(plant.leaves.len() as u32 <= MAX_LEAVES_PER_PLANT);
		assert!(plant.branches.iter().all(|b| b.depth <= MAX_BRANCH_DEPTH));
		assert!(plant.is_connected());
		assert!(plant.branches.iter().any(|b| b.is_trunk()));
		assert!(plant.branches.iter().any(|b| b.depth == 1));
		assert!(!plant.leaves.is_empty());
		Ok(())
	}

	#[test]
	fn leafless_recipe_emits_wood_only() -> anyhow::Result<()> {
		let plant = TreePlant::generate(
			3,
			1,
			Vec3::new(2.0, 0.0, -1.0),
			Vec3::Y,
			1.0,
			&TreeRecipe::leafless_storybook(),
		);
		assert!(!plant.branches.is_empty());
		assert!(plant.leaves.is_empty());
		Ok(())
	}

	#[test]
	fn identities_are_stable() -> anyhow::Result<()> {
		let a = TreePlant::generate(9, 4, Vec3::ZERO, Vec3::Y, 1.0, &TreeRecipe::storybook());
		let b = TreePlant::generate(9, 4, Vec3::ZERO, Vec3::Y, 1.0, &TreeRecipe::storybook());
		assert_eq!(a.descriptor.plant_id, b.descriptor.plant_id);
		assert_eq!(
			a.branches.iter().map(|br| br.branch_id).collect::<Vec<_>>(),
			b.branches.iter().map(|br| br.branch_id).collect::<Vec<_>>()
		);
		Ok(())
	}

	#[test]
	fn twigs_major_parent_is_a_limb() -> anyhow::Result<()> {
		let plant = TreePlant::generate(5, 0, Vec3::ZERO, Vec3::Y, 1.0, &TreeRecipe::storybook());
		for branch in &plant.branches {
			if branch.depth == 3 {
				let major = plant.branches[branch.major_parent as usize];
				assert_eq!(major.depth, 1);
			}
		}
		Ok(())
	}
}
