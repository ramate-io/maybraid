//! One CPU catalog of grass groves. Prepare uploads only admitted blade ranges.

use std::collections::HashMap;

use bevy::math::Vec3;
use bevy::prelude::*;
use vegetation_shaders::wesl::{
	BladeDescriptor, GrassPatchHeader, GroveRecord, BLADE_DESCRIPTOR_STRIDE, GROVE_RECORD_STRIDE,
};

use crate::catalog::{
	CatalogApply, CatalogOpQueue, GroveCatalogOp, GroveEncodeStats, GroveId, RangeArena, RangeSpan,
};

#[derive(Clone, Debug)]
pub enum GrassCatalogOp {
	Admit {
		id: GroveId,
		blades: Vec<BladeDescriptor>,
		aabb_min: Vec3,
		aabb_max: Vec3,
		seed: u32,
		truncated_plants: u32,
		grove_residency: bool,
	},
	Retire {
		id: GroveId,
	},
}

pub type GrassCatalogOps = CatalogOpQueue<GrassCatalogOp>;

impl GroveCatalogOp for GrassCatalogOp {
	fn grove_id(&self) -> GroveId {
		match *self {
			Self::Admit { id, .. } | Self::Retire { id } => id,
		}
	}

	fn is_retire(&self) -> bool {
		matches!(self, Self::Retire { .. })
	}
}

#[derive(Clone, Copy, Debug)]
pub struct CatalogUpload {
	pub blades: RangeSpan,
	pub patch_slot: u32,
}

#[derive(Clone, Debug)]
struct LiveGrove {
	slot: u32,
	blades: RangeSpan,
}

#[derive(Resource, Clone, Debug)]
pub struct GrassGroveCatalog {
	pub patches: Vec<GrassPatchHeader>,
	pub blades: Vec<BladeDescriptor>,
	live: HashMap<GroveId, LiveGrove>,
	blades_arena: RangeArena,
	free_slots: Vec<u32>,
	store_descriptors: bool,
	consume_ops: bool,
	pub generation: u64,
	pub last_write_bytes: u64,
	pub last_writes: u32,
	pub truncated_plants: u32,
	pub clones: u32,
	pub pending: Vec<GrassCatalogOp>,
	pub last_uploads: Vec<CatalogUpload>,
	pub grove_residency: bool,
}

impl Default for GrassGroveCatalog {
	fn default() -> Self {
		Self::with_descriptors()
	}
}

