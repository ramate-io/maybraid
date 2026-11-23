//! CPU selection matching the wood/foliage compute shaders.

use vegetation_grass_field::{
	map_work_index, visible_spans_from_records, GroveVisibleSpan, WorldFrustum, WoodyGroveRecord,
};
use vegetation_shaders::wesl::{lod_bucket, DrawIndexedIndirect, GrassViewUniform, LodThresholds};

use crate::field::BushFieldSettings;
use crate::layout::{
	BranchDescriptor, LeafDescriptor, PlantDescriptor, WoodLod, BUSH_LOD_COUNT, PARENT_NONE,
};
use crate::template::LeafTemplate;

/// Compacted wood + foliage instance lists plus overflow counts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompactedBushLists {
	pub wood: [Vec<u32>; BUSH_LOD_COUNT],
	pub foliage: [Vec<u32>; BUSH_LOD_COUNT],
	pub wood_overflow: [u32; BUSH_LOD_COUNT],
	pub foliage_overflow: [u32; BUSH_LOD_COUNT],
	pub considered_wood: u32,
	pub considered_foliage: u32,
	pub submitted_wood: u32,
	pub submitted_foliage: u32,
}

impl CompactedBushLists {
	pub fn wood_count(&self, lod: usize) -> u32 {
		self.wood[lod].len() as u32
	}

	pub fn foliage_count(&self, lod: usize) -> u32 {
		self.foliage[lod].len() as u32
	}

	pub fn triangle_count(&self) -> u32 {
		let mut tris = 0u32;
		for lod in 0..BUSH_LOD_COUNT {
			let wood = WoodLod::from_bucket(lod as u32).map(|w| w.index_count() / 3).unwrap_or(0);
			tris += wood * self.wood_count(lod);
			tris += (LeafTemplate::TEARDROP_INDEX_COUNT / 3) * self.foliage_count(lod);
		}
		tris
	}

	pub fn submitted(&self) -> u32 {
		self.submitted_wood + self.submitted_foliage
	}

	pub fn wood_indirect_args(&self) -> [DrawIndexedIndirect; BUSH_LOD_COUNT] {
		[
			DrawIndexedIndirect::empty(WoodLod::Near.index_count())
				.with_instances(self.wood_count(0)),
			DrawIndexedIndirect::empty(WoodLod::Mid.index_count())
				.with_instances(self.wood_count(1)),
			DrawIndexedIndirect::empty(WoodLod::Far.index_count())
				.with_instances(self.wood_count(2)),
		]
	}

