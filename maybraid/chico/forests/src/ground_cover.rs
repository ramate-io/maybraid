//! High-band floor-color bump-outs on the Near terrain disk.
//!
//! [`GroundCoverBumpOut`] is a sibling of [`crate::CanopyBumpOut`], not a second
//! LOD on it. Origins are the same 160 m cells; present keep is the Near High
//! disk (inner hole = 0) so moss / fields sit underfoot. Heights are
//! centimetres — never the canopy-metre tables. Identity comes from the forest
//! cell's ground-cover flip, not from canopy or tuft selection.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Color, Vec3};
use lod::gen::Id;

use crate::bump_out::{
	bump_out_cell_bounds, bump_out_cells_overlapping, bump_out_chebyshev_xz, BUMP_OUT_CELL_XZ,
};
use crate::{ForestExtent, ForestIndex, GroundCoverGroveKind};

/// Near High half-extent (8 × 160 m). Same Chebyshev family as playable Near terrain.
pub const GROUND_COVER_RADIUS_M: f32 = 8.0 * BUMP_OUT_CELL_XZ;

/// Far High starts this far inside the Near disk (one 320 m cell).
pub const GROUND_COVER_FAR_OVERLAP_M: f32 = 2.0 * BUMP_OUT_CELL_XZ;

/// Snap shared with Near / Far terrain streams (640 m).
pub const GROUND_COVER_ANCHOR_STEP_M: f32 = 4.0 * BUMP_OUT_CELL_XZ;

/// Floor-color recipe class. Same shader as canopy; centimetre displace + paint.
///
/// [`GameTrail`] stays for a later corridor mask. Forest throw maps onto moss
/// or field only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroundCoverKind {
	MossyBumps,
	GameTrail,
	Field,
}