impl GrassGroveCatalog {
	fn empty(store_descriptors: bool, consume_ops: bool) -> Self {
		Self {
			patches: Vec::new(),
			blades: Vec::new(),
			live: HashMap::new(),
			blades_arena: RangeArena::with_cap(8_192),
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

	pub fn index_only() -> Self {
		Self::empty(false, false)
	}

	pub fn with_descriptors() -> Self {
		Self::empty(true, false)
	}

	pub fn render() -> Self {
		Self::empty(true, true)
	}
}

impl GrassGroveCatalog {
	pub fn is_empty(&self) -> bool {
		self.live.is_empty()
	}

	pub fn live_grove_count(&self) -> usize {
		self.live.len()
	}

	pub fn resident_blades(&self) -> usize {
		if self.store_descriptors {
			self.blades.len()
		} else {
			self.blades_arena.used as usize
		}
	}

	pub fn grove_spans(&self, id: GroveId) -> Option<RangeSpan> {
		self.live.get(&id).map(|live| live.blades)
	}

	pub fn grove_records(&self) -> Vec<GroveRecord> {
		self.patches.iter().copied().map(GrassPatchHeader::grove_record).collect()
	}

	pub fn active_bounds(&self) -> Option<(Vec3, Vec3)> {
		let mut min = Vec3::splat(f32::MAX);
		let mut max = Vec3::splat(f32::MIN);
		let mut any = false;
		for header in self.patches.iter().filter(|header| header.is_active()) {
			min = min.min(header.aabb_min);
			max = max.max(header.aabb_max);
			any = true;
		}
		any.then_some((min, max))
	}

	pub fn contains(&self, id: GroveId) -> bool {
		self.live.contains_key(&id)
	}

	pub fn encode_plan() -> GroveEncodeStats {
		GroveEncodeStats::grass_kind()
	}

	pub fn apply(
		&mut self,
		ops: impl IntoIterator<Item = GrassCatalogOp>,
		budget: usize,
	) -> CatalogApply<GrassCatalogOp> {
		self.last_write_bytes = 0;
		self.last_writes = 0;
		self.last_uploads.clear();
		CatalogApply::budgeted_mut(ops, budget, |op| match op {
			GrassCatalogOp::Retire { id } => self.retire(*id),
			GrassCatalogOp::Admit {
				id,
				blades,
				aabb_min,
				aabb_max,
				seed,
				truncated_plants,
				grove_residency,
			} => {
				if self.store_descriptors {
					let blades =
						if self.consume_ops { std::mem::take(blades) } else { blades.clone() };
					self.admit(
						*id,
						blades,
						*aabb_min,
						*aabb_max,
						*seed,
						*truncated_plants,
						*grove_residency,
					);
				} else {
					self.admit_index(
						*id,
						blades.len() as u32,
						*aabb_min,
						*aabb_max,
						*seed,
						*truncated_plants,
						*grove_residency,
					);
				}
			}
		})
	}

	fn admit(
		&mut self,
		id: GroveId,
		blades: Vec<BladeDescriptor>,
		aabb_min: Vec3,
		aabb_max: Vec3,
		seed: u32,
		truncated_plants: u32,
		grove_residency: bool,
	) {
		self.retire(id);
		self.truncated_plants = self.truncated_plants.saturating_add(truncated_plants);
		self.grove_residency |= grove_residency;
		let span = self.blades_arena.alloc(blades.len() as u32);
		let end = (span.offset + span.count) as usize;
		if self.blades.len() < end {
			self.blades.resize(end, inactive_blade());
		}
		let slot = self.alloc_slot();
		let start = span.offset as usize;
		self.blades[start..start + blades.len()].copy_from_slice(&blades);
		self.write_header(id, slot, aabb_min, aabb_max, seed, span);
	}

	fn admit_index(
		&mut self,
		id: GroveId,
		blade_count: u32,
		aabb_min: Vec3,
		aabb_max: Vec3,
		seed: u32,
		truncated_plants: u32,
		grove_residency: bool,
	) {
		self.retire(id);
		self.truncated_plants = self.truncated_plants.saturating_add(truncated_plants);
		self.grove_residency |= grove_residency;
		let span = self.blades_arena.alloc(blade_count);
		let slot = self.alloc_slot();
		self.write_header(id, slot, aabb_min, aabb_max, seed, span);
	}

	fn write_header(
		&mut self,
		id: GroveId,
		slot: u32,
		aabb_min: Vec3,
		aabb_max: Vec3,
		seed: u32,
		span: RangeSpan,
	) {
		self.patches[slot as usize] = GrassPatchHeader {
			aabb_min,
			blade_offset: span.offset,
			aabb_max,
			blade_count: span.count,
			seed,
			coverage: 1.0,
			species: 0,
			flags: if span.count == 0 { 0 } else { GrassPatchHeader::FLAG_ACTIVE },
		};
		self.last_writes += 1;
		self.last_write_bytes += span.byte_len(BLADE_DESCRIPTOR_STRIDE) + GROVE_RECORD_STRIDE;
		self.last_uploads.push(CatalogUpload { blades: span, patch_slot: slot });
		self.live.insert(id, LiveGrove { slot, blades: span });
		self.generation = self.generation.wrapping_add(1);
	}

	fn retire(&mut self, id: GroveId) {
		let Some(live) = self.live.remove(&id) else {
			return;
		};
		if let Some(header) = self.patches.get_mut(live.slot as usize) {
			header.flags = 0;
		}
		if self.store_descriptors {
			for blade in self.blades_mut(live.blades) {
				blade.height_m = 0.0;
			}
		}
		self.blades_arena.free(live.blades);
		self.free_slots.push(live.slot);
		self.last_writes += 1;
		self.last_write_bytes += live.blades.byte_len(BLADE_DESCRIPTOR_STRIDE).max(16);
		self.last_uploads
			.push(CatalogUpload { blades: live.blades, patch_slot: live.slot });
		self.generation = self.generation.wrapping_add(1);
		if self.live.is_empty() {
			self.grove_residency = false;
		}
	}

	fn alloc_slot(&mut self) -> u32 {
		if let Some(slot) = self.free_slots.pop() {
			slot
		} else {
			let slot = self.patches.len() as u32;
			self.patches.push(GrassPatchHeader {
				aabb_min: Vec3::ZERO,
				blade_offset: 0,
				aabb_max: Vec3::ZERO,
				blade_count: 0,
				seed: 0,
				coverage: 0.0,
				species: 0,
				flags: 0,
			});
			slot
		}
	}

	fn blades_mut(&mut self, span: RangeSpan) -> &mut [BladeDescriptor] {
		let start = span.offset as usize;
		let end = (start + span.count as usize).min(self.blades.len());
		&mut self.blades[start..end]
	}
}

fn inactive_blade() -> BladeDescriptor {
	BladeDescriptor {
		root: Vec3::ZERO,
		height_m: 0.0,
		normal: Vec3::Y,
		width_m: 0.0,
		facing: Vec3::X,
		plant_id: 0,
		shape: Vec4::ZERO,
		palette: Vec4::ZERO,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::catalog::ADMISSION_BUDGET;
	use crate::field::{GrassField, GrassFieldSettings};
	use crate::patch::GrassPatch;

	fn grown(origin: Vec3) -> Vec<BladeDescriptor> {
		let patch = GrassPatch::isolated_clump(1, origin, 8);
		GrassField::rebuild([&patch], &GrassFieldSettings::default()).blades
	}

	#[test]
	fn idle_frame_writes_nothing() {
		let mut catalog = GrassGroveCatalog::default();
		let leftover = catalog.apply([], ADMISSION_BUDGET).leftover;
		assert!(leftover.is_empty());
		assert_eq!(catalog.last_writes, 0);
		assert_eq!(catalog.clones, 0);
	}

	#[test]
	fn admit_one_grove_writes_only_that_range() {
		let mut catalog = GrassGroveCatalog::default();
		let blades = grown(Vec3::ZERO);
		let bytes = blades.len() as u64 * BLADE_DESCRIPTOR_STRIDE;
		catalog.apply(
			[GrassCatalogOp::Admit {
				id: GroveId(1),
				blades,
				aabb_min: Vec3::ZERO,
				aabb_max: Vec3::ONE,
				seed: 1,
				truncated_plants: 0,
				grove_residency: true,
			}],
			ADMISSION_BUDGET,
		);
		assert_eq!(catalog.live_grove_count(), 1);
		assert_eq!(catalog.last_writes, 1);
		assert!(catalog.last_write_bytes >= bytes);
		assert!(catalog.grove_residency);
	}

	#[test]
	fn retire_does_not_rebuild() {
		let mut catalog = GrassGroveCatalog::default();
		catalog.apply(
			[GrassCatalogOp::Admit {
				id: GroveId(7),
				blades: grown(Vec3::ZERO),
				aabb_min: Vec3::ZERO,
				aabb_max: Vec3::ONE,
				seed: 7,
				truncated_plants: 0,
				grove_residency: true,
			}],
			ADMISSION_BUDGET,
		);
		let resident = catalog.blades.len();
		catalog.apply([GrassCatalogOp::Retire { id: GroveId(7) }], ADMISSION_BUDGET);
		assert_eq!(catalog.live_grove_count(), 0);
		assert_eq!(catalog.blades.len(), resident);
		assert!(catalog.blades.iter().all(|blade| blade.height_m <= 0.0));
		assert!(!catalog.grove_residency);
	}

	#[test]
	fn encode_plan_matches_for_ten_and_thousand_groves() {
		assert_eq!(
			GrassGroveCatalog::encode_plan().encode_only(),
			GroveEncodeStats::grass_kind().encode_only()
		);
	}

	#[test]
	fn retire_then_admit_reuses_freed_span() {
		let mut catalog = GrassGroveCatalog::default();
		catalog.apply([admit(1), admit(2)], ADMISSION_BUDGET);
		let first = catalog.grove_spans(GroveId(1)).expect("grove 1");
		catalog.apply([GrassCatalogOp::Retire { id: GroveId(1) }], ADMISSION_BUDGET);
		catalog.apply([admit(3)], ADMISSION_BUDGET);
		let reused = catalog.grove_spans(GroveId(3)).expect("grove 3");
		assert_eq!(reused.offset, first.offset);
		assert!(catalog.contains(GroveId(2)));
		assert!(!catalog.contains(GroveId(1)));
	}

	#[test]
	fn admit_after_growth_preserves_earlier_grove_bytes() {
		let mut catalog = GrassGroveCatalog::default();
		catalog.apply([admit(1)], ADMISSION_BUDGET);
		let first = catalog.blades.clone();
		let first_span = catalog.grove_spans(GroveId(1)).expect("grove 1");
		let extra: Vec<_> = (2..24).map(admit).collect();
		let extra_len = extra.len();
		catalog.apply(extra, extra_len);
		let start = first_span.offset as usize;
		let end = start + first_span.count as usize;
		assert_eq!(&catalog.blades[start..end], &first[start..end]);
		assert_eq!(catalog.last_uploads.len(), extra_len);
		assert!(catalog
			.last_uploads
			.iter()
			.all(|upload| { upload.blades.offset >= first_span.offset + first_span.count }));
		assert!(catalog.contains(GroveId(1)));
	}

	#[test]
	fn index_only_catalog_does_not_keep_descriptors() {
		let mut catalog = GrassGroveCatalog::index_only();
		catalog.apply([admit(1)], ADMISSION_BUDGET);
		assert!(catalog.blades.is_empty());
		assert_eq!(catalog.live_grove_count(), 1);
		assert!(catalog.resident_blades() > 0);
	}

	#[test]
	fn render_catalog_takes_descriptors_from_ops() {
		let mut catalog = GrassGroveCatalog::render();
		let op = admit(1);
		let blade_count = match &op {
			GrassCatalogOp::Admit { blades, .. } => blades.len(),
			GrassCatalogOp::Retire { .. } => 0,
		};
		let CatalogApply { applied, .. } = catalog.apply([op], ADMISSION_BUDGET);
		assert_eq!(catalog.blades.len(), blade_count);
		match &applied[0] {
			GrassCatalogOp::Admit { blades, .. } => assert!(blades.is_empty()),
			GrassCatalogOp::Retire { .. } => panic!("expected admit"),
		}
	}

	fn admit(id: u64) -> GrassCatalogOp {
		GrassCatalogOp::Admit {
			id: GroveId(id),
			blades: grown(Vec3::new(id as f32, 0.0, 0.0)),
			aabb_min: Vec3::ZERO,
			aabb_max: Vec3::ONE,
			seed: id as u32,
			truncated_plants: 0,
			grove_residency: true,
		}
	}

	fn assert_same_live(
		main: &GrassGroveCatalog,
		render: &GrassGroveCatalog,
		ids: impl IntoIterator<Item = u64>,
	) {
		for id in ids {
			assert_eq!(main.contains(GroveId(id)), render.contains(GroveId(id)), "grove {id}");
		}
	}

	#[test]
	fn mixed_batch_applies_retire_after_budgeted_admits() {
		let mut main = GrassGroveCatalog::default();
		main.apply([admit(99)], ADMISSION_BUDGET);
		let mut render = GrassGroveCatalog::default();
		render.apply([admit(99)], ADMISSION_BUDGET);
		assert!(main.contains(GroveId(99)));

		let mut batch: Vec<_> = (1..=ADMISSION_BUDGET as u64).map(admit).collect();
		batch.push(admit(100));
		batch.push(GrassCatalogOp::Retire { id: GroveId(99) });
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
		let mut main = GrassGroveCatalog::default();
		let mut batch: Vec<_> = (1..=ADMISSION_BUDGET as u64).map(admit).collect();
		batch.push(admit(100));
		batch.push(GrassCatalogOp::Retire { id: GroveId(100) });
		let CatalogApply { applied, leftover } = main.apply(batch, ADMISSION_BUDGET);

		assert!(leftover.is_empty());
		assert!(applied.iter().any(|op| op.is_retire() && op.grove_id() == GroveId(100)));
		assert!(applied
			.iter()
			.all(|op| !(matches!(op, GrassCatalogOp::Admit { id, .. } if *id == GroveId(100)))));
		assert!(!main.contains(GroveId(100)));

		let mut render = GrassGroveCatalog::default();
		let CatalogApply { leftover: render_leftover, .. } =
			render.apply(applied, ADMISSION_BUDGET);
		assert!(render_leftover.is_empty());
		assert_same_live(&main, &render, (1..=ADMISSION_BUDGET as u64).chain([100]));
	}
}
