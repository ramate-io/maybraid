//! Bounded connected skeleton + layered canopy descriptors. CPU-authored; GPU expands surfaces.

use bevy::math::{Vec3, Vec3Swizzles};
use vegetation_shaders::wesl::{hash_u32, hash_u32_to_unit, plant_id};

use crate::layout::{BranchDescriptor, LeafDescriptor, PlantDescriptor, PARENT_NONE};
use crate::recipe::TreeRecipe;

/// Explicit capacity so shaders never walk an unbounded graph.
pub const MAX_BRANCHES_PER_PLANT: u32 = 40;
pub const MAX_LEAVES_PER_PLANT: u32 = 512;
pub const MAX_BRANCH_DEPTH: u32 = 3;

/// Overlapping canopy shells. Inner hugs wood; outer fans into billowy clumps.
/// Tree-scale counts; bush `LeafLayer` stays in the bush crate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanopyLayer {
	/// `0` inner, `1` mid, `2` outer.
	pub shell: u32,
	pub height_lo: f32,
	pub height_hi: f32,
	pub clumps: u32,
	pub cards_per_clump: u32,
	pub size_scale: f32,
	pub radial_splay: f32,
	/// `1` follows the limb; `0` fans toward the sky dome.
	pub lift: f32,
}

impl CanopyLayer {
	pub const SHELL_COUNT: u32 = 3;

	pub fn inner() -> Self {
		Self {
			shell: 0,
			height_lo: 0.28,
			height_hi: 0.68,
			clumps: 2,
			cards_per_clump: 2,
			size_scale: 0.85,
			radial_splay: 0.38,
			lift: 0.58,
		}
	}

	pub fn mid() -> Self {
		Self {
			shell: 1,
			height_lo: 0.42,
			height_hi: 0.90,
			clumps: 3,
			cards_per_clump: 3,
			size_scale: 1.28,
			radial_splay: 0.78,
			lift: 0.30,
		}
	}

	pub fn outer() -> Self {
		Self {
			shell: 2,
			height_lo: 0.55,
			height_hi: 0.98,
			clumps: 4,
			cards_per_clump: 4,
			size_scale: 1.48,
			radial_splay: 1.12,
			lift: 0.12,
		}
	}

	/// Outer first so a capacity clamp keeps the crown silhouette.
	pub fn shells() -> [Self; 3] {
		[Self::outer(), Self::mid(), Self::inner()]
	}

	pub fn rim_shells() -> [Self; 2] {
		[Self::outer(), Self::mid()]
	}

	pub fn shell_unit(self) -> f32 {
		self.shell as f32 / (Self::SHELL_COUNT - 1) as f32
	}

	/// Outer shells keep first under LOD (lower `density_rank` is submitted).
	pub fn density_rank(self, hash: f32) -> f32 {
		hash * (1.0 - 0.38 * self.shell_unit())
	}

	pub fn attach_t(self, clump_i: u32, jitter: f32) -> f32 {
		let span = (self.height_hi - self.height_lo).max(0.04);
		let along = (clump_i as f32 + 0.5) / self.clumps.max(1) as f32;
		(self.height_lo + span * along + (jitter - 0.5) * 0.08).clamp(0.08, 0.98)
	}

	pub fn size_m(self, card: f32, jitter: f32) -> f32 {
		(card * self.size_scale * (0.82 + 0.32 * jitter)).max(0.55)
	}

	pub fn facing(self, up: Vec3, outward: Vec3, yaw: f32) -> Vec3 {
		let side = tangent_on(up, yaw);
		(outward * self.radial_splay + up * self.lift + side * 0.22).normalize_or(up)
	}

