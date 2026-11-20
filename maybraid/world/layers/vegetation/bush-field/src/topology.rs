//! Bounded connected skeleton + foliage descriptors. CPU-authored; GPU expands surfaces.

use bevy::math::{Vec3, Vec3Swizzles};
use vegetation_shaders::wesl::{hash_u32, hash_u32_to_unit, plant_id};

use crate::layout::{BranchDescriptor, LeafDescriptor, PlantDescriptor, PARENT_NONE};
use crate::recipe::BushRecipe;

/// Explicit capacity so shaders never walk an unbounded graph.
pub const MAX_BRANCHES_PER_PLANT: u32 = 16;
pub const MAX_LEAVES_PER_PLANT: u32 = 128;
pub const MAX_BRANCH_DEPTH: u32 = 2;

/// One overlapping foliage shell. Inner shells hug the wood; outer shells fan into a dome.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LeafLayer {
	/// `0` inner, `1` mid, `2` outer.
	pub shell: u32,
	pub height_lo: f32,
	pub height_hi: f32,
	pub clumps_per_stem: u32,
	pub cards_per_clump: u32,
	pub size_scale: f32,
	pub radial_splay: f32,
	/// `1` follows the stem; `0` fans toward horizontal.
	pub lift: f32,
}

impl LeafLayer {
	pub const SHELL_COUNT: u32 = 3;

	pub fn inner() -> Self {
		Self {
			shell: 0,
			height_lo: 0.18,
			height_hi: 0.62,
			clumps_per_stem: 3,
			cards_per_clump: 2,
			size_scale: 0.62,
			radial_splay: 0.20,
			lift: 0.78,
		}
	}

	pub fn mid() -> Self {
		Self {
			shell: 1,
			height_lo: 0.30,
			height_hi: 0.84,
			clumps_per_stem: 4,
			cards_per_clump: 2,
			size_scale: 0.82,
			radial_splay: 0.55,
			lift: 0.42,
		}
	}

	pub fn outer() -> Self {
		Self {
			shell: 2,
			height_lo: 0.40,
			height_hi: 0.98,
			clumps_per_stem: 5,
			cards_per_clump: 2,
			size_scale: 0.98,
			radial_splay: 0.96,
			lift: 0.18,
		}
	}

	/// Outer first so a capacity clamp keeps the readable crown.
	pub fn shells() -> [Self; 3] {
		[Self::outer(), Self::mid(), Self::inner()]
	}

	pub fn shell_unit(self) -> f32 {
		self.shell as f32 / (Self::SHELL_COUNT - 1) as f32
	}

	/// Outer shells keep first under LOD (lower `density_rank` is submitted).
	pub fn density_rank(self, hash: f32) -> f32 {
		hash * (1.0 - 0.40 * self.shell_unit())
	}

	pub fn attach_t(self, clump_i: u32, jitter: f32) -> f32 {
		let span = (self.height_hi - self.height_lo).max(0.04);
		let along = (clump_i as f32 + 0.5) / self.clumps_per_stem.max(1) as f32;
		(self.height_lo + span * along + (jitter - 0.5) * 0.08).clamp(0.04, 0.98)
	}

	pub fn size_m(self, card: f32, jitter: f32) -> f32 {
		(card * self.size_scale * (0.84 + 0.28 * jitter)).max(0.05)
	}

	pub fn facing(self, up: Vec3, yaw: f32) -> Vec3 {
		let side = tangent_on(up, yaw);
		(side * self.radial_splay + up * self.lift).normalize_or(up)
	}

