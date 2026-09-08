//! High-band floor-color bump-outs on the Near terrain disk.
//!
//! [`GroundCoverBumpOut`] is a sibling of [`crate::CanopyBumpOut`], not a second
//! LOD on it. Origins are the same 160 m cells; present keep is the Near High
//! disk (inner hole = 0) so moss / trails / fields sit underfoot. Heights are
//! centimetres — never the canopy-metre tables.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Color, Vec3};
use lod::gen::Id;

use crate::bump_out::{
	blend_selection_on_bounds, bump_out_cell_bounds, bump_out_cells_overlapping,
	bump_out_chebyshev_xz, BumpOutSelectionSample, BUMP_OUT_CELL_XZ,
};
use crate::{ForestGroveKind, ForestIndex, LayeringKind};

/// Near High half-extent (8 × 160 m). Same Chebyshev family as playable Near terrain.
pub const GROUND_COVER_RADIUS_M: f32 = 8.0 * BUMP_OUT_CELL_XZ;

/// Far High starts this far inside the Near disk (one 320 m cell).
pub const GROUND_COVER_FAR_OVERLAP_M: f32 = 2.0 * BUMP_OUT_CELL_XZ;

/// Snap shared with Near / Far terrain streams (640 m).
pub const GROUND_COVER_ANCHOR_STEP_M: f32 = 4.0 * BUMP_OUT_CELL_XZ;

/// Floor-color recipe. Same shader as canopy; centimetre displace + paint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroundCoverKind {
	MossyBumps,
	GameTrail,
	Field,
}

/// One blended floor-color sample (empty when the cell should not paint).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundCoverSample {
	pub kind: Option<GroundCoverKind>,
	pub density: f32,
	pub bite_size: f32,
	pub bite_size_deviation: f32,
	pub height_m: f32,
	pub height_deviation_m: f32,
	pub palette: [Color; 3],
}

impl GroundCoverSample {
	pub fn empty() -> Self {
		Self {
			kind: None,
			density: 0.0,
			bite_size: 1.0,
			bite_size_deviation: 0.0,
			height_m: 0.0,
			height_deviation_m: 0.0,
			palette: moss_palette(),
		}
	}

	pub fn from_kind(kind: GroundCoverKind, variation: f32) -> Self {
		let t = variation.clamp(0.0, 1.0);
		match kind {
			GroundCoverKind::MossyBumps => Self {
				kind: Some(kind),
				density: 0.82,
				bite_size: 2.4,
				bite_size_deviation: 0.35,
				height_m: 0.04 + 0.14 * t,
				height_deviation_m: 0.06,
				palette: moss_palette(),
			},
			GroundCoverKind::GameTrail => Self {
				kind: Some(kind),
				density: 0.28,
				bite_size: 3.2,
				bite_size_deviation: 0.22,
				height_m: -0.02 + 0.02 * t,
				height_deviation_m: 0.02,
				palette: trail_palette(),
			},
			GroundCoverKind::Field => Self {
				kind: Some(kind),
				density: 0.90,
				bite_size: 14.0,
				bite_size_deviation: 0.18,
				height_m: 0.02 + 0.06 * t,
				height_deviation_m: 0.03,
				palette: field_palette(),
			},
		}
	}

	pub fn with_density(mut self, density: f32) -> Self {
		self.density = density.clamp(0.0, 1.0);
		self
	}
}

/// Select-only floor-color overlay sampled over one Near High terrain cell.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundCoverBumpOut {
	pub bounds: Aabb3d,
	pub samples: [GroundCoverSample; 9],
}

impl GroundCoverBumpOut {
	pub fn id(&self) -> Id {
		Id::from_cell(self.bounds)
	}

	pub fn has_density(&self) -> bool {
		self.samples.iter().any(|sample| sample.density > 0.001)
	}

	pub fn center_kind(&self) -> Option<GroundCoverKind> {
		self.samples[4].kind
	}

	pub fn center_palette(&self) -> [Color; 3] {
		self.samples[4].palette
	}
}

