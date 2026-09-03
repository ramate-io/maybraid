//! Selected development cell: empty or Les Halles + pad.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use jersey_terrain_stamps::JerseyModulation;
use procedural_common::SeededHash;

use crate::cell::{
	cell_selected, BUILDING_INSET, DEVELOPMENT_CELL_SIZE, MAX_CONFINES_HEIGHT, MIN_CONFINES_HEIGHT,
	MIN_FOOTPRINT,
};
use crate::config::DevelopmentConfig;
use crate::pad::flatten_pad;

/// Fill kind for one development cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentKind {
	Empty,
	LesHalles,
}

/// Pad baked from a post-Marazion height sample.
#[derive(Debug, Clone)]
pub struct DevelopmentPad {
	pub height: f32,
	pub modulation: JerseyModulation,
}

/// One 100 m tile after selection.
#[derive(Debug, Clone)]
pub struct DevelopmentCell {
	pub cell: Aabb3d,
	pub kind: DevelopmentKind,
	pub pad: Option<DevelopmentPad>,
	/// Sampled confines height (storey stack), valid when [`Self::kind`] is Les Halles.
	pub confines_height: f32,
	/// Sampled plan footprint, inset from the cell so the slab sits on the pad.
	pub confines_extent_xz: Vec2,
}

impl DevelopmentCell {
	pub fn empty(cell: Aabb3d) -> Self {
		Self {
			cell,
			kind: DevelopmentKind::Empty,
			pad: None,
			confines_height: 0.0,
			confines_extent_xz: Vec2::ZERO,
		}
	}

	pub fn is_filled(&self) -> bool {
		self.kind == DevelopmentKind::LesHalles && self.pad.is_some()
	}

	pub fn pad_modulation(&self) -> Option<&JerseyModulation> {
		self.pad.as_ref().map(|p| &p.modulation)
	}

	/// Confines AABB sitting on the pad (world space).
	pub fn confines_bounds(&self) -> Option<Aabb3d> {
		let pad = self.pad.as_ref()?;
		if self.kind != DevelopmentKind::LesHalles {
			return None;
		}
		let c = Vec2::new(
			(self.cell.min.x + self.cell.max.x) * 0.5,
			(self.cell.min.z + self.cell.max.z) * 0.5,
		);
		let hx = self.confines_extent_xz.x * 0.5;
		let hz = self.confines_extent_xz.y * 0.5;
		let y0 = pad.height;
		Some(Aabb3d::from_min_max(
			bevy::math::Vec3::new(c.x - hx, y0, c.y - hz),
			bevy::math::Vec3::new(c.x + hx, y0 + self.confines_height, c.y + hz),
		))
	}

	pub fn filled(cell: Aabb3d, pad_height: f32, config: &DevelopmentConfig) -> Self {
		let hash = SeededHash::new(config.seed.wrapping_add(cell_salt(cell)));
		let max_foot = (DEVELOPMENT_CELL_SIZE - 2.0 * BUILDING_INSET).max(MIN_FOOTPRINT);
		let extent_x = MIN_FOOTPRINT + (max_foot - MIN_FOOTPRINT) * hash.unit(11);
		let extent_z = MIN_FOOTPRINT + (max_foot - MIN_FOOTPRINT) * hash.unit(13);
		let confines_height =
			MIN_CONFINES_HEIGHT + (MAX_CONFINES_HEIGHT - MIN_CONFINES_HEIGHT) * hash.unit(17);
		Self {
			cell,
			kind: DevelopmentKind::LesHalles,
			pad: Some(DevelopmentPad {
				height: pad_height,
				modulation: flatten_pad(cell, pad_height),
			}),
			confines_height,
			confines_extent_xz: Vec2::new(extent_x, extent_z),
		}
	}
}

pub fn should_fill(cell: Aabb3d, config: &DevelopmentConfig) -> bool {
	cell_selected(cell, config.occupancy_seed(), config.likelihood, config.spatial_correlation)
}

fn cell_salt(cell: Aabb3d) -> u32 {
	cell.min.x.to_bits().wrapping_mul(73856093) ^ cell.min.z.to_bits().wrapping_mul(19349663)
}