	pub fn push_cards(
		self,
		leaves: &mut Vec<LeafDescriptor>,
		branch_index: u32,
		branch_id: u32,
		up: Vec3,
		root: Vec3,
		card: f32,
		recipe: &BushRecipe,
	) {
		for clump_i in 0..self.clumps_per_stem {
			if leaves.len() as u32 >= MAX_LEAVES_PER_PLANT {
				return;
			}
			let clump_id = hash_u32(branch_id ^ (0x300 + self.shell * 17 + clump_i));
			let pal = recipe.foliage.varied(root.xz(), clump_id);
			let base_yaw = (clump_i as f32 + hash_u32_to_unit(clump_id)) * std::f32::consts::TAU
				/ self.clumps_per_stem.max(1) as f32;
			for card_i in 0..self.cards_per_clump {
				if leaves.len() as u32 >= MAX_LEAVES_PER_PLANT {
					return;
				}
				let leaf_id = clump_id.wrapping_mul(4).wrapping_add(card_i);
				let rank_hash = hash_u32_to_unit(leaf_id ^ 0x85ebca6b);
				if rank_hash >= recipe.foliage_density {
					continue;
				}
				let jitter = hash_u32_to_unit(leaf_id ^ 0x91);
				let yaw = base_yaw + (card_i as f32 - 0.45) * 0.38;
				leaves.push(LeafDescriptor {
					plant_index: 0,
					parent_branch: branch_index,
					attach_t: self.attach_t(clump_i, jitter),
					size_m: self.size_m(card, jitter),
					twist: (hash_u32_to_unit(leaf_id ^ 0xb1) - 0.5) * 1.15,
					fold: 0.38 + 0.42 * hash_u32_to_unit(leaf_id ^ 0xc1),
					flutter: 0.05 + 0.10 * hash_u32_to_unit(leaf_id ^ 0xd1),
					density_rank: self.density_rank(rank_hash),
					facing: self.facing(up, yaw),
					leaf_id,
					palette: pal.tip_rgb,
					shape: self.shell_unit(),
				});
			}
		}
	}
}

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

		// Short woody collar plus several stems from the base — a shrub, not a sapling.
		let collar_len = height * 0.14;
		let stem_radius = (height * 0.028).clamp(0.014, 0.034);
		branches.push(BranchDescriptor {
			p0: root,
			parent_index: PARENT_NONE,
			p1: root + upn * (collar_len * 0.52) + tangent_on(upn, 0.0) * (height * 0.012),
			parent_t: 0.0,
			p2: root + upn * collar_len,
			plant_index: 0,
			root_radius_m: stem_radius,
			tip_radius_m: stem_radius * 0.72,
			stiffness: recipe.wood_stiffness,
			keep_rank: 0.0,
			twist: 0.0,
			depth: 0,
			branch_id: id,
			major_parent: PARENT_NONE,
		});

		let mut stem_indices = Vec::new();
		for stem_i in 0..shoot_count {
			if branches.len() as u32 >= MAX_BRANCHES_PER_PLANT {
				break;
			}
			let stem_id = hash_u32(id ^ (0x100 + stem_i));
			let yaw = (stem_i as f32 + hash_u32_to_unit(stem_id)) * std::f32::consts::TAU
				/ shoot_count.max(1) as f32;
			let offset = tangent_on(upn, yaw) * (height * (0.045 + 0.035 * hash_u32_to_unit(stem_id ^ 0x11)));
			let p0 = root + offset;
			let splay = (radial + 0.42).min(0.90);
			let rise = (vertical * 0.55 + 0.38).clamp(0.34, 0.82);
			let stem_h = height * (0.78 + 0.16 * hash_u32_to_unit(stem_id ^ 0x21));
			let outward = tangent_on(upn, yaw);
			// Arch out, then ease back in so the tips form a rounded dome.
			let p1 = p0 + upn * (stem_h * rise * 0.48) + outward * (height * splay * 0.46);
			let p2 = p0 + upn * (stem_h * rise) + outward * (height * splay * 0.30);
			let stem_index = branches.len() as u32;
			branches.push(BranchDescriptor {
				p0,
				parent_index: PARENT_NONE,
				p1,
				parent_t: 0.0,
				p2,
				plant_index: 0,
				root_radius_m: stem_radius * 0.78,
				tip_radius_m: stem_radius * 0.32,
				stiffness: (recipe.wood_stiffness - 0.08).max(0.50),
				keep_rank: 0.0,
				twist: (hash_u32_to_unit(stem_id ^ 0x41) - 0.5) * 0.45,
				depth: 0,
				branch_id: stem_id,
				major_parent: PARENT_NONE,
			});
			stem_indices.push((stem_index, stem_id));

			if depth_budget >= 1 && hash_u32_to_unit(stem_id ^ 0x51) > 0.28 {
				if branches.len() as u32 >= MAX_BRANCHES_PER_PLANT {
					break;
				}
				let twig_id = hash_u32(stem_id ^ 0x61);
				let twig_yaw = yaw + 0.85 * (hash_u32_to_unit(twig_id) - 0.5);
				let twig_dir =
					shoot_direction(upn, twig_yaw, (radial + 0.52).min(1.0), vertical * 0.42 + 0.18);
				let twig_local = child_bezier(stem_h * 0.42, twig_dir, upn, hash_u32_to_unit(twig_id ^ 1));
				branches.push(BranchDescriptor {
					p0: twig_local[0],
					parent_index: stem_index,
					p1: twig_local[1],
					parent_t: 0.62,
					p2: twig_local[2],
					plant_index: 0,
					root_radius_m: stem_radius * 0.30,
					tip_radius_m: stem_radius * 0.14,
					stiffness: (recipe.wood_stiffness - 0.22).max(0.38),
					keep_rank: 0.48 + 0.22 * hash_u32_to_unit(twig_id ^ 0x71),
					twist: (hash_u32_to_unit(twig_id ^ 0x81) - 0.5) * 0.8,
					depth: 1,
					branch_id: twig_id,
					major_parent: stem_index,
				});
			}
		}

		if recipe.foliage_density > 0.0 {
			let card = (leaf_radius * 1.55 + height * 0.10).clamp(0.045, 0.13);
			for layer in LeafLayer::shells() {
				for &(stem_index, stem_id) in &stem_indices {
					layer.push_cards(&mut leaves, stem_index, stem_id, upn, root, card, recipe);
				}
			}
			if depth_budget >= 1 {
				for (branch_i, branch) in branches.iter().enumerate() {
					if branch.depth == 0 {
						continue;
					}
					LeafLayer::outer().push_cards(
						&mut leaves,
						branch_i as u32,
						branch.branch_id,
						upn,
						root,
						card * 0.82,
						recipe,
					);
				}
			}
		}

		let crown = height * (0.62 + 0.22 * radial);
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

	#[test]
	fn shrub_has_several_base_stems() -> anyhow::Result<()> {
		let plant = BushPlant::generate(11, 0, Vec3::ZERO, Vec3::Y, 1.0, &BushRecipe::small_scrub());
		let stems = plant.branches.iter().filter(|branch| branch.is_trunk()).count();
		assert!(stems >= 4, "expected a multi-stem shrub, got {stems}");
		assert!(plant.branches[0].is_trunk());
		assert!((plant.branches[0].p0 - Vec3::ZERO).length() < 1e-4);
		Ok(())
	}

	#[test]
	fn leaf_layers_cover_inner_mid_and_outer_shells() -> anyhow::Result<()> {
		let plant = BushPlant::generate(5, 0, Vec3::ZERO, Vec3::Y, 1.0, &BushRecipe::small_scrub());
		let mut shells = [0u32; 3];
		for leaf in &plant.leaves {
			shells[leaf.layer_shell() as usize] += 1;
		}
		assert!(shells[0] > 0 && shells[1] > 0 && shells[2] > 0, "shells {shells:?}");
		let inner = plant
			.leaves
			.iter()
			.filter(|leaf| leaf.layer_shell() == 0)
			.map(|leaf| leaf.size_m)
			.fold(0.0_f32, f32::max);
		let outer = plant
			.leaves
			.iter()
			.filter(|leaf| leaf.layer_shell() == 2)
			.map(|leaf| leaf.size_m)
			.fold(0.0_f32, f32::max);
		assert!(outer > inner);
		Ok(())
	}

	#[test]
	fn leaf_layer_descriptors_fan_outward() -> anyhow::Result<()> {
		let inner = LeafLayer::inner();
		let mid = LeafLayer::mid();
		let outer = LeafLayer::outer();
		assert_eq!(inner.shell, 0);
		assert_eq!(mid.shell, 1);
		assert_eq!(outer.shell, 2);
		assert!(inner.size_scale < mid.size_scale && mid.size_scale < outer.size_scale);
		assert!(inner.radial_splay < mid.radial_splay && mid.radial_splay < outer.radial_splay);
		assert!(inner.lift > mid.lift && mid.lift > outer.lift);
		assert!(outer.density_rank(0.5) < inner.density_rank(0.5));
		Ok(())
	}

	#[test]
	fn sparse_recipe_keeps_wood_and_some_leaves() -> anyhow::Result<()> {
		let plant = BushPlant::generate(2, 0, Vec3::ZERO, Vec3::Y, 1.0, &BushRecipe::sparse_scrub());
		assert!(plant.branches.iter().filter(|branch| branch.is_trunk()).count() >= 3);
		assert!(!plant.leaves.is_empty());
		assert!(plant.leaves.len() < 40);
		Ok(())
	}
}
