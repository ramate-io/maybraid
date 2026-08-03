//! Commercial stall: boundary shell + interior subtype fill.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::bedroom::shell::face_rectangle;
use crate::constraints::FaceKind;
use crate::fit::{aabb_near_plane, aabb_xz_overlap_area, Confines, FillableRegions, Fit, FitError};
use crate::openings::{OpeningLabel, Openings};
use crate::paneling::Rectangle;
use crate::paneling::DEFAULT_PANEL_THICKNESS;
use crate::usage_areas::commercial_stall_interior::CommercialStallInterior;

const MIN_STALL: f32 = 1.2;

/// Noise knobs for a commercial stall shell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommercialStallParameterized {
	pub style: LabelStyle,
}

impl CommercialStallParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Self {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let t = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 11.0);
		Self {
			style: LabelStyle::from_unit(t),
		}
	}
}

/// Stall plan: walls + interior.
#[derive(Debug, Clone, PartialEq)]
pub struct CommercialStallPlan {
	pub parameterized: CommercialStallParameterized,
	pub walls: Vec<Rectangle>,
	pub interior: CommercialStallInterior,
}

impl CommercialStallPlan {
	pub fn from_parameterized(
		params: CommercialStallParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<Self, FitError> {
		let min = Vec3::from(confines.bounds.min);
		let max = Vec3::from(confines.bounds.max);
		let extent = (max - min).max(Vec3::splat(1e-4));
		if extent.x < MIN_STALL || extent.z < MIN_STALL || extent.y < 0.8 {
			return Err(FitError::TooSmall { reason: "stall" });
		}
		let walls = shell_walls(confines);
		let interior_confines = Confines::new(
			confines.bounds,
			confines.roll,
			forward_openings(&confines.openings),
		);
		let (interior, _) =
			CommercialStallInterior::fit_to_confines(&interior_confines, noise)?;
		Ok(Self {
			parameterized: params,
			walls,
			interior,
		})
	}
}

/// Full commercial stall.
#[derive(Debug, Clone, PartialEq)]
pub struct CommercialStall {
	pub plan: CommercialStallPlan,
}

impl CommercialStall {
	pub fn from_plan(plan: CommercialStallPlan) -> Self {
		Self { plan }
	}

	pub fn interior(&self) -> &CommercialStallInterior {
		&self.plan.interior
	}
}

impl Fit for CommercialStall {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = CommercialStallParameterized::sample(confines, noise);
		let plan = CommercialStallPlan::from_parameterized(params, confines, noise)?;
		Ok((Self::from_plan(plan), FillableRegions::empty()))
	}
}

impl BuildingComponents for CommercialStall {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for wall in &self.plan.walls {
			out.extend(wall.panel_nodes_for_level(level));
		}
		out.extend(self.plan.interior.panel_nodes_for_level(level));
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		self.plan.interior.label_nodes_for_level(level)
	}
}

fn shell_walls(confines: &Confines) -> Vec<Rectangle> {
	let mut walls = Vec::new();
	for face in [
		FaceKind::Front,
		FaceKind::Back,
		FaceKind::Left,
		FaceKind::Right,
	] {
		if face_excluded(&confines.bounds, face, &confines.openings) {
			continue;
		}
		if let Some(wall) = face_rectangle(&confines.bounds, face, DEFAULT_PANEL_THICKNESS) {
			walls.push(wall);
		}
	}
	walls
}

fn face_excluded(bounds: &Aabb3d, face: FaceKind, openings: &Openings) -> bool {
	for (_id, opening) in openings.iter() {
		if !matches!(
			opening.label,
			OpeningLabel::Exclusion | OpeningLabel::Boundary
		) {
			continue;
		}
		if opening_covers_face(bounds, face, &opening.bounds) {
			return true;
		}
	}
	false
}

fn opening_covers_face(bounds: &Aabb3d, face: FaceKind, opening: &Aabb3d) -> bool {
	let bmin = Vec3::from(bounds.min);
	let bmax = Vec3::from(bounds.max);
	let omin = Vec3::from(opening.min);
	let omax = Vec3::from(opening.max);
	let tol = 0.4_f32;
	let on_plane = match face {
		FaceKind::Front => aabb_near_plane(omin.z, omax.z, bmin.z, tol),
		FaceKind::Back => aabb_near_plane(omin.z, omax.z, bmax.z, tol),
		FaceKind::Right => aabb_near_plane(omin.x, omax.x, bmax.x, tol),
		FaceKind::Left => aabb_near_plane(omin.x, omax.x, bmin.x, tol),
		FaceKind::Top | FaceKind::Bottom => return false,
	};
	if !on_plane {
		return false;
	}
	// Require substantial along-face coverage (≥55% of face length).
	let face_region = match face {
		FaceKind::Front | FaceKind::Back => bevy_math::bounding::Aabb2d {
			min: bevy_math::Vec2::new(bmin.x, bmin.z - 0.5),
			max: bevy_math::Vec2::new(bmax.x, bmax.z + 0.5),
		},
		FaceKind::Left | FaceKind::Right => bevy_math::bounding::Aabb2d {
			min: bevy_math::Vec2::new(bmin.x - 0.5, bmin.z),
			max: bevy_math::Vec2::new(bmax.x + 0.5, bmax.z),
		},
		_ => return false,
	};
	let face_len = match face {
		FaceKind::Front | FaceKind::Back => (bmax.x - bmin.x).max(1e-3),
		FaceKind::Left | FaceKind::Right => (bmax.z - bmin.z).max(1e-3),
		_ => 1.0,
	};
	let overlap = aabb_xz_overlap_area(opening, &face_region);
	// Rough length proxy from overlap area / thickness band.
	overlap >= face_len * 0.55 * 0.5
}

fn forward_openings(openings: &Openings) -> Openings {
	let mut out = Openings::new();
	for (id, opening) in openings.iter() {
		if matches!(
			opening.label,
			OpeningLabel::Passage | OpeningLabel::Aperture
		) {
			out.insert(id.clone(), opening.clone());
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use crate::openings::{Opening, OpeningId};

	#[test]
	fn stall_fit_emits_walls_and_interior_labels() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(1.0, 0.0, -0.2),
				Vec3::new(2.5, 2.2, 0.2),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 5.0)),
			0.0,
			openings,
		);
		let (stall, regions) =
			CommercialStall::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(regions.within.is_empty());
		assert_eq!(stall.plan.walls.len(), 4);
		assert!(!stall
			.label_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.is_empty());
	}

	#[test]
	fn exclusion_skips_wall_face() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("open_front"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(0.0, 0.0, -0.3), Vec3::new(4.0, 3.0, 0.3)),
				OpeningLabel::Exclusion,
			),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 5.0)),
			0.0,
			openings,
		);
		let (stall, _) =
			CommercialStall::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(stall.plan.walls.len() < 4);
	}
}
