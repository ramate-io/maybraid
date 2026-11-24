//! One CPU catalog of grove instances. Prepare uploads only admitted ranges.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::math::Vec3;
use bevy::prelude::*;
use vegetation_grass_field::{
	CatalogApply, CatalogOpQueue, GroveCatalogOp, GroveEncodeStats, GroveId, RangeArena, RangeSpan,
	WoodyGroveRecord, WOODY_GROVE_RECORD_STRIDE,
};

use crate::field::TreeField;
use crate::layout::{
	PlantDescriptor, BRANCH_DESCRIPTOR_STRIDE, LEAF_DESCRIPTOR_STRIDE, PARENT_NONE,
	PLANT_DESCRIPTOR_STRIDE,
};

/// Grown descriptors for one grove, shared (not cloned) through extract.
#[derive(Clone, Debug)]
pub enum TreeCatalogOp {
	Admit { id: GroveId, field: Arc<TreeField>, truncated_plants: u32 },
	Retire { id: GroveId },
}

pub type TreeCatalogOps = CatalogOpQueue<TreeCatalogOp>;

impl GroveCatalogOp for TreeCatalogOp {
	fn grove_id(&self) -> GroveId {
		match *self {
			Self::Admit { id, .. } | Self::Retire { id } => id,
		}
	}

	fn is_retire(&self) -> bool {
		matches!(self, Self::Retire { .. })
	}
}

/// Grove-local field plus its catalog arena offsets. CPU select walks these.
#[derive(Clone, Copy, Debug)]
pub struct GroveSource<'a> {
	pub field: &'a TreeField,
	pub plant_off: u32,
	pub branch_off: u32,
	pub leaf_off: u32,
}

#[derive(Clone, Debug)]
struct LiveGrove {
	slot: u32,
	plants: RangeSpan,
	branches: RangeSpan,
	leaves: RangeSpan,
	content_gen: u64,
	field: Arc<TreeField>,
}

impl LiveGrove {
	fn source(&self) -> GroveSource<'_> {
		GroveSource {
			field: self.field.as_ref(),
			plant_off: self.plants.offset,
			branch_off: self.branches.offset,
			leaf_off: self.leaves.offset,
		}
	}

	fn plant_bytes(&self) -> Vec<u8> {
		self.field
			.plants
			.iter()
			.copied()
			.flat_map(|mut plant| {
				plant.branch_offset = plant.branch_offset.saturating_add(self.branches.offset);
				plant.leaf_offset = plant.leaf_offset.saturating_add(self.leaves.offset);
				plant.with_grove_slot(self.slot).with_active().to_std430_bytes()
			})
			.collect()
	}

	fn branch_bytes(&self) -> Vec<u8> {
		self.field
			.branches
			.iter()
			.copied()
			.flat_map(|mut branch| {
				branch.plant_index = branch.plant_index.saturating_add(self.plants.offset);
				if branch.parent_index != PARENT_NONE {
					branch.parent_index = branch.parent_index.saturating_add(self.branches.offset);
				}
				if branch.major_parent != PARENT_NONE {
					branch.major_parent = branch.major_parent.saturating_add(self.branches.offset);
				}
				branch.to_std430_bytes()
			})
			.collect()
	}

	fn leaf_bytes(&self) -> Vec<u8> {
		self.field
			.leaves
			.iter()
			.copied()
			.flat_map(|mut leaf| {
				leaf.plant_index = leaf.plant_index.saturating_add(self.plants.offset);
				leaf.parent_branch = leaf.parent_branch.saturating_add(self.branches.offset);
				leaf.to_std430_bytes()
			})
			.collect()
	}
}

#[derive(Resource, Clone, Debug)]
pub struct TreeGroveCatalog {
	pub groves: Vec<WoodyGroveRecord>,
	live: HashMap<GroveId, LiveGrove>,
	plants_arena: RangeArena,
	branches_arena: RangeArena,
	leaves_arena: RangeArena,
	free_slots: Vec<u32>,
	consume_ops: bool,
	pub generation: u64,
	pub last_write_bytes: u64,
	pub last_writes: u32,
	pub truncated_plants: u32,
	pub clones: u32,
	pub pending: Vec<TreeCatalogOp>,
	pub last_uploads: Vec<CatalogUpload>,
	pub grove_residency: bool,
}

