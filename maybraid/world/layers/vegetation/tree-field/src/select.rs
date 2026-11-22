//! CPU selection matching the wood/foliage compute shaders.

use vegetation_grass_field::WorldFrustum;
use vegetation_shaders::wesl::{
	lod_bucket, nested_keep, DrawIndexedIndirect, GrassViewUniform, LodThresholds,
};

use crate::field::TreeFieldSettings;
use crate::layout::{
	BranchDescriptor, LeafDescriptor, PlantDescriptor, WoodLod, PARENT_NONE, TREE_LOD_COUNT,
};
use crate::template::LeafTemplate;

/// Compacted wood + foliage instance lists plus overflow counts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompactedTreeLists {
	pub wood: [Vec<u32>; TREE_LOD_COUNT],
	pub foliage: [Vec<u32>; TREE_LOD_COUNT],
	pub wood_overflow: [u32; TREE_LOD_COUNT],
	pub foliage_overflow: [u32; TREE_LOD_COUNT],
	pub considered_wood: u32,
	pub considered_foliage: u32,
	pub submitted_wood: u32,
	pub submitted_foliage: u32,
}

impl CompactedTreeLists {
	pub fn wood_count(&self, lod: usize) -> u32 {
		self.wood[lod].len() as u32
	}

	pub fn foliage_count(&self, lod: usize) -> u32 {
		self.foliage[lod].len() as u32
	}

	pub fn triangle_count(&self) -> u32 {
		let mut tris = 0u32;
		for lod in 0..TREE_LOD_COUNT {
			let wood = WoodLod::from_bucket(lod as u32).map(|w| w.index_count() / 3).unwrap_or(0);
			tris += wood * self.wood_count(lod);
			tris += (LeafTemplate::TEARDROP_INDEX_COUNT / 3) * self.foliage_count(lod);
		}
		tris
	}

	pub fn submitted(&self) -> u32 {
		self.submitted_wood + self.submitted_foliage
	}

	pub fn wood_indirect_args(&self) -> [DrawIndexedIndirect; TREE_LOD_COUNT] {
		[
			DrawIndexedIndirect::empty(WoodLod::Near.index_count())
				.with_instances(self.wood_count(0)),
			DrawIndexedIndirect::empty(WoodLod::Mid.index_count())
				.with_instances(self.wood_count(1)),
			DrawIndexedIndirect::empty(WoodLod::Far.index_count())
				.with_instances(self.wood_count(2)),
		]
	}

	pub fn foliage_indirect_args(&self) -> [DrawIndexedIndirect; TREE_LOD_COUNT] {
		[
			DrawIndexedIndirect::empty(LeafTemplate::TEARDROP_INDEX_COUNT)
				.with_instances(self.foliage_count(0)),
			DrawIndexedIndirect::empty(LeafTemplate::TEARDROP_INDEX_COUNT)
				.with_instances(self.foliage_count(1)),
			DrawIndexedIndirect::empty(LeafTemplate::TEARDROP_INDEX_COUNT)
				.with_instances(self.foliage_count(2)),
		]
	}
}

/// Mirrors `select_wood.wesl` / `select_foliage.wesl`. Topology-preserving prune.
pub struct TreeSelector {
	pub view: GrassViewUniform,
	pub thresholds: LodThresholds,
	pub capacities: [u32; TREE_LOD_COUNT],
}

impl TreeSelector {
	pub fn from_view(view: GrassViewUniform, settings: &TreeFieldSettings) -> Self {
		Self {
			thresholds: LodThresholds {
				near_m: view.lod_distances.y,
				mid_m: view.lod_distances.z,
				card_m: view.lod_distances.w,
				near_density: view.lod_density.x,
				mid_density: view.lod_density.y,
				card_density: view.lod_density.z,
				min_height_px: view.lod_density.w,
			},
			view,
			capacities: [settings.max_per_lod; TREE_LOD_COUNT],
		}
	}

	pub fn plant_bucket(&self, plant: &PlantDescriptor) -> Option<u32> {
		let camera = self.view.world_position.truncate();
		let radius = plant.conservative_extent_m(self.view.wind.w);
		let frustum = WorldFrustum::from_clip(self.view.clip_from_world);
		let center = plant.root + plant.up * (plant.height_m * 0.5);
		if !frustum.contains_sphere(center, radius) {
			return None;
		}
		let distance = camera.distance(plant.root);
		let bucket = lod_bucket(
			distance,
			plant.height_m,
			0.0,
			self.view.render_height_px(),
			self.view.fov_y_radians(),
			self.thresholds,
		);
		WoodLod::from_bucket(bucket).map(|lod| lod.bucket())
	}

