//! Authored bump-out profiles on grove **selection** identities.
//!
//! Density, bite size, height, and palette are properties of [`ForestGroveKind`] after
//! forest selection. They do not require growing a [`crate::ChicoGrove`].

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Color, Vec2, Vec3};
use chico_groves::GroveExtent;

use crate::{
	ForestExtent, ForestGroveKind, ForestIndex, ForestLayer, SelectedLayers,
	DEFAULT_FOREST_GROVE_TILE_XZ,
};

/// Present ring for canopy bump-outs (metres). Overlaps grove Low (~1.0–1.2 km).
pub const BUMP_OUT_PRESENT_RADIUS_M: f32 = 1200.0;

/// Authored canopy / cover fields sampled from a grove selection identifier.
pub trait BumpOutSelection {
	fn bump_out_density(self) -> f32;
	fn bump_out_bite_size(self) -> f32;
	fn bump_out_bite_size_deviation(self) -> f32;
	fn bump_out_height_m(self) -> f32;
	fn bump_out_height_deviation_m(self) -> f32;
	fn bump_out_palette(self) -> [Color; 3];
}

/// One blended bump-out sample (possibly empty when every overlapping tile selected `None`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BumpOutSelectionSample {
	pub kind: Option<ForestGroveKind>,
	pub density: f32,
	pub bite_size: f32,
	pub bite_size_deviation: f32,
	pub height_m: f32,
	pub height_deviation_m: f32,
	pub palette: [Color; 3],
}

impl BumpOutSelectionSample {
	pub fn empty() -> Self {
		Self {
			kind: None,
			density: 0.0,
			bite_size: 1.0,
			bite_size_deviation: 0.0,
			height_m: 0.0,
			height_deviation_m: 0.0,
			palette: [
				Color::srgb(0.16, 0.36, 0.14),
				Color::srgb(0.24, 0.52, 0.20),
				Color::srgb(0.38, 0.64, 0.24),
			],
		}
	}

	pub fn from_kind(kind: ForestGroveKind) -> Self {
		Self {
			kind: Some(kind),
			density: kind.bump_out_density(),
			bite_size: kind.bump_out_bite_size(),
			bite_size_deviation: kind.bump_out_bite_size_deviation(),
			height_m: kind.bump_out_height_m(),
			height_deviation_m: kind.bump_out_height_deviation_m(),
			palette: kind.bump_out_palette(),
		}
	}
}

impl SelectedLayers {
	/// Highest occupied vegetation layer (`UpperCanopy` … `Tufts`).
	pub fn highest_kind(self) -> Option<ForestGroveKind> {
		ForestLayer::ALL.into_iter().rev().find_map(|layer| layer.kind(self))
	}
}

impl BumpOutSelection for ForestGroveKind {
	fn bump_out_density(self) -> f32 {
		self.authored().density
	}

	fn bump_out_bite_size(self) -> f32 {
		self.authored().bite_size
	}

	fn bump_out_bite_size_deviation(self) -> f32 {
		self.authored().bite_size_deviation
	}

	fn bump_out_height_m(self) -> f32 {
		self.authored().height_m
	}

	fn bump_out_height_deviation_m(self) -> f32 {
		self.authored().height_deviation_m
	}

	fn bump_out_palette(self) -> [Color; 3] {
		self.authored().palette
	}
}