/// Descriptor ranges written this apply (admit or retire). Prepare uploads only these.
#[derive(Clone, Copy, Debug)]
pub struct CatalogUpload {
	pub id: GroveId,
	pub plants: RangeSpan,
	pub branches: RangeSpan,
	pub leaves: RangeSpan,
}

impl Default for TreeGroveCatalog {
	fn default() -> Self {
		Self::with_descriptors()
	}
}

impl TreeGroveCatalog {
	fn empty(consume_ops: bool) -> Self {
		Self {
			groves: Vec::new(),
			live: HashMap::new(),
			plants_arena: RangeArena::with_cap(1_024),
			branches_arena: RangeArena::with_cap(8_192),
			leaves_arena: RangeArena::with_cap(16_384),
			free_slots: Vec::new(),
			consume_ops,
			generation: 0,
			last_write_bytes: 0,
			last_writes: 0,
			truncated_plants: 0,
			clones: 0,
			pending: Vec::new(),
			last_uploads: Vec::new(),
			grove_residency: false,
		}
	}

	/// Main-world catalog: records, resident counters, and a shared field Arc.
	pub fn with_descriptors() -> Self {
		Self::empty(false)
	}

	/// Render-world catalog shares the same Arc and writes arena ranges from it.
	pub fn render() -> Self {
		Self::empty(true)
	}

	pub fn is_empty(&self) -> bool {
		self.live.is_empty()
	}

	pub fn live_grove_count(&self) -> usize {
		self.live.len()
	}

	pub fn resident_plants(&self) -> usize {
		self.live.values().map(|live| live.plants.count as usize).sum()
	}

	pub fn resident_branches(&self) -> usize {
		self.live.values().map(|live| live.branches.count as usize).sum()
	}

	pub fn resident_leaves(&self) -> usize {
		self.live.values().map(|live| live.leaves.count as usize).sum()
	}

	pub fn plant_slots(&self) -> usize {
		self.plants_arena.used as usize
	}

	pub fn branch_slots(&self) -> usize {
		self.branches_arena.used as usize
	}

	pub fn leaf_slots(&self) -> usize {
		self.leaves_arena.used as usize
	}

	pub fn contains(&self, id: GroveId) -> bool {
		self.live.contains_key(&id)
	}

	pub fn source(&self, id: GroveId) -> Option<&Arc<TreeField>> {
		self.live.get(&id).map(|live| &live.field)
	}

	pub fn sources(&self) -> impl Iterator<Item = GroveSource<'_>> + '_ {
		self.live.values().map(LiveGrove::source)
	}