impl GroundCoverKind {
	/// Cheese / coverage / fragment-height preset for [`chico_bumpout::BumpOutStyle`].
	pub fn style_values(self) -> GroundCoverStyle {
		match self {
			Self::MossyBumps => GroundCoverStyle {
				coverage_softness: 0.05,
				roughness: 0.96,
				normal_soften: 0.40,
				cheese_amount: 0.92,
				cheese_scale: 1.45,
				fragment_height_frequency: 6.0,
				fragment_height_amplitude: 0.10,
				noise_frequency: 0.085,
				noise_octaves: 2,
			},
			Self::GameTrail => GroundCoverStyle {
				coverage_softness: 0.08,
				roughness: 0.98,
				normal_soften: 0.55,
				cheese_amount: 0.55,
				cheese_scale: 0.85,
				fragment_height_frequency: 3.5,
				fragment_height_amplitude: 0.04,
				noise_frequency: 0.055,
				noise_octaves: 2,
			},
			Self::Field => GroundCoverStyle {
				coverage_softness: 0.035,
				roughness: 0.94,
				normal_soften: 0.32,
				cheese_amount: 0.22,
				cheese_scale: 0.55,
				fragment_height_frequency: 2.2,
				fragment_height_amplitude: 0.05,
				noise_frequency: 0.018,
				noise_octaves: 2,
			},
		}
	}
}

/// Material-level knobs the presenter maps onto [`chico_bumpout::BumpOutStyle`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundCoverStyle {
	pub coverage_softness: f32,
	pub roughness: f32,
	pub normal_soften: f32,
	pub cheese_amount: f32,
	pub cheese_scale: f32,
	pub fragment_height_frequency: f32,
	pub fragment_height_amplitude: f32,
	pub noise_frequency: f32,
	pub noise_octaves: u32,
}

/// Whether a 160 m cell center lies in the Near High keep disk (inner hole = 0).
pub fn ground_cover_in_near_disk(bounds: Aabb3d, region: Aabb3d) -> bool {
	let origin =
		Vec3::new((region.min.x + region.max.x) * 0.5, 0.0, (region.min.z + region.max.z) * 0.5);
	bump_out_chebyshev_xz(bounds, origin) <= GROUND_COVER_RADIUS_M
}

/// Integer 160 m index of a cell AABB.
pub fn ground_cover_cell_index(bounds: Aabb3d) -> (i32, i32) {
	let s = BUMP_OUT_CELL_XZ;
	((bounds.min.x / s).floor() as i32, (bounds.min.z / s).floor() as i32)
}

/// 3×3 floor-color neighborhood centered on `bounds`.
pub fn blend_ground_cover_neighborhood(
	index: &ForestIndex,
	bounds: Aabb3d,
) -> [GroundCoverSample; 9] {
	let size = (bounds.max.x - bounds.min.x).max(BUMP_OUT_CELL_XZ);
	let mut samples = [GroundCoverSample::empty(); 9];
	for row in 0..3 {
		for column in 0..3 {
			let dx = (column as f32 - 1.0) * size;
			let dz = (row as f32 - 1.0) * size;
			let min = Vec3::new(bounds.min.x + dx, bounds.min.y, bounds.min.z + dz);
			let max = Vec3::new(bounds.max.x + dx, bounds.max.y, bounds.max.z + dz);
			samples[row * 3 + column] =
				ground_cover_sample_on_bounds(index, Aabb3d::from_min_max(min, max));
		}
	}
	samples
}

fn ground_cover_sample_on_bounds(index: &ForestIndex, bounds: Aabb3d) -> GroundCoverSample {
	let selection = blend_selection_on_bounds(index, bounds);
	let center = Vec3::from((bounds.min + bounds.max) * 0.5);
	let (forest_ix, forest_iz) = crate::ForestExtent::cell_index_containing(center);
	let layering = index
		.selected_layers_for(crate::ForestExtent::from_cell_index(forest_ix, forest_iz))
		.layering;
	let (ix, iz) = ground_cover_cell_index(bounds);
	let seed = index.noise.seed as u32;
	let Some(kind) = pick_ground_cover_kind(selection, layering, ix, iz, seed) else {
		return GroundCoverSample::empty();
	};
	let variation = cell_hash01(ix, iz, seed.wrapping_add(19));
	let mut sample = GroundCoverSample::from_kind(kind, variation);
	if kind == GroundCoverKind::GameTrail {
		sample = sample.with_density(0.18 + 0.22 * trail_weight(ix, iz, seed));
	}
	sample
}

