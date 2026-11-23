//! One CPU catalog of grove instances. Prepare uploads only admitted ranges.

use std::collections::HashMap;

use bevy::math::Vec3;
use bevy::prelude::*;
use vegetation_grass_field::{
	CatalogApply, CatalogOpQueue, GroveCatalogOp, GroveEncodeStats, GroveId, RangeArena, RangeSpan,
	WoodyGroveRecord, WOODY_GROVE_RECORD_STRIDE,
};

use crate::layout::{
	BRANCH_DESCRIPTOR_STRIDE, BranchDescriptor, LEAF_DESCRIPTOR_STRIDE, LeafDescriptor,
	PLANT_DESCRIPTOR_STRIDE, PlantDescriptor,
};

/// Grown descriptors for one grove, moved (not cloned) through extract.
#[derive(Clone, Debug)]
pub enum BushCatalogOp {
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

pub type BushCatalogOps = CatalogOpQueue<BushCatalogOp>;

impl GroveCatalogOp for BushCatalogOp {
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
pub struct BushGroveCatalog {
	pub groves: Vec<WoodyGroveRecord>,
	pub plants: Vec<PlantDescriptor>,
	pub branches: Vec<BranchDescriptor>,
	pub leaves: Vec<LeafDescriptor>,
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
	pub pending: Vec<BushCatalogOp>,
	pub last_uploads: Vec<CatalogUpload>,
	pub grove_residency: bool,
}

/// Descriptor ranges written this apply (admit or retire). Prepare uploads only these.
#[derive(Clone, Copy, Debug)]
pub struct CatalogUpload {
	pub plants: RangeSpan,
	pub branches: RangeSpan,
	pub leaves: RangeSpan,
}

impl Default for BushGroveCatalog {
	fn default() -> Self {
		Self::with_descriptors()
	}
}

impl BushGroveCatalog {
	fn empty(consume_ops: bool) -> Self {
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

	pub fn with_descriptors() -> Self {
		Self::empty(false)
	}

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
		self.plants.len()
	}

	pub fn resident_branches(&self) -> usize {
		self.branches.len()
	}

	pub fn resident_leaves(&self) -> usize {
		self.leaves.len()
	}

	pub fn contains(&self, id: GroveId) -> bool {
		self.live.contains_key(&id)
	}