/// One blended floor-color sample (empty when the cell should not paint).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundCoverSample {
	pub grove: Option<GroundCoverGroveKind>,
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
			grove: None,
			kind: None,
			density: 0.0,
			bite_size: 1.0,
			bite_size_deviation: 0.0,
			height_m: 0.0,
			height_deviation_m: 0.0,
			palette: moss_palette(),
		}
	}

	pub fn from_grove(grove: GroundCoverGroveKind, variation: f32) -> Self {
		let t = variation.clamp(0.0, 1.0);
		Self {
			grove: Some(grove),
			kind: Some(grove.overlay_kind()),
			density: grove.density(),
			bite_size: grove.bite_size(),
			bite_size_deviation: grove.bite_size_deviation(),
			height_m: grove.height_m(t),
			height_deviation_m: grove.height_deviation_m(),
			palette: grove.palette(),
		}
	}

	pub fn from_kind(kind: GroundCoverKind, variation: f32) -> Self {
		let t = variation.clamp(0.0, 1.0);
		match kind {
			GroundCoverKind::MossyBumps => Self {
				grove: None,
				kind: Some(kind),
				density: 0.82,
				bite_size: 2.4,
				bite_size_deviation: 0.35,
				height_m: 0.04 + 0.14 * t,
				height_deviation_m: 0.06,
				palette: moss_palette(),
			},
			GroundCoverKind::GameTrail => Self {
				grove: None,
				kind: Some(kind),
				density: 0.28,
				bite_size: 3.2,
				bite_size_deviation: 0.22,
				height_m: -0.02 + 0.02 * t,
				height_deviation_m: 0.02,
				palette: trail_palette(),
			},
			GroundCoverKind::Field => Self {
				grove: None,
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
}

impl GroundCoverGroveKind {
	/// Shader style class for this grove. Trails are not a forest throw.
	pub fn overlay_kind(self) -> GroundCoverKind {
		match self {
			Self::HuelgoatPitch | Self::FloorScrub | Self::FleckingBed | Self::JimsCollage => {
				GroundCoverKind::MossyBumps
			}
			Self::Allbed | Self::GrassyMounds => GroundCoverKind::Field,
		}
	}

	pub fn density(self) -> f32 {
		match self {
			Self::HuelgoatPitch => 0.82,
			Self::FloorScrub => 0.78,
			Self::FleckingBed => 0.68,
			Self::JimsCollage => 0.74,
			Self::Allbed => 0.90,
			Self::GrassyMounds => 0.88,
		}
	}

	pub fn bite_size(self) -> f32 {
		match self {
			Self::HuelgoatPitch | Self::FloorScrub | Self::FleckingBed | Self::JimsCollage => 2.4,
			Self::Allbed | Self::GrassyMounds => 14.0,
		}
	}

	pub fn bite_size_deviation(self) -> f32 {
		match self {
			Self::HuelgoatPitch | Self::FloorScrub => 0.35,
			Self::FleckingBed | Self::JimsCollage => 0.28,
			Self::Allbed => 0.18,
			Self::GrassyMounds => 0.22,
		}
	}

	pub fn height_m(self, variation: f32) -> f32 {
		let t = variation.clamp(0.0, 1.0);
		match self {
			Self::HuelgoatPitch => 0.04 + 0.14 * t,
			Self::FloorScrub => 0.03 + 0.10 * t,
			Self::FleckingBed => 0.02 + 0.08 * t,
			Self::JimsCollage => 0.03 + 0.11 * t,
			Self::Allbed => 0.02 + 0.06 * t,
			Self::GrassyMounds => 0.04 + 0.08 * t,
		}
	}

	pub fn height_deviation_m(self) -> f32 {
		match self {
			Self::HuelgoatPitch => 0.06,
			Self::FloorScrub | Self::JimsCollage => 0.05,
			Self::FleckingBed => 0.04,
			Self::Allbed => 0.03,
			Self::GrassyMounds => 0.04,
		}
	}

	pub fn palette(self) -> [Color; 3] {
		match self {
			Self::HuelgoatPitch => moss_palette(),
			Self::FloorScrub => {
				[rgb(0.12, 0.20, 0.08), rgb(0.22, 0.30, 0.10), rgb(0.34, 0.38, 0.14)]
			}
			Self::FleckingBed => {
				[rgb(0.08, 0.22, 0.08), rgb(0.16, 0.34, 0.12), rgb(0.28, 0.40, 0.14)]
			}
			Self::JimsCollage => {
				[rgb(0.10, 0.18, 0.08), rgb(0.20, 0.30, 0.10), rgb(0.32, 0.42, 0.16)]
			}
			Self::Allbed => field_palette(),
			Self::GrassyMounds => {
				[rgb(0.18, 0.36, 0.10), rgb(0.32, 0.50, 0.14), rgb(0.52, 0.58, 0.20)]
			}
		}
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

	pub fn center_grove(&self) -> Option<GroundCoverGroveKind> {
		self.samples[4].grove
	}

	pub fn center_palette(&self) -> [Color; 3] {
		self.samples[4].palette
	}

	/// Density-weighted mix of the 3×3 palettes so identity seams do not snap.
	pub fn blended_palette(&self) -> [Color; 3] {
		let mut acc = [[0.0f32; 3]; 3];
		let mut weight = 0.0f32;
		for sample in &self.samples {
			let density = sample.density;
			if density <= 0.001 {
				continue;
			}
			weight += density;
			for (i, color) in sample.palette.iter().enumerate() {
				let srgba = color.to_srgba();
				acc[i][0] += srgba.red * density;
				acc[i][1] += srgba.green * density;
				acc[i][2] += srgba.blue * density;
			}
		}
		if weight <= 0.001 {
			return self.center_palette();
		}
		acc.map(|channel| {
			Color::srgb(channel[0] / weight, channel[1] / weight, channel[2] / weight)
		})
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
	soften_identity_changes(&mut samples);
	samples
}

fn ground_cover_sample_on_bounds(index: &ForestIndex, bounds: Aabb3d) -> GroundCoverSample {
	let center = Vec3::from((bounds.min + bounds.max) * 0.5);
	let (forest_ix, forest_iz) = ForestExtent::cell_index_containing(center);
	let grove = index
		.selected_layers_for(ForestExtent::from_cell_index(forest_ix, forest_iz))
		.ground_cover;
	let Some(grove) = grove else {
		return GroundCoverSample::empty();
	};
	let (ix, iz) = ground_cover_cell_index(bounds);
	let variation = cell_hash01(ix, iz, index.noise.seed as u32);
	GroundCoverSample::from_grove(grove, variation)
}

fn soften_identity_changes(samples: &mut [GroundCoverSample; 9]) {
	let center = samples[4].grove;
	for (index, sample) in samples.iter_mut().enumerate() {
		if index == 4 || sample.grove == center {
			continue;
		}
		sample.density *= 0.55;
	}
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{ForestIndex, LayeringKind};
	use anyhow::Result;

	#[test]
	fn recipes_use_centimetre_heights() -> Result<()> {
		let moss = GroundCoverSample::from_grove(GroundCoverGroveKind::HuelgoatPitch, 0.5);
		assert!(moss.height_m > 0.03 && moss.height_m < 0.20);
		assert!(moss.height_deviation_m < 0.10);
		assert!(moss.density > 0.7);
		assert_eq!(moss.kind, Some(GroundCoverKind::MossyBumps));

		let trail = GroundCoverSample::from_kind(GroundCoverKind::GameTrail, 0.5);
		assert!(trail.height_m <= 0.0);
		assert!(trail.height_m >= -0.03);
		assert!(trail.density < 0.4);

		let field = GroundCoverSample::from_grove(GroundCoverGroveKind::Allbed, 0.5);
		assert!(field.height_m > 0.01 && field.height_m < 0.09);
		assert!(field.bite_size > 8.0);
		assert!(field.density > 0.8);
		assert_eq!(field.kind, Some(GroundCoverKind::Field));
		Ok(())
	}

	#[test]
	fn empty_sample_has_no_density() -> Result<()> {
		let empty = GroundCoverSample::empty();
		assert!(empty.kind.is_none());
		assert!(empty.grove.is_none());
		assert!(empty.density <= 0.001);
		Ok(())
	}

	#[test]
	fn barren_typical_cover_skips() -> Result<()> {
		assert!(LayeringKind::SunsBarren.layering().typical_layers().ground_cover.is_none());
		assert!(LayeringKind::OwlsDesert.layering().typical_layers().ground_cover.is_none());
		Ok(())
	}

	#[test]
	fn meadowland_typical_cover_is_huelgoat_pitch() -> Result<()> {
		assert_eq!(
			LayeringKind::Meadowland.layering().typical_layers().ground_cover,
			Some(GroundCoverGroveKind::HuelgoatPitch)
		);
		Ok(())
	}

	#[test]
	fn mi_robles_typical_cover_is_allbed() -> Result<()> {
		assert_eq!(
			LayeringKind::MiRobles.layering().typical_layers().ground_cover,
			Some(GroundCoverGroveKind::Allbed)
		);
		Ok(())
	}

	#[test]
	fn pinned_forest_neighborhood_shares_one_identity() -> Result<()> {
		let mut index = ForestIndex::default();
		index.layering = Some(LayeringKind::MiRobles);
		let samples = blend_ground_cover_neighborhood(&index, bump_out_cell_bounds(0, 0));
		assert!(samples.iter().all(|sample| sample.grove == Some(GroundCoverGroveKind::Allbed)));
		assert!(samples.iter().all(|sample| sample.kind == Some(GroundCoverKind::Field)));
		Ok(())
	}

	#[test]
	fn identity_change_softens_neighbor_density() -> Result<()> {
		let mut samples = [GroundCoverSample::from_grove(GroundCoverGroveKind::Allbed, 0.5); 9];
		samples[5] = GroundCoverSample::from_grove(GroundCoverGroveKind::HuelgoatPitch, 0.5);
		let before = samples[5].density;
		soften_identity_changes(&mut samples);
		assert!((samples[4].density - GroundCoverGroveKind::Allbed.density()).abs() < 1e-4);
		assert!((samples[5].density - before * 0.55).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn blended_palette_mixes_neighbor_stops() -> Result<()> {
		let mut samples = [GroundCoverSample::from_grove(GroundCoverGroveKind::Allbed, 0.5); 9];
		samples[0] = GroundCoverSample::from_grove(GroundCoverGroveKind::HuelgoatPitch, 0.5);
		let cell = GroundCoverBumpOut { bounds: bump_out_cell_bounds(0, 0), samples };
		let mixed = cell.blended_palette();
		let field = GroundCoverGroveKind::Allbed.palette()[1].to_srgba();
		let moss = GroundCoverGroveKind::HuelgoatPitch.palette()[1].to_srgba();
		let mid = mixed[1].to_srgba();
		assert!(mid.green < field.green || mid.red < field.red);
		assert!(mid.green > moss.green || mid.red > moss.red);
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
}
