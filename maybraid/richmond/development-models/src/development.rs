//! Selected development cell with kind-specific payloads.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use procedural_common::SeededHash;
use richmond_developments::ShepherdsVillage;

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
	ShepherdsVillage,
}

/// Pad baked from a post-Marazion height sample: flatten terrace + ease skirt.
#[derive(Debug, Clone)]
pub struct DevelopmentPad {
	pub height: f32,
	pub complex: PadComplex,
}

/// Data owned only by a selected Les Halles cell.
#[derive(Debug, Clone)]
pub struct LesHallesCell {
	pub pad: DevelopmentPad,
	pub confines_height: f32,
	pub confines_extent_xz: Vec2,
	pub confines_yaw: f32,
	pub finish: DevelopmentFinish,
}

/// Data owned only by a selected Shepherds Village cell.
#[derive(Debug, Clone)]
pub struct ShepherdsVillageCell {
	pub pads: Vec<DevelopmentPad>,
	pub village: ShepherdsVillage,
}

/// Mutually exclusive content generated for one development tile.
#[derive(Debug, Clone)]
pub enum DevelopmentContent {
	Empty,
	LesHalles(LesHallesCell),
	ShepherdsVillage(ShepherdsVillageCell),
}

/// One development tile after selection.
#[derive(Debug, Clone)]
pub struct DevelopmentCell {
	pub cell: Aabb3d,
	pub content: DevelopmentContent,
}

impl DevelopmentCell {
	pub fn empty(cell: Aabb3d) -> Self {
		Self { cell, content: DevelopmentContent::Empty }
	}

	pub fn kind(&self) -> DevelopmentKind {
		match self.content {
			DevelopmentContent::Empty => DevelopmentKind::Empty,
			DevelopmentContent::LesHalles(_) => DevelopmentKind::LesHalles,
			DevelopmentContent::ShepherdsVillage(_) => DevelopmentKind::ShepherdsVillage,
		}
	}

	pub fn is_filled(&self) -> bool {
		!matches!(self.content, DevelopmentContent::Empty)
	}

	pub fn pad_complex(&self) -> Option<&PadComplex> {
		self.pads().next().map(|p| &p.complex)
	}

	pub fn pads(&self) -> impl Iterator<Item = &DevelopmentPad> {
		let pads: &[DevelopmentPad] = match &self.content {
			DevelopmentContent::Empty => &[],
			DevelopmentContent::LesHalles(content) => std::slice::from_ref(&content.pad),
			DevelopmentContent::ShepherdsVillage(content) => &content.pads,
		};
		pads.iter()
	}

	pub fn pad_complexes(&self) -> impl Iterator<Item = &PadComplex> {
		self.pads().map(|p| &p.complex)
	}

	pub fn les_halles(&self) -> Option<&LesHallesCell> {
		match &self.content {
			DevelopmentContent::LesHalles(content) => Some(content),
			_ => None,
		}
	}

	pub fn shepherds_village(&self) -> Option<&ShepherdsVillageCell> {
		match &self.content {
			DevelopmentContent::ShepherdsVillage(content) => Some(content),
			_ => None,
		}
	}

	/// Unrotated confines AABB sitting on the pad (world space).
	///
	/// Les Halles authors against this axis-aligned box. Its sampled yaw is
	/// recorded on [`Confines::roll`] and applied at host spawn about the cell center.
	/// Label wireframes fill the AABB with identity local yaw so they inherit that pose.
	pub fn confines_bounds(&self) -> Option<Aabb3d> {
		let content = self.les_halles()?;
		let c = Vec2::new(
			(self.cell.min.x + self.cell.max.x) * 0.5,
			(self.cell.min.z + self.cell.max.z) * 0.5,
		);
		let hx = content.confines_extent_xz.x * 0.5;
		let hz = content.confines_extent_xz.y * 0.5;
		let y0 = content.pad.height;
		Some(Aabb3d::from_min_max(
			bevy::math::Vec3::new(c.x - hx, y0, c.y - hz),
			bevy::math::Vec3::new(c.x + hx, y0 + content.confines_height, c.y + hz),
		))
	}

	/// Fitted confines: unrotated AABB plus yaw on [`Confines::roll`].
	pub fn confines(&self) -> Option<Confines> {
		Some(Confines::new(
			self.confines_bounds()?,
			self.les_halles()?.confines_yaw,
			Openings::new(),
		))
	}

