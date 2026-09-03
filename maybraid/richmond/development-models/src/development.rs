//! Selected development cell: empty or Les Halles + pad.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use procedural_common::SeededHash;

use crate::cell::{
	available_footprint, cell_selected, inscribe_yawed_extents, sample_confines_yaw,
	MAX_CONFINES_HEIGHT, MIN_CONFINES_HEIGHT, MIN_FOOTPRINT,
};
use crate::config::DevelopmentConfig;
use crate::finish::DevelopmentFinish;
use crate::pad::{cell_center_xz, PadComplex, PadParams};
use richmond_buildings::{Confines, Openings};

/// Fill kind for one development cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentKind {
	Empty,
	LesHalles,
}

/// Pad baked from a post-Marazion height sample: flatten terrace + ease skirt.
#[derive(Debug, Clone)]
pub struct DevelopmentPad {
	pub height: f32,
	pub complex: PadComplex,
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
	/// Discrete yaw (radians) applied at host spawn about the cell center.
	pub confines_yaw: f32,
	/// Wall / roof shader look, valid when [`Self::kind`] is Les Halles.
	pub finish: Option<DevelopmentFinish>,
}

impl DevelopmentCell {
	pub fn empty(cell: Aabb3d) -> Self {
		Self {
			cell,
			kind: DevelopmentKind::Empty,
			pad: None,
			confines_height: 0.0,
			confines_extent_xz: Vec2::ZERO,
			confines_yaw: 0.0,
			finish: None,
		}
	}

	pub fn is_filled(&self) -> bool {
		self.kind == DevelopmentKind::LesHalles && self.pad.is_some()
	}

	pub fn pad_complex(&self) -> Option<&PadComplex> {
		self.pad.as_ref().map(|p| &p.complex)
	}

	/// Unrotated confines AABB sitting on the pad (world space).
	///
	/// Les Halles authors against this axis-aligned box. [`Self::confines_yaw`] is
	/// recorded on [`Confines::roll`] and applied at host spawn about the cell center.
	/// Label wireframes fill the AABB with identity local yaw so they inherit that pose.
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

	/// Fitted confines: unrotated AABB plus yaw on [`Confines::roll`].
	pub fn confines(&self) -> Option<Confines> {
		Some(Confines::new(self.confines_bounds()?, self.confines_yaw, Openings::new()))
	}

	pub fn filled(cell: Aabb3d, pad_height: f32, config: &DevelopmentConfig) -> Self {
		let hash = SeededHash::new(config.seed.wrapping_add(cell_salt(cell)));
		let max_foot = available_footprint();
		let yaw = sample_confines_yaw(hash.unit(37));
		let extent_x = MIN_FOOTPRINT + (max_foot - MIN_FOOTPRINT) * hash.unit(11);
		let extent_z = MIN_FOOTPRINT + (max_foot - MIN_FOOTPRINT) * hash.unit(13);
		let confines_height =
			MIN_CONFINES_HEIGHT + (MAX_CONFINES_HEIGHT - MIN_CONFINES_HEIGHT) * hash.unit(17);
		let confines_extent_xz = inscribe_yawed_extents(extent_x, extent_z, yaw, max_foot);
		Self {
			cell,
			kind: DevelopmentKind::LesHalles,
			pad: Some(DevelopmentPad {
				height: pad_height,
				complex: PadComplex::building_skirt(
					cell_center_xz(cell),
					confines_extent_xz * 0.5,
					yaw,
					pad_height,
					PadParams::default(),
				),
			}),
			confines_height,
			confines_extent_xz,
			confines_yaw: yaw,
			finish: Some(DevelopmentFinish::pick(hash)),
		}
	}
}

pub fn should_fill(cell: Aabb3d, config: &DevelopmentConfig) -> bool {
	cell_selected(cell, config.occupancy_seed(), config.likelihood, config.spatial_correlation)
}

fn cell_salt(cell: Aabb3d) -> u32 {
	cell.min.x.to_bits().wrapping_mul(73856093) ^ cell.min.z.to_bits().wrapping_mul(19349663)
}

#[cfg(test)]
mod tests {
	use bevy::math::bounding::Aabb3d;
	use bevy::math::Vec3;
	use material_ref::MaterialId;
	use std::f32::consts::TAU;

	use super::*;
	use crate::cell::{available_footprint, yawed_plan_aabb_extent};

	#[test]
	fn filled_cell_picks_urban_finish() {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let filled = DevelopmentCell::filled(cell, 12.0, &DevelopmentConfig::default());
		let finish = filled.finish.expect("filled cells pick a finish");
		assert!(matches!(
			&finish.wall.name,
			MaterialId::Name(n) if n == "stucco" || n == "wood"
		));
		assert!(matches!(
			&finish.roof.name,
			MaterialId::Name(n) if n == "iron" || n == "terracotta" || n == "hay"
		));
	}

	#[test]
	fn filled_cell_samples_continuous_yaw() {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let eighth = TAU / 8.0;
		let mut off_grid = false;
		for seed in 0..48u32 {
			let config = DevelopmentConfig { seed, ..DevelopmentConfig::default() };
			let filled = DevelopmentCell::filled(cell, 12.0, &config);
			assert!(filled.confines_yaw >= 0.0 && filled.confines_yaw <= TAU + 1e-5);
			let phase = filled.confines_yaw.rem_euclid(eighth);
			if phase > 0.05 && phase < eighth - 0.05 {
				off_grid = true;
			}
			let pad = available_footprint();
			let occupied = yawed_plan_aabb_extent(
				filled.confines_extent_xz.x,
				filled.confines_extent_xz.y,
				filled.confines_yaw,
			);
			assert!(occupied.x <= pad + 1e-3, "yawed AABB x {} exceeds pad {}", occupied.x, pad);
			assert!(occupied.y <= pad + 1e-3, "yawed AABB z {} exceeds pad {}", occupied.y, pad);
			let confines = filled.confines().expect("filled cell has confines");
			assert!((confines.roll - filled.confines_yaw).abs() < 1e-6);
		}
		assert!(off_grid, "expected at least one heading off the old π/4 lattice");
	}

	#[test]
	fn filled_cell_pad_flattens_the_building_center() {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let filled = DevelopmentCell::filled(cell, 12.0, &DevelopmentConfig::default());
		let pad = filled.pad_complex().expect("filled cell has a pad");
		let c = cell_center_xz(cell);
		assert!((pad.modify_elevation(3.0, c.x, c.y) - 12.0).abs() < 1e-3);
		assert!((pad.modify_elevation(3.0, 400.0, 400.0) - 3.0).abs() < 1e-3);
	}
}