fn pick_ground_cover_kind(
	selection: BumpOutSelectionSample,
	layering: LayeringKind,
	ix: i32,
	iz: i32,
	seed: u32,
) -> Option<GroundCoverKind> {
	if layering.prefers_empty_cover() {
		return None;
	}
	if trail_weight(ix, iz, seed) > 0.55 {
		return Some(GroundCoverKind::GameTrail);
	}
	if selection.kind.is_none() && selection.density <= 0.001 {
		return None;
	}
	if layering.prefers_field_cover() || field_cell(selection, ix, iz, seed) {
		return Some(GroundCoverKind::Field);
	}
	Some(GroundCoverKind::MossyBumps)
}

fn field_cell(selection: BumpOutSelectionSample, ix: i32, iz: i32, seed: u32) -> bool {
	if selection.kind.is_some_and(ForestGroveKind::is_tuft) {
		return true;
	}
	if meadow_like_kind(selection.kind) {
		return true;
	}
	// 5×5 correlation so fields span many 160 m cells without a hard square.
	let block = cell_hash01(ix.div_euclid(5), iz.div_euclid(5), seed.wrapping_add(41));
	block > 0.62 && selection.density > 0.04
}

fn meadow_like_kind(kind: Option<ForestGroveKind>) -> bool {
	kind.is_some_and(|kind| {
		kind.is_tuft()
			|| matches!(
				kind,
				ForestGroveKind::BraidGrass | ForestGroveKind::Vineyard | ForestGroveKind::Orchard
			)
	})
}

/// Sparse 1-D hash ridges through the 160 m lattice (v1 trails; not jersey paths).
fn trail_weight(ix: i32, iz: i32, seed: u32) -> f32 {
	let ns = ridge_distance(ix, iz, seed, 11);
	let ew = ridge_distance(iz, ix, seed.wrapping_add(91), 13);
	(1.0 - ns.min(ew)).clamp(0.0, 1.0)
}

fn ridge_distance(along: i32, across: i32, seed: u32, period: i32) -> f32 {
	let band = along.div_euclid(period);
	let wander = (cell_hash01(band, 0, seed) * 6.0 - 3.0).round() as i32;
	let center = band * 2 + wander;
	((across - center).abs() as f32 / 1.35).min(1.0)
}

fn cell_hash01(ix: i32, iz: i32, salt: u32) -> f32 {
	let h = (ix as u32)
		.wrapping_mul(0x9e3779b9)
		.wrapping_add((iz as u32).wrapping_mul(0x85ebca77))
		.wrapping_add(salt.wrapping_mul(0xc2b2ae35));
	(h as f32) / (u32::MAX as f32)
}

fn moss_palette() -> [Color; 3] {
	[rgb(0.06, 0.18, 0.08), rgb(0.10, 0.32, 0.12), rgb(0.18, 0.42, 0.16)]
}

fn trail_palette() -> [Color; 3] {
	[rgb(0.28, 0.20, 0.10), rgb(0.42, 0.32, 0.14), rgb(0.58, 0.48, 0.22)]
}

fn field_palette() -> [Color; 3] {
	[rgb(0.20, 0.38, 0.10), rgb(0.36, 0.52, 0.14), rgb(0.58, 0.62, 0.22)]
}

fn rgb(r: f32, g: f32, b: f32) -> Color {
	Color::srgb(r, g, b)
}

/// World-aligned 160 m cells overlapping `region` that sit in the Near disk.
pub fn ground_cover_cells_in_near_disk(region: Aabb3d) -> impl Iterator<Item = (i32, i32)> {
	bump_out_cells_overlapping(region)
		.filter(move |(ix, iz)| ground_cover_in_near_disk(bump_out_cell_bounds(*ix, *iz), region))
}