	pub fn compact(
		&self,
		plants: &[PlantDescriptor],
		branches: &[BranchDescriptor],
		leaves: &[LeafDescriptor],
	) -> CompactedTreeLists {
		let mut lists = CompactedTreeLists::default();
		let wood_keep = keep_fraction(self.capacities[0], branches.len() as u32);
		let leaf_keep = keep_fraction(self.capacities[0], leaves.len() as u32);
		for (branch_index, branch) in branches.iter().enumerate() {
			lists.considered_wood += 1;
			let Some(plant) = plants.get(branch.plant_index as usize) else {
				continue;
			};
			let Some(bucket) = self.plant_bucket(plant) else {
				continue;
			};
			let lod = WoodLod::from_bucket(bucket).unwrap_or(WoodLod::Far);
			if branch.depth > lod.max_depth() {
				continue;
			}
			if !branch.parent_is_resolvable(branches.len() as u32) {
				continue;
			}
			if branch.keep_rank >= wood_keep && !branch.is_trunk() {
				continue;
			}
			let slot = bucket as usize;
			if lists.wood[slot].len() as u32 >= self.capacities[slot] {
				lists.wood_overflow[slot] += 1;
				continue;
			}
			lists.wood[slot].push(branch_index as u32);
			lists.submitted_wood += 1;
		}
		for (leaf_index, leaf) in leaves.iter().enumerate() {
			lists.considered_foliage += 1;
			let Some(plant) = plants.get(leaf.plant_index as usize) else {
				continue;
			};
			let Some(bucket) = self.plant_bucket(plant) else {
				continue;
			};
			let lod = WoodLod::from_bucket(bucket).unwrap_or(WoodLod::Far);
			let mut parent_index = leaf.parent_branch;
			let mut hops = 0u32;
			loop {
				let Some(parent) = branches.get(parent_index as usize) else {
					parent_index = PARENT_NONE;
					break;
				};
				if parent.depth <= lod.max_depth() {
					break;
				}
				if parent.major_parent == PARENT_NONE || hops >= 3 {
					parent_index = PARENT_NONE;
					break;
				}
				parent_index = parent.major_parent;
				hops += 1;
			}
			if parent_index == PARENT_NONE {
				continue;
			}
			let density = match lod {
				WoodLod::Near => self.thresholds.near_density,
				WoodLod::Mid => self.thresholds.mid_density,
				WoodLod::Far => self.thresholds.card_density,
			};
			if !nested_keep(leaf.density_rank, density.min(leaf_keep)) {
				continue;
			}
			let slot = bucket as usize;
			if lists.foliage[slot].len() as u32 >= self.capacities[slot] {
				lists.foliage_overflow[slot] += 1;
				continue;
			}
			lists.foliage[slot].push(leaf_index as u32);
			lists.submitted_foliage += 1;
		}
		lists
	}

	pub fn pruning_preserves_parents(
		&self,
		plants: &[PlantDescriptor],
		branches: &[BranchDescriptor],
		lists: &CompactedTreeLists,
	) -> bool {
		let submitted: std::collections::HashSet<u32> =
			lists.wood.iter().flat_map(|list| list.iter().copied()).collect();
		for index in submitted.iter().copied() {
			let Some(branch) = branches.get(index as usize) else {
				return false;
			};
			if branch.is_trunk() {
				continue;
			}
			if plants.get(branch.plant_index as usize).is_none() {
				return false;
			}
			if !submitted.contains(&branch.parent_index) {
				return false;
			}
		}
		true
	}
}