	pub fn live_records(&self) -> impl Iterator<Item = (GroveId, WoodyGroveRecord)> + '_ {
		self.live.iter().filter_map(|(&id, live)| {
			self.groves.get(live.slot as usize).copied().map(|record| (id, record))
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

	pub fn apply(
		&mut self,
		ops: impl IntoIterator<Item = BushCatalogOp>,
		budget: usize,
	) -> CatalogApply<BushCatalogOp> {
		self.last_write_bytes = 0;
		self.last_writes = 0;
		self.last_uploads.clear();
		CatalogApply::budgeted_mut(ops, budget, |op| match op {
			BushCatalogOp::Retire { id } => self.retire(*id),
			BushCatalogOp::Admit {
				id,
				plants,
				branches,
				leaves,
				aabb_min,
				aabb_max,
				seed,
				truncated_plants,
			} => {
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
		self.write_record(
			slot,
			aabb_min,
			aabb_max,
			seed,
			plant_span,
			branch_span,
			leaf_span,
		);
		self.last_writes += 1;
		self.last_write_bytes += plant_span.byte_len(PLANT_DESCRIPTOR_STRIDE)
			+ branch_span.byte_len(BRANCH_DESCRIPTOR_STRIDE)
			+ leaf_span.byte_len(LEAF_DESCRIPTOR_STRIDE)
			+ WOODY_GROVE_RECORD_STRIDE;
		self.last_uploads.push(CatalogUpload {
			plants: plant_span,
			branches: branch_span,
			leaves: leaf_span,
		});
		self.live.insert(
			id,
			LiveGrove { slot, plants: plant_span, branches: branch_span, leaves: leaf_span },
		);
		self.generation = self.generation.wrapping_add(1);
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
		};
	}

	fn retire(&mut self, id: GroveId) {
		let Some(live) = self.live.remove(&id) else {
			return;
		};
		if let Some(record) = self.groves.get_mut(live.slot as usize) {
			record.flags = 0;
		}
		for plant in self.plants_mut(live.plants) {
			plant.flags &= !PlantDescriptor::FLAG_ACTIVE;
		}
		self.last_uploads.push(CatalogUpload {
			plants: live.plants,
			branches: RangeSpan::default(),
			leaves: RangeSpan::default(),
		});
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
		self.plants.len() as u64 * PLANT_DESCRIPTOR_STRIDE
			+ self.branches.len() as u64 * BRANCH_DESCRIPTOR_STRIDE
			+ self.leaves.len() as u64 * LEAF_DESCRIPTOR_STRIDE
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::field::{BushField, BushFieldSettings};
	use crate::patch::BushPatch;
	use crate::recipe::BushRecipe;
	use vegetation_grass_field::ADMISSION_BUDGET;

	fn grown(origin: Vec3) -> (Vec<PlantDescriptor>, Vec<BranchDescriptor>, Vec<LeafDescriptor>) {
		let patch = BushPatch::isolated(1, origin, BushRecipe::small_scrub());
		let field = BushField::rebuild([&patch], &BushFieldSettings::default());
		(field.plants, field.branches, field.leaves)
	}

	#[test]
	fn idle_frame_writes_nothing() {
		let mut catalog = BushGroveCatalog::default();
		let leftover = catalog.apply([], ADMISSION_BUDGET).leftover;
		assert!(leftover.is_empty());
		assert_eq!(catalog.last_writes, 0);
		assert_eq!(catalog.last_write_bytes, 0);
		assert_eq!(catalog.clones, 0);
	}

	#[test]
	fn admit_one_grove_writes_only_that_range() {
		let mut catalog = BushGroveCatalog::default();
		let (plants, branches, leaves) = grown(Vec3::ZERO);
		let plant_bytes = plants.len() as u64 * PLANT_DESCRIPTOR_STRIDE;
		catalog.apply(
			[BushCatalogOp::Admit {
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
		let mut catalog = BushGroveCatalog::default();
		let (plants, branches, leaves) = grown(Vec3::ZERO);
		catalog.apply(
			[BushCatalogOp::Admit {
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
		catalog.apply([BushCatalogOp::Retire { id: GroveId(7) }], ADMISSION_BUDGET);
		assert_eq!(catalog.live_grove_count(), 0);
		assert_eq!(catalog.plants.len(), resident);
		assert!(catalog.plants.iter().all(|plant| !plant.is_active()));
		assert_eq!(catalog.clones, 0);
	}

	#[test]
	fn encode_plan_matches_for_ten_and_thousand_groves() {
		assert_eq!(
			BushGroveCatalog::encode_plan().encode_only(),
			GroveEncodeStats::woody_kind().encode_only()
		);
	}

	#[test]
	fn admission_budget_leaves_overflow() {
		let mut catalog = BushGroveCatalog::default();
		let ops: Vec<_> = (0..3)
			.map(|i| {
				let (plants, branches, leaves) = grown(Vec3::new(i as f32, 0.0, 0.0));
				BushCatalogOp::Admit {
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

	fn admit(id: u64) -> BushCatalogOp {
		let (plants, branches, leaves) = grown(Vec3::new(id as f32, 0.0, 0.0));
		BushCatalogOp::Admit {
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
		main: &BushGroveCatalog,
		render: &BushGroveCatalog,
		ids: impl IntoIterator<Item = u64>,
	) {
		for id in ids {
			assert_eq!(main.contains(GroveId(id)), render.contains(GroveId(id)), "grove {id}");
		}
	}

	#[test]
	fn mixed_batch_applies_retire_after_budgeted_admits() {
		let mut main = BushGroveCatalog::default();
		main.apply([admit(99)], ADMISSION_BUDGET);
		let mut render = BushGroveCatalog::default();
		render.apply([admit(99)], ADMISSION_BUDGET);
		assert!(main.contains(GroveId(99)));

		let mut batch: Vec<_> = (1..=ADMISSION_BUDGET as u64).map(admit).collect();
		batch.push(admit(100));
		batch.push(BushCatalogOp::Retire { id: GroveId(99) });
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
		let mut catalog = BushGroveCatalog::with_descriptors();
		catalog.apply([admit(1)], ADMISSION_BUDGET);
		let first_plants = catalog.plants.clone();
		let first_len = first_plants.len();
		assert!(first_len > 0);
		catalog.apply([admit(2)], ADMISSION_BUDGET);
		assert_eq!(&catalog.plants[..first_len], &first_plants[..]);
		assert!(catalog.plants.len() > first_len);
		assert_eq!(catalog.last_uploads.len(), 1);
		assert!(catalog.last_uploads[0].plants.offset >= first_len as u32);
	}

	#[test]
	fn mixed_batch_cancels_pending_admit_on_retire() {
		let mut main = BushGroveCatalog::default();
		let mut batch: Vec<_> = (1..=ADMISSION_BUDGET as u64).map(admit).collect();
		batch.push(admit(100));
		batch.push(BushCatalogOp::Retire { id: GroveId(100) });
		let CatalogApply { applied, leftover } = main.apply(batch, ADMISSION_BUDGET);

		assert!(leftover.is_empty());
		assert!(applied.iter().any(|op| op.is_retire() && op.grove_id() == GroveId(100)));
		assert!(
			applied
				.iter()
				.all(|op| !(matches!(op, BushCatalogOp::Admit { id, .. } if *id == GroveId(100))))
		);
		assert!(!main.contains(GroveId(100)));

		let mut render = BushGroveCatalog::default();
		let CatalogApply { leftover: render_leftover, .. } =
			render.apply(applied, ADMISSION_BUDGET);
		assert!(render_leftover.is_empty());
		assert_same_live(&main, &render, (1..=ADMISSION_BUDGET as u64).chain([100]));
	}
}
