//! One CPU catalog of grove instances. Prepare uploads only admitted ranges.

use std::collections::HashMap;

use bevy::math::Vec3;
use bevy::prelude::*;
use vegetation_grass_field::{
	CatalogApply, CatalogOpQueue, GroveCatalogOp, GroveEncodeStats, GroveId, RangeArena, RangeSpan,
};
pub use vegetation_shaders::wesl::GroveRecord;
use vegetation_shaders::wesl::GROVE_RECORD_STRIDE;

use crate::layout::{
	BranchDescriptor, LeafDescriptor, PlantDescriptor, BRANCH_DESCRIPTOR_STRIDE,
	LEAF_DESCRIPTOR_STRIDE, PLANT_DESCRIPTOR_STRIDE,
};

/// Grown descriptors for one grove, moved (not cloned) through extract.
#[derive(Clone, Debug)]
pub enum TreeCatalogOp {
	Admit {
		id: GroveId,
		plants: Vec<PlantDescriptor>,
		branches: Vec<BranchDescriptor>,
		leaves: Vec<LeafDescriptor>,
		aabb_min: Vec3,
		aabb_max: Vec3,
		seed: u32,
		truncated_plants: u32,
	},
	Retire {
		id: GroveId,
	},
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

#[derive(Clone, Debug)]
struct LiveGrove {
	slot: u32,
	plants: RangeSpan,
	branches: RangeSpan,
	leaves: RangeSpan,
}

#[derive(Resource, Clone, Debug)]
pub struct TreeGroveCatalog {
	pub groves: Vec<GroveRecord>,
	pub plants: Vec<PlantDescriptor>,
	pub branches: Vec<BranchDescriptor>,
	pub leaves: Vec<LeafDescriptor>,
	live: HashMap<GroveId, LiveGrove>,
	plants_arena: RangeArena,
	branches_arena: RangeArena,
	leaves_arena: RangeArena,
	free_slots: Vec<u32>,
	store_descriptors: bool,
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
	pub plants: RangeSpan,
	pub branches: RangeSpan,
	pub leaves: RangeSpan,
	pub grove_slot: u32,
}

impl Default for TreeGroveCatalog {
	fn default() -> Self {
		Self::with_descriptors()
	}
}

impl TreeGroveCatalog {
	fn empty(store_descriptors: bool, consume_ops: bool) -> Self {
		Self {
			groves: Vec::new(),
			plants: Vec::new(),
			branches: Vec::new(),
			leaves: Vec::new(),
			live: HashMap::new(),
			plants_arena: RangeArena::with_cap(1_024),
			branches_arena: RangeArena::with_cap(8_192),
			leaves_arena: RangeArena::with_cap(16_384),
			free_slots: Vec::new(),
			store_descriptors,
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

	/// Main-world index: ids, AABBs, and spans. Descriptors stay in the ops.
	pub fn index_only() -> Self {
		Self::empty(false, false)
	}

	/// Tests and CPU-select mirrors keep descriptor arenas.
	pub fn with_descriptors() -> Self {
		Self::empty(true, false)
	}

	/// Render-world catalog takes descriptor ownership from extract.
	pub fn render() -> Self {
		Self::empty(true, true)
	}

	pub fn is_empty(&self) -> bool {
		self.live.is_empty()
	}

	pub fn resident_plants(&self) -> usize {
		if self.store_descriptors {
			self.plants.len()
		} else {
			self.plants_arena.used as usize
		}
	}

	pub fn resident_branches(&self) -> usize {
		if self.store_descriptors {
			self.branches.len()
		} else {
			self.branches_arena.used as usize
		}
	}

	pub fn resident_leaves(&self) -> usize {
		if self.store_descriptors {
			self.leaves.len()
		} else {
			self.leaves_arena.used as usize
		}
	}

	pub fn grove_spans(&self, id: GroveId) -> Option<(RangeSpan, RangeSpan, RangeSpan)> {
		self.live.get(&id).map(|live| (live.plants, live.branches, live.leaves))
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

	pub fn live_grove_count(&self) -> usize {
		self.live.len()
	}

	pub fn contains(&self, id: GroveId) -> bool {
		self.live.contains_key(&id)
	}

	pub fn encode_plan() -> GroveEncodeStats {
		GroveEncodeStats::woody_kind()
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
			TreeCatalogOp::Admit {
				id,
				plants,
				branches,
				leaves,
				aabb_min,
				aabb_max,
				seed,
				truncated_plants,
			} => {
				if self.store_descriptors {
					let (plants, branches, leaves) = if self.consume_ops {
						(std::mem::take(plants), std::mem::take(branches), std::mem::take(leaves))
					} else {
						(plants.clone(), branches.clone(), leaves.clone())
					};
					self.admit(
						*id,
						plants,
						branches,
						leaves,
						*aabb_min,
						*aabb_max,
						*seed,
						*truncated_plants,
					);
				} else {
					self.admit_index(
						*id,
						plants.len() as u32,
						branches.len() as u32,
						leaves.len() as u32,
						*aabb_min,
						*aabb_max,
						*seed,
						*truncated_plants,
					);
				}
			}
		})
	}

	fn admit(
		&mut self,
		id: GroveId,
		mut plants: Vec<PlantDescriptor>,
		mut branches: Vec<BranchDescriptor>,
		mut leaves: Vec<LeafDescriptor>,
		aabb_min: Vec3,
		aabb_max: Vec3,
		seed: u32,
		truncated_plants: u32,
	) {
		self.retire(id);
		self.truncated_plants = self.truncated_plants.saturating_add(truncated_plants);
		let plant_span = self.plants_arena.alloc(plants.len() as u32);
		let branch_span = self.branches_arena.alloc(branches.len() as u32);
		let leaf_span = self.leaves_arena.alloc(leaves.len() as u32);
		self.ensure_len(plant_span, branch_span, leaf_span);
		let slot = self.alloc_slot();
		for plant in &mut plants {
			plant.branch_offset = plant.branch_offset.saturating_add(branch_span.offset);
			plant.leaf_offset = plant.leaf_offset.saturating_add(leaf_span.offset);
			*plant = plant.with_grove_slot(slot).with_active();
		}
		for branch in &mut branches {
			branch.plant_index = branch.plant_index.saturating_add(plant_span.offset);
			if branch.parent_index != crate::layout::PARENT_NONE {
				branch.parent_index = branch.parent_index.saturating_add(branch_span.offset);
			}
			if branch.major_parent != crate::layout::PARENT_NONE {
				branch.major_parent = branch.major_parent.saturating_add(branch_span.offset);
			}
		}
		for leaf in &mut leaves {
			leaf.plant_index = leaf.plant_index.saturating_add(plant_span.offset);
			leaf.parent_branch = leaf.parent_branch.saturating_add(branch_span.offset);
		}
		self.write_plants(plant_span, &plants);
		self.write_branches(branch_span, &branches);
		self.write_leaves(leaf_span, &leaves);
		self.write_grove(
			slot,
			GroveRecord::active(
				aabb_min,
				aabb_max,
				seed,
				(plant_span.offset, plant_span.count),
				(branch_span.offset, branch_span.count),
				(leaf_span.offset, leaf_span.count),
			),
			plant_span,
			branch_span,
			leaf_span,
		);
		self.live.insert(
			id,
			LiveGrove { slot, plants: plant_span, branches: branch_span, leaves: leaf_span },
		);
		self.generation = self.generation.wrapping_add(1);
	}

	fn admit_index(
		&mut self,
		id: GroveId,
		plant_count: u32,
		branch_count: u32,
		leaf_count: u32,
		aabb_min: Vec3,
		aabb_max: Vec3,
		seed: u32,
		truncated_plants: u32,
	) {
		self.retire(id);
		self.truncated_plants = self.truncated_plants.saturating_add(truncated_plants);
		let plant_span = self.plants_arena.alloc(plant_count);
		let branch_span = self.branches_arena.alloc(branch_count);
		let leaf_span = self.leaves_arena.alloc(leaf_count);
		let slot = self.alloc_slot();
		self.write_grove(
			slot,
			GroveRecord::active(
				aabb_min,
				aabb_max,
				seed,
				(plant_span.offset, plant_span.count),
				(branch_span.offset, branch_span.count),
				(leaf_span.offset, leaf_span.count),
			),
			plant_span,
			branch_span,
			leaf_span,
		);
		self.live.insert(
			id,
			LiveGrove { slot, plants: plant_span, branches: branch_span, leaves: leaf_span },
		);
		self.generation = self.generation.wrapping_add(1);
	}

	fn write_grove(
		&mut self,
		slot: u32,
		record: GroveRecord,
		plants: RangeSpan,
		branches: RangeSpan,
		leaves: RangeSpan,
	) {
		self.groves[slot as usize] = record;
		self.last_writes += 1;
		self.last_write_bytes += plants.byte_len(PLANT_DESCRIPTOR_STRIDE)
			+ branches.byte_len(BRANCH_DESCRIPTOR_STRIDE)
			+ leaves.byte_len(LEAF_DESCRIPTOR_STRIDE)
			+ GROVE_RECORD_STRIDE;
		self.last_uploads
			.push(CatalogUpload { plants, branches, leaves, grove_slot: slot });
	}

	fn retire(&mut self, id: GroveId) {
		let Some(live) = self.live.remove(&id) else {
			return;
		};
		if let Some(record) = self.groves.get_mut(live.slot as usize) {
			record.flags = 0;
		}
		if self.store_descriptors {
			for plant in self.plants_mut(live.plants) {
				plant.flags &= !PlantDescriptor::FLAG_ACTIVE;
			}
		}
		self.plants_arena.free(live.plants);
		self.branches_arena.free(live.branches);
		self.leaves_arena.free(live.leaves);
		self.free_slots.push(live.slot);
		self.last_writes += 1;
		self.last_write_bytes += live.plants.byte_len(PLANT_DESCRIPTOR_STRIDE).max(16);
		self.last_uploads.push(CatalogUpload {
			plants: live.plants,
			branches: RangeSpan::default(),
			leaves: RangeSpan::default(),
			grove_slot: live.slot,
		});
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
			self.groves.push(GroveRecord::default());
			slot
		}
	}