fn keep_fraction(capacity: u32, total: u32) -> f32 {
	if total == 0 {
		1.0
	} else {
		(capacity as f32 / total as f32).min(1.0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::field::{TreeField, TreeFieldSettings};
	use crate::patch::TreePatch;
	use bevy::camera::CameraProjection;
	use bevy::math::{Vec2, Vec3, Vec4};
	use bevy::prelude::{PerspectiveProjection, Transform};

	fn look_view(origin: Vec3, target: Vec3, height_px: f32) -> GrassViewUniform {
		let view_from_world = Transform::from_translation(origin)
			.looking_at(target, Vec3::Y)
			.to_matrix()
			.inverse();
		let projection = PerspectiveProjection {
			fov: 1.0,
			aspect_ratio: 16.0 / 9.0,
			near: 0.1,
			far: 250.0,
			..Default::default()
		};
		let clip = projection.get_clip_from_view() * view_from_world;
		let t = LodThresholds::tree_default();
		GrassViewUniform {
			clip_from_world: clip,
			world_from_clip: clip.inverse(),
			world_position: origin.extend(0.0),
			prev_world_position: origin.extend(0.0),
			resolution: Vec4::new(1280.0, height_px, 1.0, 0.0),
			wind: Vec4::new(0.92, 0.38, 0.0, 0.22),
			lod_distances: Vec4::new(0.85, t.near_m, t.mid_m, t.card_m),
			lod_density: Vec4::new(t.near_density, t.mid_density, t.card_density, t.min_height_px),
			counts: Vec4::new(0.0, 16384.0, 0.0, 0.0),
			light_direction: Vec4::new(0.4, 0.8, 0.3, 0.0),
		}
	}

	#[test]
	fn resident_grove_keeps_wood_and_canopy_beyond_playground_range() {
		let settings = TreeFieldSettings::default();
		let patch = TreePatch::isolated(42, Vec3::ZERO, crate::TreeRecipe::storybook());
		let field = TreeField::rebuild([&patch], &settings);
		let camera = Vec3::new(0., 10., 600.);
		let mut view = look_view(camera, Vec3::new(0., 8., 0.), 720.);
		let transform =
			Transform::from_translation(camera).looking_at(Vec3::new(0., 8., 0.), Vec3::Y);
		let projection = PerspectiveProjection {
			fov: 1.,
			aspect_ratio: 16. / 9.,
			near: 0.1,
			far: 10000.,
			..Default::default()
		};
		view.clip_from_world = projection.get_clip_from_view() * transform.to_matrix().inverse();
		view.world_from_clip = view.clip_from_world.inverse();
		assert_eq!(TreeSelector::from_view(view, &settings).plant_bucket(&field.plants[0]), None);
		view.lod_distances.w = f32::MAX;
		let selector = TreeSelector::from_view(view, &settings);
		assert_eq!(selector.plant_bucket(&field.plants[0]), Some(2));
		let lists = selector.compact(&field.plants, &field.branches, &field.leaves);
		assert!(lists.submitted_wood > 0);
		assert!(lists.submitted_foliage > 0);
	}

	#[test]
	fn pruning_does_not_leave_floating_children() -> anyhow::Result<()> {
		let settings = TreeFieldSettings::default();
		let patch = TreePatch::grove(4, Vec3::ZERO, Vec2::splat(24.0), 12.0);
		let field = TreeField::rebuild([&patch], &settings);
		let selector = TreeSelector::from_view(
			look_view(Vec3::new(8.0, 7.0, 18.0), Vec3::new(8.0, 6.0, 8.0), 720.0),
			&settings,
		);
		let lists = selector.compact(&field.plants, &field.branches, &field.leaves);
		assert!(selector.pruning_preserves_parents(&field.plants, &field.branches, &lists));
		assert!(lists.submitted_wood > 0);
		assert!(lists.submitted_foliage > 0);
		Ok(())
	}

	#[test]
	fn far_lod_drops_twigs_but_keeps_trunks_and_limbs() -> anyhow::Result<()> {
		let settings = TreeFieldSettings::default();
		let patch = TreePatch::grove(6, Vec3::ZERO, Vec2::splat(24.0), 12.0);
		let field = TreeField::rebuild([&patch], &settings);
		let far = TreeSelector::from_view(
			look_view(Vec3::new(12.0, 18.0, 110.0), Vec3::new(12.0, 6.0, 12.0), 720.0),
			&settings,
		)
		.compact(&field.plants, &field.branches, &field.leaves);
		assert_eq!(far.wood_count(0), 0);
		for index in far.wood[2].iter().copied() {
			assert!(field.branches[index as usize].depth <= 1);
		}
		assert!(far.wood[2].iter().any(|&i| field.branches[i as usize].depth == 0));
		assert!(far.wood_count(2) > 0);
		assert!(far.foliage_count(2) > 0);
		Ok(())
	}

	#[test]
	fn off_camera_shadow_view_keeps_casters() -> anyhow::Result<()> {
		let settings = TreeFieldSettings::default();
		let patch = TreePatch::isolated(
			2,
			Vec3::new(40.0, 0.0, 0.0),
			crate::recipe::TreeRecipe::storybook(),
		);
		let field = TreeField::rebuild([&patch], &settings);
		let camera = TreeSelector::from_view(
			look_view(Vec3::new(-40.0, 6.0, 0.0), Vec3::new(-80.0, 0.0, 0.0), 720.0),
			&settings,
		)
		.compact(&field.plants, &field.branches, &field.leaves);
		let mut shadow_view =
			look_view(Vec3::new(40.0, 14.0, 18.0), Vec3::new(40.0, 6.0, 0.0), 1024.0);
		shadow_view.resolution.w = 1.0;
		let shadow = TreeSelector::from_view(shadow_view, &settings).compact(
			&field.plants,
			&field.branches,
			&field.leaves,
		);
		assert_eq!(camera.submitted(), 0);
		assert!(shadow.submitted() > 0);
		Ok(())
	}

	#[test]
	fn lod_reduces_submitted_work() -> anyhow::Result<()> {
		let settings = TreeFieldSettings::default();
		let patch = TreePatch::grove(8, Vec3::ZERO, Vec2::splat(36.0), 12.0);
		let field = TreeField::rebuild([&patch], &settings);
		let near = TreeSelector::from_view(
			look_view(Vec3::new(6.0, 8.0, 14.0), Vec3::new(12.0, 6.0, 12.0), 720.0),
			&settings,
		)
		.compact(&field.plants, &field.branches, &field.leaves);
		let far = TreeSelector::from_view(
			look_view(Vec3::new(18.0, 22.0, 140.0), Vec3::new(18.0, 6.0, 18.0), 720.0),
			&settings,
		)
		.compact(&field.plants, &field.branches, &field.leaves);
		assert!(far.triangle_count() < near.triangle_count());
		Ok(())
	}
}