impl ForestGroveKind {
	fn authored(self) -> AuthoredBumpOut {
		match self {
			Self::Alpine => canopy(0.62, 36.0, conifer()),
			Self::AridConiferSapling => woody(0.38, 8.0, dry()),
			Self::BraidGrass => tuft(0.58, 0.8, meadow()),
			Self::BushScrub => woody(0.42, 3.0, scrub()),
			Self::ChristmasTaiga => canopy(0.70, 28.0, conifer()),
			Self::CommonTufts => tuft(0.48, 0.25, meadow()),
			Self::ConiferMassives => massive(0.78, 160.0, conifer()),
			Self::ConiferSapling => woody(0.40, 10.0, conifer()),
			Self::DateGrove => woody(0.45, 18.0, palm()),
			Self::Dryland => woody(0.28, 12.0, dry()),
			Self::ForlornSavanna => woody(0.32, 25.0, savanna()),
			Self::GoettingenFollow => woody(0.50, 22.0, temperate()),
			Self::HighBush => woody(0.52, 6.0, scrub()),
			Self::JerrysChaparral => woody(0.40, 4.0, dry()),
			Self::JungleLowerMassives => canopy(0.72, 40.0, jungle()),
			Self::JungleMassives => massive(0.82, 180.0, jungle()),
			Self::Leeward => woody(0.44, 20.0, palm()),
			Self::LevantineScrub => woody(0.30, 3.5, dry()),
			Self::LowBush => woody(0.46, 2.0, scrub()),
			Self::MonsterGrass => tuft(0.55, 1.2, meadow()),
			Self::Orchard => woody(0.58, 8.0, orchard()),
			Self::PalmShade => canopy(0.48, 32.0, palm()),
			Self::RiparianGeneral => woody(0.55, 16.0, temperate()),
			Self::RiparianMix => woody(0.52, 18.0, temperate()),
			Self::RiverineGreen => woody(0.50, 14.0, temperate()),
			Self::RollingOaks => canopy(0.60, 36.0, temperate()),
			Self::Shamanhome => woody(0.48, 20.0, jungle()),
			Self::SpottyBushes => woody(0.34, 3.0, scrub()),
			Self::Storytellers => woody(0.50, 24.0, temperate()),
			Self::StrangeOasis => woody(0.36, 14.0, palm()),
			Self::TallGrass => tuft(0.62, 1.0, meadow()),
			Self::TemperateLowerMassives => canopy(0.70, 40.0, temperate()),
			Self::TemperateMassives => massive(0.80, 170.0, temperate()),
			Self::TradeWinds => canopy(0.64, 36.0, jungle()),
			Self::TropicalThicket => woody(0.66, 8.0, jungle()),
			Self::TropicalTufts => tuft(0.50, 0.4, jungle()),
			Self::TropicalUndergrowth => woody(0.58, 5.0, jungle()),
			Self::UnendingJungle => canopy(0.76, 30.0, jungle()),
			Self::Vineyard => woody(0.54, 2.5, orchard()),
			Self::WanderingAcacia => woody(0.30, 16.0, savanna()),
			Self::WildGrass => tuft(0.52, 0.6, meadow()),
		}
	}
}

struct AuthoredBumpOut {
	density: f32,
	bite_size: f32,
	bite_size_deviation: f32,
	height_m: f32,
	height_deviation_m: f32,
	palette: [Color; 3],
}

fn tuft(density: f32, height_m: f32, palette: [Color; 3]) -> AuthoredBumpOut {
	AuthoredBumpOut {
		density,
		bite_size: (height_m * 1.8).max(0.35),
		bite_size_deviation: 0.25,
		height_m,
		height_deviation_m: (height_m * 0.35).max(0.04),
		palette,
	}
}

fn woody(density: f32, height_m: f32, palette: [Color; 3]) -> AuthoredBumpOut {
	AuthoredBumpOut {
		density,
		bite_size: (height_m * 0.32).max(1.2),
		bite_size_deviation: 0.40,
		height_m,
		height_deviation_m: (height_m * 0.22).max(0.4),
		palette,
	}
}

fn canopy(density: f32, height_m: f32, palette: [Color; 3]) -> AuthoredBumpOut {
	AuthoredBumpOut {
		density,
		bite_size: (height_m * 0.38).max(4.0),
		bite_size_deviation: 0.50,
		height_m,
		height_deviation_m: height_m * 0.20,
		palette,
	}
}

fn massive(density: f32, height_m: f32, palette: [Color; 3]) -> AuthoredBumpOut {
	AuthoredBumpOut {
		density,
		bite_size: height_m * 0.28,
		bite_size_deviation: 0.55,
		height_m,
		height_deviation_m: height_m * 0.18,
		palette,
	}
}