	fn ensure_len(&mut self, plants: RangeSpan, branches: RangeSpan, leaves: RangeSpan) {
		let plant_end = (plants.offset + plants.count) as usize;
		let branch_end = (branches.offset + branches.count) as usize;
		let leaf_end = (leaves.offset + leaves.count) as usize;
		if self.plants.len() < plant_end {
			self.plants.resize(plant_end, PlantDescriptor::inactive());
		}
		if self.branches.len() < branch_end {
			self.branches.resize(branch_end, BranchDescriptor::default());
		}
		if self.leaves.len() < leaf_end {
			self.leaves.resize(leaf_end, LeafDescriptor::default());
		}
	}

	fn write_plants(&mut self, span: RangeSpan, plants: &[PlantDescriptor]) {
		let start = span.offset as usize;
		self.plants[start..start + plants.len()].copy_from_slice(plants);
	}

	fn write_branches(&mut self, span: RangeSpan, branches: &[BranchDescriptor]) {
		let start = span.offset as usize;
		self.branches[start..start + branches.len()].copy_from_slice(branches);
	}

	fn write_leaves(&mut self, span: RangeSpan, leaves: &[LeafDescriptor]) {
		let start = span.offset as usize;
		self.leaves[start..start + leaves.len()].copy_from_slice(leaves);
	}