/// Layering hint used when a presenter / test pins a forest cell.
pub fn kind_from_layering(
	layering: LayeringKind,
	ix: i32,
	iz: i32,
	seed: u32,
) -> Option<GroundCoverKind> {
	if layering.prefers_empty_cover() {
		return None;
	}
	if trail_weight(ix, iz, seed) > 0.55 {
		return Some(GroundCoverKind::GameTrail);
	}
	if layering.prefers_field_cover() {
		return Some(GroundCoverKind::Field);
	}
	Some(GroundCoverKind::MossyBumps)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn recipes_use_centimetre_heights() -> Result<()> {
		let moss = GroundCoverSample::from_kind(GroundCoverKind::MossyBumps, 0.5);
		assert!(moss.height_m > 0.03 && moss.height_m < 0.20);
		assert!(moss.height_deviation_m < 0.10);
		assert!(moss.density > 0.7);

		let trail = GroundCoverSample::from_kind(GroundCoverKind::GameTrail, 0.5);
		assert!(trail.height_m <= 0.0);
		assert!(trail.height_m >= -0.03);
		assert!(trail.density < 0.4);

		let field = GroundCoverSample::from_kind(GroundCoverKind::Field, 0.5);
		assert!(field.height_m > 0.01 && field.height_m < 0.09);
		assert!(field.bite_size > 8.0);
		assert!(field.density > 0.8);
		Ok(())
	}

	#[test]
	fn empty_sample_has_no_density() -> Result<()> {
		let empty = GroundCoverSample::empty();
		assert!(empty.kind.is_none());
		assert!(empty.density <= 0.001);
		Ok(())
	}

	#[test]
	fn barren_layering_skips_cover() -> Result<()> {
		assert!(kind_from_layering(LayeringKind::SunsBarren, 0, 0, 1).is_none());
		assert!(kind_from_layering(LayeringKind::OwlsDesert, 3, 4, 1).is_none());
		Ok(())
	}

	#[test]
	fn meadow_layering_picks_field() -> Result<()> {
		let mut found_field = false;
		for iz in 0..24 {
			for ix in 0..24 {
				if kind_from_layering(LayeringKind::Meadowland, ix, iz, 7)
					== Some(GroundCoverKind::Field)
				{
					found_field = true;
				}
			}
		}
		assert!(found_field);
		Ok(())
	}

	#[test]
	fn forest_layering_picks_moss_off_trail() -> Result<()> {
		let mut found_moss = false;
		for iz in 0..24 {
			for ix in 0..24 {
				if kind_from_layering(LayeringKind::LushJungle, ix, iz, 3)
					== Some(GroundCoverKind::MossyBumps)
				{
					found_moss = true;
				}
			}
		}
		assert!(found_moss);
		Ok(())
	}

	#[test]
	fn trail_ridge_marks_a_corridor() -> Result<()> {
		let seed = 11;
		let on_path: Vec<(i32, i32)> = (-16..16)
			.flat_map(|ix| (-16..16).map(move |iz| (ix, iz)))
			.filter(|(ix, iz)| trail_weight(*ix, *iz, seed) > 0.55)
			.collect();
		assert!(!on_path.is_empty(), "expected at least one trail cell");
		assert!(on_path.len() < 16 * 16 / 3, "trail should stay a thin corridor");
		Ok(())
	}

	#[test]
	fn near_disk_includes_the_origin_cell() -> Result<()> {
		let region = crate::ForestExtent::xz_radius_aabb(Vec3::ZERO, GROUND_COVER_RADIUS_M);
		let origin = bump_out_cell_bounds(0, 0);
		assert!(ground_cover_in_near_disk(origin, region));
		assert!(!ground_cover_in_near_disk(bump_out_cell_bounds(12, 0), region));
		Ok(())
	}

	#[test]
	fn field_style_uses_weak_cheese_and_low_frequency() -> Result<()> {
		let field = GroundCoverKind::Field.style_values();
		let moss = GroundCoverKind::MossyBumps.style_values();
		assert!(field.cheese_amount < 0.35);
		assert!(field.noise_frequency < moss.noise_frequency);
		assert!(moss.cheese_amount > 0.8);
		Ok(())
	}

	#[test]
	fn tuft_kind_is_classified() -> Result<()> {
		assert!(ForestGroveKind::WildGrass.is_tuft());
		assert!(ForestGroveKind::CommonTufts.is_tuft());
		assert!(!ForestGroveKind::RollingOaks.is_tuft());
		Ok(())
	}
}