	pub fn with_les_halles(cell: Aabb3d, pad_height: f32, config: &DevelopmentConfig) -> Self {
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
			content: DevelopmentContent::LesHalles(LesHallesCell {
				pad: DevelopmentPad {
					height: pad_height,
					complex: PadComplex::building_skirt(
						cell_center_xz(cell),
						confines_extent_xz * 0.5,
						yaw,
						pad_height,
						PadParams::default(),
					),
				},
				confines_height,
				confines_extent_xz,
				confines_yaw: yaw,
				finish: DevelopmentFinish::pick(hash),
			}),
		}
	}

	pub fn with_shepherds_village(
		cell: Aabb3d,
		village: ShepherdsVillage,
		pads: Vec<DevelopmentPad>,
	) -> Self {
		Self {
			cell,
			content: DevelopmentContent::ShepherdsVillage(ShepherdsVillageCell { pads, village }),
		}
	}
}

pub fn select_kind(cell: Aabb3d, config: &DevelopmentConfig) -> DevelopmentKind {
	if !cell_selected(cell, config.occupancy_seed(), config.likelihood, config.spatial_correlation)
	{
		return DevelopmentKind::Empty;
	}
	let les_halles = config.les_halles_weight.max(0.0);
	let shepherds = config.shepherds_village_weight.max(0.0);
	let total = les_halles + shepherds;
	if total <= f32::EPSILON {
		return DevelopmentKind::Empty;
	}
	let hash = SeededHash::new(config.seed.wrapping_add(cell_salt(cell)));
	if hash.unit(44) * total < les_halles {
		DevelopmentKind::LesHalles
	} else {
		DevelopmentKind::ShepherdsVillage
	}
}

pub(crate) fn cell_salt(cell: Aabb3d) -> u32 {
	cell.min.x.to_bits().wrapping_mul(73856093) ^ cell.min.z.to_bits().wrapping_mul(19349663)
}

#[cfg(test)]
mod tests {
	use material_ref::MaterialId;
	use std::f32::consts::TAU;

	use super::*;
	use crate::cell::{available_footprint, yawed_plan_aabb_extent, DevelopmentExtent};

	#[test]
	fn filled_cell_picks_urban_finish() {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let filled = DevelopmentCell::with_les_halles(cell, 12.0, &DevelopmentConfig::default());
		let finish = &filled.les_halles().expect("Les Halles payload").finish;
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
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let eighth = TAU / 8.0;
		let mut off_grid = false;
		for seed in 0..48u32 {
			let config = DevelopmentConfig { seed, ..DevelopmentConfig::default() };
			let filled = DevelopmentCell::with_les_halles(cell, 12.0, &config);
			let les_halles = filled.les_halles().expect("Les Halles payload");
			assert!(les_halles.confines_yaw >= 0.0 && les_halles.confines_yaw <= TAU + 1e-5);
			let phase = les_halles.confines_yaw.rem_euclid(eighth);
			if phase > 0.05 && phase < eighth - 0.05 {
				off_grid = true;
			}
			let pad = available_footprint();
			let occupied = yawed_plan_aabb_extent(
				les_halles.confines_extent_xz.x,
				les_halles.confines_extent_xz.y,
				les_halles.confines_yaw,
			);
			assert!(occupied.x <= pad + 1e-3, "yawed AABB x {} exceeds pad {}", occupied.x, pad);
			assert!(occupied.y <= pad + 1e-3, "yawed AABB z {} exceeds pad {}", occupied.y, pad);
			let confines = filled.confines().expect("filled cell has confines");
			assert!((confines.roll - les_halles.confines_yaw).abs() < 1e-6);
		}
		assert!(off_grid, "expected at least one heading off the old π/4 lattice");
	}

	#[test]
	fn filled_cell_pad_flattens_the_building_center() {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let filled = DevelopmentCell::with_les_halles(cell, 12.0, &DevelopmentConfig::default());
		let pad = filled.pad_complex().expect("filled cell has a pad");
		let c = cell_center_xz(cell);
		assert!((pad.modify_elevation(3.0, c.x, c.y) - 12.0).abs() < 1e-3);
		assert!((pad.modify_elevation(3.0, 400.0, 400.0) - 3.0).abs() < 1e-3);
	}

	#[test]
	fn selected_kinds_respect_zero_weights() {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let les_halles = DevelopmentConfig {
			likelihood: 1.0,
			les_halles_weight: 1.0,
			shepherds_village_weight: 0.0,
			..DevelopmentConfig::default()
		};
		assert_eq!(select_kind(cell, &les_halles), DevelopmentKind::LesHalles);
		let shepherds = DevelopmentConfig {
			les_halles_weight: 0.0,
			shepherds_village_weight: 1.0,
			..les_halles
		};
		assert_eq!(select_kind(cell, &shepherds), DevelopmentKind::ShepherdsVillage);
	}
}