	pub fn foliage_indirect_args(&self) -> [DrawIndexedIndirect; BUSH_LOD_COUNT] {
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
pub struct BushSelector {
	pub view: GrassViewUniform,
	pub thresholds: LodThresholds,
	pub capacities: [u32; BUSH_LOD_COUNT],
}

impl BushSelector {
	pub fn from_view(view: GrassViewUniform, settings: &BushFieldSettings) -> Self {
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
			capacities: [settings.max_per_lod; BUSH_LOD_COUNT],
		}
	}

	pub fn plant_bucket(&self, plant: &PlantDescriptor) -> Option<u32> {
		if !plant.is_active() {
			return None;
		}
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

	pub fn compact_grove_catalog(
		&self,
		groves: &[WoodyGroveRecord],
		plants: &[PlantDescriptor],
		branches: &[BranchDescriptor],
		leaves: &[LeafDescriptor],
	) -> CompactedBushLists {
		let height = |record: WoodyGroveRecord| {
			plants
				.get(record.plant_off as usize)
				.map(|plant| plant.height_m)
				.unwrap_or(record.extent_radius())
		};
		let (spans, _, _) = visible_spans_from_records(
			&self.view,
			self.thresholds,
			groves,
			height,
			self.view.wind.w,
		);
		self.compact_spans(&spans, plants, branches, leaves)
	}

	pub fn compact(
		&self,
		plants: &[PlantDescriptor],
		branches: &[BranchDescriptor],
		leaves: &[LeafDescriptor],
	) -> CompactedBushLists {
		let spans = [GroveVisibleSpan {
			branch_off: 0,
			branch_count: branches.len() as u32,
			leaf_off: 0,
			leaf_count: leaves.len() as u32,
		}];
		self.compact_spans(&spans, plants, branches, leaves)
	}

	fn compact_spans(
		&self,
		spans: &[GroveVisibleSpan],
		plants: &[PlantDescriptor],
		branches: &[BranchDescriptor],
		leaves: &[LeafDescriptor],
	) -> CompactedBushLists {
		let mut lists = CompactedBushLists::default();
		let wood_keep = keep_fraction(self.capacities[0], branches.len() as u32);
		let leaf_keep = keep_fraction(self.capacities[0], leaves.len() as u32);
		for work in 0..spans.iter().map(|s| s.branch_count).sum::<u32>() {
			let Some(branch_index) = map_work_index(spans, work, true) else {
				continue;
			};
			let branch_index = branch_index as usize;
			let Some(branch) = branches.get(branch_index) else {
				continue;
			};
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
		for work in 0..spans.iter().map(|s| s.leaf_count).sum::<u32>() {
			let Some(leaf_index) = map_work_index(spans, work, false) else {
				continue;
			};
			let leaf_index = leaf_index as usize;
			let Some(leaf) = leaves.get(leaf_index) else {
				continue;
			};
			lists.considered_foliage += 1;
			let Some(plant) = plants.get(leaf.plant_index as usize) else {
				continue;
			};
			let Some(bucket) = self.plant_bucket(plant) else {
				continue;
			};
			let lod = WoodLod::from_bucket(bucket).unwrap_or(WoodLod::Far);
			let mut parent_index = leaf.parent_branch;
			if let Some(parent) = branches.get(parent_index as usize) {
				if parent.depth > lod.max_depth() {
					parent_index = parent.major_parent;
					let Some(major) = branches.get(parent_index as usize) else {
						continue;
					};
					if major.depth > lod.max_depth() || parent_index == PARENT_NONE {
						continue;
					}
				}
			} else {
				continue;
			}
			let density = match lod {
				WoodLod::Near => self.thresholds.near_density,
				WoodLod::Mid => self.thresholds.mid_density,
				WoodLod::Far => self.thresholds.card_density,
			};
			if leaf.density_rank >= density.min(leaf_keep) {
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
		lists: &CompactedBushLists,
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
	use crate::field::{BushField, BushFieldSettings};
	use crate::patch::BushPatch;
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
		let t = LodThresholds::bush_default();
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
	fn culled_grove_catalog_has_zero_pass2_work() {
		let settings = BushFieldSettings::default();
		let thresholds = LodThresholds::grove_bush();
		let view = look_view(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, 5.0, 500.0), 720.0);
		let mut view = view;
		view.lod_distances = Vec4::new(0.85, thresholds.near_m, thresholds.mid_m, thresholds.card_m);
		let selector = BushSelector::from_view(view, &settings);
		let far = WoodyGroveRecord {
			aabb_min: Vec3::new(400.0, 0.0, 400.0),
			aabb_max: Vec3::new(410.0, 8.0, 410.0),
			branch_off: 10,
			branch_count: 50,
			leaf_off: 100,
			leaf_count: 200,
			flags: WoodyGroveRecord::FLAG_ACTIVE,
			..Default::default()
		};
		let lists = selector.compact_grove_catalog(&[far], &[], &[], &[]);
		assert_eq!(lists.submitted_wood, 0);
		assert_eq!(lists.submitted_foliage, 0);
	}

	#[test]
	fn pruning_does_not_leave_floating_children() -> anyhow::Result<()> {
		let settings = BushFieldSettings::default();
		let patch = BushPatch::dense_scrub(4, Vec3::ZERO, Vec2::splat(8.0), 1.4);
		let field = BushField::rebuild([&patch], &settings);
		let selector = BushSelector::from_view(
			look_view(Vec3::new(4.0, 2.0, 12.0), Vec3::new(4.0, 0.3, 4.0), 720.0),
			&settings,
		);
		let lists = selector.compact(&field.plants, &field.branches, &field.leaves);
		assert!(selector.pruning_preserves_parents(&field.plants, &field.branches, &lists));
		assert!(lists.submitted_wood > 0);
		assert!(lists.submitted_foliage > 0);
		Ok(())
	}

	#[test]
	fn far_lod_drops_twigs_but_keeps_trunks() -> anyhow::Result<()> {
		let settings = BushFieldSettings::default();
		let patch = BushPatch::dense_scrub(6, Vec3::ZERO, Vec2::splat(10.0), 1.6);
		let field = BushField::rebuild([&patch], &settings);
		let far = BushSelector::from_view(
			look_view(Vec3::new(5.0, 8.0, 90.0), Vec3::new(5.0, 0.0, 5.0), 720.0),
			&settings,
		)
		.compact(&field.plants, &field.branches, &field.leaves);
		assert_eq!(far.wood_count(0), 0);
		for index in far.wood[2].iter().copied() {
			assert_eq!(field.branches[index as usize].depth, 0);
		}
		assert!(far.wood_count(2) > 0);
		Ok(())
	}

	#[test]
	fn off_camera_shadow_view_keeps_casters() -> anyhow::Result<()> {
		let settings = BushFieldSettings::default();
		let patch = BushPatch::isolated(
			2,
			Vec3::new(40.0, 0.0, 0.0),
			crate::recipe::BushRecipe::small_scrub(),
		);
		let field = BushField::rebuild([&patch], &settings);
		let camera = BushSelector::from_view(
			look_view(Vec3::new(-20.0, 2.0, 0.0), Vec3::new(-40.0, 0.0, 0.0), 720.0),
			&settings,
		)
		.compact(&field.plants, &field.branches, &field.leaves);
		let mut shadow_view =
			look_view(Vec3::new(40.0, 4.0, 12.0), Vec3::new(40.0, 0.0, 0.0), 1024.0);
		shadow_view.resolution.w = 1.0;
		let shadow = BushSelector::from_view(shadow_view, &settings).compact(
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
		let settings = BushFieldSettings::default();
		let patch = BushPatch::dense_scrub(8, Vec3::ZERO, Vec2::splat(12.0), 1.3);
		let field = BushField::rebuild([&patch], &settings);
		let near = BushSelector::from_view(
			look_view(Vec3::new(2.0, 1.2, 4.0), Vec3::new(4.0, 0.3, 4.0), 720.0),
			&settings,
		)
		.compact(&field.plants, &field.branches, &field.leaves);
		let far = BushSelector::from_view(
			look_view(Vec3::new(6.0, 10.0, 80.0), Vec3::new(6.0, 0.0, 6.0), 720.0),
			&settings,
		)
		.compact(&field.plants, &field.branches, &field.leaves);
		assert!(far.triangle_count() < near.triangle_count());
		Ok(())
	}
}