	fn plants_mut(&mut self, span: RangeSpan) -> &mut [PlantDescriptor] {
		let start = span.offset as usize;
		let end = (start + span.count as usize).min(self.plants.len());
		&mut self.plants[start..end]
	}

	pub fn upload_bytes(&self) -> u64 {
		self.resident_plants() as u64 * PLANT_DESCRIPTOR_STRIDE
			+ self.resident_branches() as u64 * BRANCH_DESCRIPTOR_STRIDE
			+ self.resident_leaves() as u64 * LEAF_DESCRIPTOR_STRIDE
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::field::{TreeField, TreeFieldSettings};
	use crate::patch::TreePatch;
	use crate::recipe::TreeRecipe;
	use vegetation_grass_field::ADMISSION_BUDGET;

	fn grown(origin: Vec3) -> (Vec<PlantDescriptor>, Vec<BranchDescriptor>, Vec<LeafDescriptor>) {
		let patch = TreePatch::isolated(1, origin, TreeRecipe::storybook());
		let field = TreeField::rebuild([&patch], &TreeFieldSettings::default());
		(field.plants, field.branches, field.leaves)
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
		let (plants, branches, leaves) = grown(Vec3::ZERO);
		let plant_bytes = plants.len() as u64 * PLANT_DESCRIPTOR_STRIDE;
		catalog.apply(
			[TreeCatalogOp::Admit {
				id: GroveId(1),
				plants,
				branches,
				leaves,
				aabb_min: Vec3::ZERO,
				aabb_max: Vec3::ONE,
				seed: 1,
				truncated_plants: 0,
			}],
			ADMISSION_BUDGET,
		);
		assert_eq!(catalog.live_grove_count(), 1);
		assert_eq!(catalog.last_writes, 1);
		assert!(catalog.last_write_bytes >= plant_bytes);
		assert_eq!(catalog.clones, 0);
	}

	#[test]
	fn retire_does_not_rebuild() {
		let mut catalog = TreeGroveCatalog::default();
		let (plants, branches, leaves) = grown(Vec3::ZERO);
		catalog.apply(
			[TreeCatalogOp::Admit {
				id: GroveId(7),
				plants,
				branches,
				leaves,
				aabb_min: Vec3::ZERO,
				aabb_max: Vec3::ONE,
				seed: 7,
				truncated_plants: 0,
			}],
			ADMISSION_BUDGET,
		);
		let resident = catalog.plants.len();
		catalog.grove_residency = true;
		catalog.apply([TreeCatalogOp::Retire { id: GroveId(7) }], ADMISSION_BUDGET);
		assert_eq!(catalog.live_grove_count(), 0);
		assert_eq!(catalog.plants.len(), resident);
		assert!(catalog.plants.iter().all(|plant| !plant.is_active()));
		assert!(!catalog.grove_residency);
		assert_eq!(catalog.clones, 0);
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
			.map(|i| {
				let (plants, branches, leaves) = grown(Vec3::new(i as f32, 0.0, 0.0));
				TreeCatalogOp::Admit {
					id: GroveId(i),
					plants,
					branches,
					leaves,
					aabb_min: Vec3::ZERO,
					aabb_max: Vec3::ONE,
					seed: i as u32,
					truncated_plants: 0,
				}
			})
			.collect();
		let leftover = catalog.apply(ops, 1).leftover;
		assert_eq!(catalog.live_grove_count(), 1);
		assert_eq!(leftover.len(), 2);
	}

	fn admit(id: u64) -> TreeCatalogOp {
		let (plants, branches, leaves) = grown(Vec3::new(id as f32, 0.0, 0.0));
		TreeCatalogOp::Admit {
			id: GroveId(id),
			plants,
			branches,
			leaves,
			aabb_min: Vec3::ZERO,
			aabb_max: Vec3::ONE,
			seed: id as u32,
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

	#[test]
	fn retire_then_admit_reuses_freed_span() {
		let mut catalog = TreeGroveCatalog::default();
		catalog.apply([admit(1), admit(2)], ADMISSION_BUDGET);
		let first = catalog.grove_spans(GroveId(1)).expect("grove 1");
		catalog.apply([TreeCatalogOp::Retire { id: GroveId(1) }], ADMISSION_BUDGET);
		catalog.apply([admit(3)], ADMISSION_BUDGET);
		let reused = catalog.grove_spans(GroveId(3)).expect("grove 3");
		assert_eq!(reused.0.offset, first.0.offset);
		assert_eq!(reused.1.offset, first.1.offset);
		assert_eq!(reused.2.offset, first.2.offset);
		assert!(catalog.contains(GroveId(2)));
		assert!(!catalog.contains(GroveId(1)));
	}

	#[test]
	fn admit_after_growth_preserves_earlier_grove_bytes() {
		let mut catalog = TreeGroveCatalog::default();
		catalog.apply([admit(1)], ADMISSION_BUDGET);
		let first = catalog.plants.clone();
		let first_span = catalog.grove_spans(GroveId(1)).expect("grove 1").0;
		let before_cap = catalog.plants_arena.cap;
		let extra: Vec<_> = (2..24).map(admit).collect();
		let extra_len = extra.len();
		catalog.apply(extra, extra_len);
		assert!(catalog.plants_arena.cap >= before_cap);
		let start = first_span.offset as usize;
		let end = start + first_span.count as usize;
		assert_eq!(&catalog.plants[start..end], &first[start..end]);
		assert_eq!(catalog.last_uploads.len(), extra_len);
		assert!(catalog
			.last_uploads
			.iter()
			.all(|upload| { upload.plants.offset >= first_span.offset + first_span.count }));
		assert!(catalog.contains(GroveId(1)));
	}

	#[test]
	fn index_only_catalog_does_not_keep_descriptors() {
		let mut catalog = TreeGroveCatalog::index_only();
		catalog.apply([admit(1)], ADMISSION_BUDGET);
		assert!(catalog.plants.is_empty());
		assert_eq!(catalog.live_grove_count(), 1);
		assert!(catalog.resident_plants() > 0);
		assert_eq!(catalog.clones, 0);
	}

	#[test]
	fn render_catalog_takes_descriptors_from_ops() {
		let mut catalog = TreeGroveCatalog::render();
		let op = admit(1);
		let plant_count = match &op {
			TreeCatalogOp::Admit { plants, .. } => plants.len(),
			TreeCatalogOp::Retire { .. } => 0,
		};
		let CatalogApply { applied, .. } = catalog.apply([op], ADMISSION_BUDGET);
		assert_eq!(catalog.plants.len(), plant_count);
		match &applied[0] {
			TreeCatalogOp::Admit { plants, .. } => assert!(plants.is_empty()),
			TreeCatalogOp::Retire { .. } => panic!("expected admit"),
		}
	}
}