	pub fn push_cards(
		self,
		leaves: &mut Vec<LeafDescriptor>,
		branch_index: u32,
		branch_id: u32,
		up: Vec3,
		outward: Vec3,
		root: Vec3,
		card: f32,
		recipe: &TreeRecipe,
	) {
		for clump_i in 0..self.clumps {
			if leaves.len() as u32 >= MAX_LEAVES_PER_PLANT {
				return;
			}
			let clump_id = hash_u32(branch_id ^ (0x300 + self.shell * 19 + clump_i));
			let pal = recipe.foliage.varied(root.xz(), clump_id);
			let base_yaw = (clump_i as f32 + hash_u32_to_unit(clump_id)) * std::f32::consts::TAU
				/ self.clumps.max(1) as f32;
			for card_i in 0..self.cards_per_clump {
				if leaves.len() as u32 >= MAX_LEAVES_PER_PLANT {
					return;
				}
				let leaf_id = clump_id.wrapping_mul(5).wrapping_add(card_i);
				let rank_hash = hash_u32_to_unit(leaf_id ^ 0x85ebca6b);
				if rank_hash >= recipe.foliage_density {
					continue;
				}
				let jitter = hash_u32_to_unit(leaf_id ^ 0x91);
				let yaw = base_yaw + (card_i as f32 - 0.45) * 0.42;
				leaves.push(LeafDescriptor {
					plant_index: 0,
					parent_branch: branch_index,
					attach_t: self.attach_t(clump_i, jitter),
					size_m: self.size_m(card, jitter),
					twist: (hash_u32_to_unit(leaf_id ^ 0xb1) - 0.5) * 1.35,
					fold: 0.55 + 0.32 * hash_u32_to_unit(leaf_id ^ 0xc1),
					flutter: 0.028 + 0.048 * hash_u32_to_unit(leaf_id ^ 0xd1),
					density_rank: self.density_rank(rank_hash),
					facing: self.facing(up, outward, yaw),
					leaf_id,
					palette: pal.tip_rgb,
					shape: self.shell_unit(),
				});
			}
		}
	}
}

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
		let mut limb_sites = Vec::new();
		let mut fork_sites = Vec::new();
		let mut twig_sites = Vec::new();

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

			limb_sites.push((limb_index, limb_id, dir, leaf_size));

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
				fork_sites.push((fork_index, fork_id, fork_dir, leaf_size * 0.92));

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
				twig_sites.push((twig_index, twig_id, twig_dir, leaf_size * 0.82));
			}
		}

		if recipe.foliage_density > 0.0 {
			// Outer shells on every limb first so a capacity clamp keeps the crown.
			for layer in CanopyLayer::shells() {
				for &(index, id, dir, size) in &limb_sites {
					layer.push_cards(&mut leaves, index, id, upn, dir, root, size, recipe);
				}
			}
			for layer in CanopyLayer::rim_shells() {
				for &(index, id, dir, size) in &fork_sites {
					layer.push_cards(&mut leaves, index, id, upn, dir, root, size, recipe);
				}
			}
			for &(index, id, dir, size) in &twig_sites {
				CanopyLayer::outer().push_cards(&mut leaves, index, id, upn, dir, root, size, recipe);
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

	#[test]
	fn canopy_layers_cover_inner_mid_and_outer_shells() -> anyhow::Result<()> {
		let plant = TreePlant::generate(8, 0, Vec3::ZERO, Vec3::Y, 1.0, &TreeRecipe::storybook());
		let mut shells = [0u32; 3];
		for leaf in &plant.leaves {
			shells[leaf.layer_shell() as usize] += 1;
		}
		assert!(shells[0] > 0 && shells[1] > 0 && shells[2] > 0, "shells {shells:?}");
		let inner = plant.leaves.iter().filter(|leaf| leaf.layer_shell() == 0).count();
		let outer = plant.leaves.iter().filter(|leaf| leaf.layer_shell() == 2).count();
		assert!(outer >= inner);
		assert_eq!(CanopyLayer::inner().shell, 0);
		assert_eq!(CanopyLayer::mid().shell, 1);
		assert_eq!(CanopyLayer::outer().shell, 2);
		Ok(())
	}

	#[test]
	fn every_limb_keeps_outer_canopy() -> anyhow::Result<()> {
		let plant = TreePlant::generate(8, 0, Vec3::ZERO, Vec3::Y, 1.0, &TreeRecipe::storybook());
		for (index, branch) in plant.branches.iter().enumerate() {
			if branch.depth != 1 {
				continue;
			}
			assert!(
				plant.leaves.iter().any(|leaf| leaf.parent_branch == index as u32),
				"limb {index} has no canopy cards"
			);
		}
		Ok(())
	}
}