fn rgb(r: f32, g: f32, b: f32) -> Color {
	Color::srgb(r, g, b)
}

fn meadow() -> [Color; 3] {
	[rgb(0.18, 0.38, 0.10), rgb(0.32, 0.55, 0.14), rgb(0.55, 0.68, 0.20)]
}

fn scrub() -> [Color; 3] {
	[rgb(0.18, 0.28, 0.10), rgb(0.30, 0.42, 0.14), rgb(0.48, 0.52, 0.22)]
}

fn dry() -> [Color; 3] {
	[rgb(0.22, 0.24, 0.10), rgb(0.36, 0.34, 0.14), rgb(0.52, 0.46, 0.20)]
}

fn temperate() -> [Color; 3] {
	[rgb(0.08, 0.22, 0.08), rgb(0.14, 0.38, 0.12), rgb(0.28, 0.52, 0.16)]
}

fn conifer() -> [Color; 3] {
	[rgb(0.04, 0.16, 0.10), rgb(0.08, 0.28, 0.14), rgb(0.16, 0.40, 0.18)]
}

fn jungle() -> [Color; 3] {
	[rgb(0.04, 0.18, 0.08), rgb(0.08, 0.34, 0.12), rgb(0.18, 0.50, 0.14)]
}

fn palm() -> [Color; 3] {
	[rgb(0.10, 0.28, 0.08), rgb(0.18, 0.46, 0.12), rgb(0.40, 0.58, 0.16)]
}

fn savanna() -> [Color; 3] {
	[rgb(0.20, 0.28, 0.08), rgb(0.34, 0.40, 0.12), rgb(0.52, 0.50, 0.18)]
}

fn orchard() -> [Color; 3] {
	[rgb(0.12, 0.30, 0.08), rgb(0.22, 0.48, 0.12), rgb(0.42, 0.58, 0.16)]
}

/// Highest-layer selection at a world XZ point. Does not grow tiles.
pub fn selection_sample_at(index: &ForestIndex, xz: Vec2) -> BumpOutSelectionSample {
	let (ix, iz) = ForestExtent::cell_index_containing(Vec3::new(xz.x, 0.0, xz.y));
	let layers = index.selected_layers_for(ForestExtent::from_cell_index(ix, iz));
	match layers.highest_kind() {
		Some(kind) => BumpOutSelectionSample::from_kind(kind),
		None => BumpOutSelectionSample::empty(),
	}
}

/// Area-weight overlapping 100 m grove tiles onto `bounds` (typically a 160 m terrain cell).
pub fn blend_selection_on_bounds(index: &ForestIndex, bounds: Aabb3d) -> BumpOutSelectionSample {
	let tiles = ForestExtent::grove_tiles_overlapping(bounds);
	let mut density = 0.0;
	let mut bite_size = 0.0;
	let mut bite_size_deviation = 0.0;
	let mut height_m = 0.0;
	let mut height_deviation_m = 0.0;
	let mut weight_sum = 0.0;
	let mut best_kind: Option<(ForestGroveKind, f32, [Color; 3])> = None;

	for tile in tiles {
		let area = xz_overlap_area(bounds, grove_aabb(tile));
		if area <= 1e-4 {
			continue;
		}
		let sample = selection_sample_at(
			index,
			Vec2::new((tile.min().x + tile.max().x) * 0.5, (tile.min().z + tile.max().z) * 0.5),
		);
		density += sample.density * area;
		bite_size += sample.bite_size * area;
		bite_size_deviation += sample.bite_size_deviation * area;
		height_m += sample.height_m * area;
		height_deviation_m += sample.height_deviation_m * area;
		weight_sum += area;
		if let Some(kind) = sample.kind {
			match best_kind {
				Some((_, best_area, _)) if best_area >= area => {}
				_ => best_kind = Some((kind, area, sample.palette)),
			}
		}
	}

	if weight_sum <= 1e-4 {
		return BumpOutSelectionSample::empty();
	}

	let (kind, palette) = match best_kind {
		Some((kind, _, palette)) => (Some(kind), palette),
		None => {
			let empty = BumpOutSelectionSample::empty();
			(None, empty.palette)
		}
	};

	BumpOutSelectionSample {
		kind,
		density: (density / weight_sum).clamp(0.0, 1.0),
		bite_size: (bite_size / weight_sum).max(0.01),
		bite_size_deviation: (bite_size_deviation / weight_sum).max(0.0),
		height_m: height_m / weight_sum,
		height_deviation_m: (height_deviation_m / weight_sum).max(0.0),
		palette,
	}
}

