//! One CPU catalog of grass groves. Prepare uploads only admitted blade ranges.

use std::collections::HashMap;

use bevy::math::Vec3;
use bevy::prelude::*;
use vegetation_shaders::wesl::{BLADE_DESCRIPTOR_STRIDE, BladeDescriptor, GrassPatchHeader};

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
		Self {
			patches: Vec::new(),
			blades: Vec::new(),
			live: HashMap::new(),
			blades_arena: RangeArena::with_cap(8_192),
			free_slots: Vec::new(),
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
}

impl GrassGroveCatalog {
	pub fn is_empty(&self) -> bool {
		self.live.is_empty()
	}

	pub fn live_grove_count(&self) -> usize {
		self.live.len()
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
		CatalogApply::budgeted(ops, budget, |op| match op {
			GrassCatalogOp::Retire { id } => self.retire(*id),
			GrassCatalogOp::Admit {
				id,
				blades,
				aabb_min,
				aabb_max,
				seed,
				truncated_plants,
				grove_residency,
			} => self.admit(
				*id,
				blades.clone(),
				*aabb_min,
				*aabb_max,
				*seed,
				*truncated_plants,
				*grove_residency,
			),
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
		self.last_write_bytes += span.byte_len(BLADE_DESCRIPTOR_STRIDE) + 48;
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
		for blade in self.blades_mut(live.blades) {
			blade.height_m = 0.0;
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
		assert!(
			applied
				.iter()
				.all(|op| !(matches!(op, GrassCatalogOp::Admit { id, .. } if *id == GroveId(100))))
		);
		assert!(!main.contains(GroveId(100)));

		let mut render = GrassGroveCatalog::default();
		let CatalogApply { leftover: render_leftover, .. } =
			render.apply(applied, ADMISSION_BUDGET);
		assert!(render_leftover.is_empty());
		assert_same_live(&main, &render, (1..=ADMISSION_BUDGET as u64).chain([100]));
	}
}