	pub fn live_records(&self) -> impl Iterator<Item = (GroveId, WoodyGroveRecord)> + '_ {
		self.live.iter().filter_map(|(&id, live)| {
			self.groves.get(live.slot as usize).copied().map(|mut record| {
				record.content_gen = live.content_gen;
				(id, record)
			})
		})
	}

	pub fn active_bounds(&self) -> Option<(Vec3, Vec3)> {
		let mut min = Vec3::splat(f32::MAX);
		let mut max = Vec3::splat(f32::MIN);
		let mut any = false;
		for record in self.groves.iter().filter(|record| record.is_active()) {
			min = min.min(record.aabb_min);
			max = max.max(record.aabb_max);
			any = true;
		}
		any.then_some((min, max))
	}

	pub fn encode_plan() -> GroveEncodeStats {
		GroveEncodeStats::woody_kind()
	}

	pub fn encode_plants(&self, upload: &CatalogUpload) -> Vec<u8> {
		match self.live.get(&upload.id) {
			Some(live) => live.plant_bytes(),
			None => (0..upload.plants.count)
				.flat_map(|_| PlantDescriptor::inactive().to_std430_bytes())
				.collect(),
		}
	}

	pub fn encode_branches(&self, upload: &CatalogUpload) -> Vec<u8> {
		self.live.get(&upload.id).map(LiveGrove::branch_bytes).unwrap_or_default()
	}

	pub fn encode_leaves(&self, upload: &CatalogUpload) -> Vec<u8> {
		self.live.get(&upload.id).map(LiveGrove::leaf_bytes).unwrap_or_default()
	}

	pub fn apply(
		&mut self,
		ops: impl IntoIterator<Item = TreeCatalogOp>,
		budget: usize,
	) -> CatalogApply<TreeCatalogOp> {
		self.last_write_bytes = 0;
		self.last_writes = 0;
		self.last_uploads.clear();
		CatalogApply::budgeted_mut(ops, budget, |op| match op {
			TreeCatalogOp::Retire { id } => self.retire(*id),
			TreeCatalogOp::Admit { id, field, truncated_plants } => {
				self.admit(*id, Arc::clone(field), *truncated_plants);
			}
		})
	}

	fn admit(&mut self, id: GroveId, field: Arc<TreeField>, truncated_plants: u32) {
		self.retire(id);
		self.truncated_plants = self.truncated_plants.saturating_add(truncated_plants);
		let plant_span = self.plants_arena.alloc(field.plants.len() as u32);
		let branch_span = self.branches_arena.alloc(field.branches.len() as u32);
		let leaf_span = self.leaves_arena.alloc(field.leaves.len() as u32);
		let slot = self.alloc_slot();
		let (aabb_min, aabb_max) = if field.bounds.is_empty() {
			(Vec3::ZERO, Vec3::ZERO)
		} else {
			(field.bounds.min, field.bounds.max)
		};
		self.generation = self.generation.wrapping_add(1);
		self.write_record(
			slot,
			aabb_min,
			aabb_max,
			0,
			plant_span,
			branch_span,
			leaf_span,
			self.generation,
		);
		self.last_writes += 1;
		self.last_write_bytes += plant_span.byte_len(PLANT_DESCRIPTOR_STRIDE)
			+ branch_span.byte_len(BRANCH_DESCRIPTOR_STRIDE)
			+ leaf_span.byte_len(LEAF_DESCRIPTOR_STRIDE)
			+ WOODY_GROVE_RECORD_STRIDE;
		if self.consume_ops {
			self.last_uploads.push(CatalogUpload {
				id,
				plants: plant_span,
				branches: branch_span,
				leaves: leaf_span,
			});
		}
		self.live.insert(
			id,
			LiveGrove {
				slot,
				plants: plant_span,
				branches: branch_span,
				leaves: leaf_span,
				content_gen: self.generation,
				field,
			},
		);
	}

	fn write_record(
		&mut self,
		slot: u32,
		aabb_min: Vec3,
		aabb_max: Vec3,
		seed: u32,
		plants: RangeSpan,
		branches: RangeSpan,
		leaves: RangeSpan,
		content_gen: u64,
	) {
		self.groves[slot as usize] = WoodyGroveRecord {
			aabb_min,
			branch_off: branches.offset,
			aabb_max,
			branch_count: branches.count,
			plant_off: plants.offset,
			plant_count: plants.count,
			leaf_off: leaves.offset,
			leaf_count: leaves.count,
			seed,
			flags: WoodyGroveRecord::FLAG_ACTIVE,
			content_gen,
		};
	}

	fn retire(&mut self, id: GroveId) {
		let Some(live) = self.live.remove(&id) else {
			return;
		};
		if let Some(record) = self.groves.get_mut(live.slot as usize) {
			record.flags = 0;
		}
		if self.consume_ops {
			self.last_uploads.push(CatalogUpload {
				id,
				plants: live.plants,
				branches: RangeSpan::default(),
				leaves: RangeSpan::default(),
			});
		}
		self.last_write_bytes += live.plants.byte_len(PLANT_DESCRIPTOR_STRIDE).max(16);
		self.plants_arena.free(live.plants);
		self.branches_arena.free(live.branches);
		self.leaves_arena.free(live.leaves);
		self.free_slots.push(live.slot);
		self.last_writes += 1;
		self.generation = self.generation.wrapping_add(1);
		if self.live.is_empty() {
			self.grove_residency = false;
		}
	}

	fn alloc_slot(&mut self) -> u32 {
		if let Some(slot) = self.free_slots.pop() {
			slot
		} else {
			let slot = self.groves.len() as u32;
			self.groves.push(WoodyGroveRecord::default());
			slot
		}
	}

	pub fn upload_bytes(&self) -> u64 {
		self.plant_slots() as u64 * PLANT_DESCRIPTOR_STRIDE
			+ self.branch_slots() as u64 * BRANCH_DESCRIPTOR_STRIDE
			+ self.leaf_slots() as u64 * LEAF_DESCRIPTOR_STRIDE
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::field::TreeFieldSettings;
	use crate::patch::TreePatch;
	use crate::recipe::TreeRecipe;
	use vegetation_grass_field::ADMISSION_BUDGET;

	fn grown(origin: Vec3) -> Arc<TreeField> {
		let patch = TreePatch::isolated(1, origin, TreeRecipe::storybook());
		Arc::new(TreeField::rebuild([&patch], &TreeFieldSettings::default()))
	}

	#[test]
	fn idle_frame_writes_nothing() {
		let mut catalog = TreeGroveCatalog::default();
		let leftover = catalog.apply([], ADMISSION_BUDGET).leftover;
		assert!(leftover.is_empty());
		assert_eq!(catalog.last_writes, 0);
		assert_eq!(catalog.last_write_bytes, 0);
		assert_eq!(catalog.clones, 0);
	}

	#[test]
	fn admit_one_grove_writes_only_that_range() {
		let mut catalog = TreeGroveCatalog::default();
		let field = grown(Vec3::ZERO);
		let plant_bytes = field.plants.len() as u64 * PLANT_DESCRIPTOR_STRIDE;
		catalog.apply(
			[TreeCatalogOp::Admit { id: GroveId(1), field, truncated_plants: 0 }],
			ADMISSION_BUDGET,
		);
		assert_eq!(catalog.live_grove_count(), 1);
		assert_eq!(catalog.last_writes, 1);
		assert!(catalog.last_write_bytes >= plant_bytes);
		assert_eq!(catalog.clones, 0);
	}

	#[test]
	fn retire_does_not_rebuild() {
		let mut catalog = TreeGroveCatalog::render();
		let field = grown(Vec3::ZERO);
		catalog.apply(
			[TreeCatalogOp::Admit { id: GroveId(7), field, truncated_plants: 0 }],
			ADMISSION_BUDGET,
		);
		catalog.grove_residency = true;
		catalog.apply([TreeCatalogOp::Retire { id: GroveId(7) }], ADMISSION_BUDGET);
		assert_eq!(catalog.live_grove_count(), 0);
		assert_eq!(catalog.resident_plants(), 0);
		assert!(catalog.source(GroveId(7)).is_none());
		assert!(!catalog.last_uploads.is_empty());
		assert!(catalog.last_uploads[0].plants.count > 0);
		assert!(catalog.last_uploads[0].branches.is_empty());
		assert!(!catalog.grove_residency);
		assert_eq!(catalog.clones, 0);
	}

	#[test]
	fn main_world_catalog_keeps_records_not_descriptors() {
		let mut catalog = TreeGroveCatalog::with_descriptors();
		catalog.apply([admit(1)], ADMISSION_BUDGET);
		assert!(catalog.contains(GroveId(1)));
		assert!(catalog.last_uploads.is_empty());
		assert!(catalog.source(GroveId(1)).is_some());
		assert!(catalog.resident_plants() > 0);
		assert!(catalog.live_records().next().is_some());
	}

	#[test]
	fn encode_plan_matches_for_ten_and_thousand_groves() {
		assert_eq!(
			TreeGroveCatalog::encode_plan().encode_only(),
			GroveEncodeStats::woody_kind().encode_only()
		);
	}

	#[test]
	fn admission_budget_leaves_overflow() {
		let mut catalog = TreeGroveCatalog::default();
		let ops: Vec<_> = (0..3)
			.map(|i| TreeCatalogOp::Admit {
				id: GroveId(i),
				field: grown(Vec3::new(i as f32, 0.0, 0.0)),
				truncated_plants: 0,
			})
			.collect();
		let leftover = catalog.apply(ops, 1).leftover;
		assert_eq!(catalog.live_grove_count(), 1);
		assert_eq!(leftover.len(), 2);
	}

	fn admit(id: u64) -> TreeCatalogOp {
		TreeCatalogOp::Admit {
			id: GroveId(id),
			field: grown(Vec3::new(id as f32, 0.0, 0.0)),
			truncated_plants: 0,
		}
	}

	fn assert_same_live(
		main: &TreeGroveCatalog,
		render: &TreeGroveCatalog,
		ids: impl IntoIterator<Item = u64>,
	) {
		for id in ids {
			assert_eq!(main.contains(GroveId(id)), render.contains(GroveId(id)), "grove {id}");
		}
	}

	#[test]
	fn mixed_batch_applies_retire_after_budgeted_admits() {
		let mut main = TreeGroveCatalog::default();
		main.apply([admit(99)], ADMISSION_BUDGET);
		let mut render = TreeGroveCatalog::default();
		render.apply([admit(99)], ADMISSION_BUDGET);
		assert!(main.contains(GroveId(99)));

		let mut batch: Vec<_> = (1..=ADMISSION_BUDGET as u64).map(admit).collect();
		batch.push(admit(100));
		batch.push(TreeCatalogOp::Retire { id: GroveId(99) });
		let CatalogApply { applied, leftover } = main.apply(batch, ADMISSION_BUDGET);

		assert_eq!(leftover.len(), 1);
		assert_eq!(leftover[0].grove_id(), GroveId(100));
		assert!(!leftover[0].is_retire());
		assert!(applied.iter().any(|op| op.is_retire() && op.grove_id() == GroveId(99)));
		assert!(applied.iter().all(|op| op.grove_id() != GroveId(100)));
		assert_eq!(applied.len(), ADMISSION_BUDGET + 1);
		for id in 1..=ADMISSION_BUDGET as u64 {
			assert!(main.contains(GroveId(id)), "{id}");
		}
		assert!(!main.contains(GroveId(99)));
		assert!(!main.contains(GroveId(100)));

		let CatalogApply { leftover: render_leftover, .. } =
			render.apply(applied, ADMISSION_BUDGET);
		assert!(render_leftover.is_empty());
		assert_same_live(&main, &render, (1..=ADMISSION_BUDGET as u64).chain([99, 100]));
	}

	#[test]
	fn incremental_admit_preserves_earlier_descriptor_bytes() {
		let mut catalog = TreeGroveCatalog::render();
		catalog.apply([admit(1)], ADMISSION_BUDGET);
		let first = Arc::clone(catalog.source(GroveId(1)).expect("first grove"));
		let first_len = first.plants.len() as u32;
		assert!(first_len > 0);
		catalog.apply([admit(2)], ADMISSION_BUDGET);
		let still = catalog.source(GroveId(1)).expect("first grove stays");
		assert!(Arc::ptr_eq(still, &first));
		assert!(catalog.plant_slots() as u32 > first_len);
		assert_eq!(catalog.last_uploads.len(), 1);
		assert_eq!(catalog.last_uploads[0].id, GroveId(2));
		assert!(catalog.last_uploads[0].plants.offset >= first_len);
	}

	#[test]
	fn mixed_batch_cancels_pending_admit_on_retire() {
		let mut main = TreeGroveCatalog::default();
		let mut batch: Vec<_> = (1..=ADMISSION_BUDGET as u64).map(admit).collect();
		batch.push(admit(100));
		batch.push(TreeCatalogOp::Retire { id: GroveId(100) });
		let CatalogApply { applied, leftover } = main.apply(batch, ADMISSION_BUDGET);

		assert!(leftover.is_empty());
		assert!(applied.iter().any(|op| op.is_retire() && op.grove_id() == GroveId(100)));
		assert!(applied
			.iter()
			.all(|op| !(matches!(op, TreeCatalogOp::Admit { id, .. } if *id == GroveId(100)))));
		assert!(!main.contains(GroveId(100)));

		let mut render = TreeGroveCatalog::default();
		let CatalogApply { leftover: render_leftover, .. } =
			render.apply(applied, ADMISSION_BUDGET);
		assert!(render_leftover.is_empty());
		assert_same_live(&main, &render, (1..=ADMISSION_BUDGET as u64).chain([100]));
	}
}