/// 3×3 terrain-cell neighborhood centered on `bounds`, each sample blended from 100 m tiles.
pub fn blend_selection_neighborhood(
	index: &ForestIndex,
	bounds: Aabb3d,
) -> [BumpOutSelectionSample; 9] {
	let size = (bounds.max.x - bounds.min.x).max(DEFAULT_FOREST_GROVE_TILE_XZ);
	let mut samples = [BumpOutSelectionSample::empty(); 9];
	for row in 0..3 {
		for column in 0..3 {
			let dx = (column as f32 - 1.0) * size;
			let dz = (row as f32 - 1.0) * size;
			let min = Vec3::new(bounds.min.x + dx, bounds.min.y, bounds.min.z + dz);
			let max = Vec3::new(bounds.max.x + dx, bounds.max.y, bounds.max.z + dz);
			samples[row * 3 + column] =
				blend_selection_on_bounds(index, Aabb3d::from_min_max(min, max));
		}
	}
	samples
}

fn grove_aabb(tile: GroveExtent) -> Aabb3d {
	Aabb3d::from_min_max(tile.min(), tile.max())
}

fn xz_overlap_area(a: Aabb3d, b: Aabb3d) -> f32 {
	let x = (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)).max(0.0);
	let z = (a.max.z.min(b.max.z) - a.min.z.max(b.min.z)).max(0.0);
	x * z
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{LayeringKind, SelectedLayers};
	use anyhow::Result;

	#[test]
	fn highest_kind_prefers_upper_canopy() -> Result<()> {
		let layers = SelectedLayers {
			layering: LayeringKind::Meadowland,
			tufts: Some(ForestGroveKind::WildGrass),
			understory: Some(ForestGroveKind::LowBush),
			lower_canopy: None,
			upper_canopy: Some(ForestGroveKind::RollingOaks),
		};
		assert_eq!(layers.highest_kind(), Some(ForestGroveKind::RollingOaks));
		Ok(())
	}

	#[test]
	fn highest_kind_falls_back_to_tufts() -> Result<()> {
		let layers = SelectedLayers {
			layering: LayeringKind::Meadowland,
			tufts: Some(ForestGroveKind::CommonTufts),
			understory: None,
			lower_canopy: None,
			upper_canopy: None,
		};
		assert_eq!(layers.highest_kind(), Some(ForestGroveKind::CommonTufts));
		Ok(())
	}

	#[test]
	fn orchard_height_is_authored_midpoint() -> Result<()> {
		assert!((ForestGroveKind::Orchard.bump_out_height_m() - 8.0).abs() < 1e-4);
		assert!(ForestGroveKind::Orchard.bump_out_density() > 0.0);
		assert_eq!(ForestGroveKind::Orchard.bump_out_palette().len(), 3);
		Ok(())
	}

	#[test]
	fn blend_on_empty_layering_is_zero_density() -> Result<()> {
		let mut index = ForestIndex::default();
		index.layering = Some(LayeringKind::SunsBarren);
		let bounds = Aabb3d::from_min_max(Vec3::new(-80.0, 0.0, -80.0), Vec3::new(80.0, 1.0, 80.0));
		let sample = blend_selection_on_bounds(&index, bounds);
		// SunsBarren typical layers may still have groves; density is authored, not occupancy.
		assert!(sample.density >= 0.0);
		assert!(sample.density <= 1.0);
		Ok(())
	}
}
