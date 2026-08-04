//! Cardinal façade helpers for retail / office / restroom stall interiors.
//!
//! Distinct from bites packing ([`super::bites`]), which uses
//! [`procedural_common::PlanOpeningFace`] as the sole opening-face source of truth.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

use crate::bedroom::shell::face_rectangle;
use crate::constraints::FaceKind;
use crate::fit::{aabb_near_plane, aabb_xz_extent, Confines};
use crate::openings::{Opening, OpeningLabel};
use crate::paneling::Rectangle;
use crate::paneling::DEFAULT_PANEL_THICKNESS;

/// Plan cardinal used for façade / band placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StallSide {
	South,
	North,
	East,
	West,
}

impl StallSide {
	/// Longest connectable façade opening, or the longest plan edge if none.
	pub fn primary(confines: &Confines) -> (Self, f32) {
		let mut best: Option<(Self, f32)> = None;
		for (_id, opening) in confines.openings.iter() {
			if !matches!(
				opening.label,
				OpeningLabel::Passage | OpeningLabel::Aperture
			) {
				continue;
			}
			let Some(side) = Self::for_opening(&confines.bounds, opening) else {
				continue;
			};
			let len = side.opening_along_len(opening);
			if best.map(|(_, l)| len > l).unwrap_or(true) {
				best = Some((side, len));
			}
		}
		if let Some(b) = best {
			return b;
		}
		let e = aabb_xz_extent(&confines.bounds);
		if e.x >= e.y {
			(Self::South, e.x)
		} else {
			(Self::East, e.y)
		}
	}

	pub fn for_opening(bounds: &Aabb3d, opening: &Opening) -> Option<Self> {
		let bmin = Vec3::from(bounds.min);
		let bmax = Vec3::from(bounds.max);
		let omin = Vec3::from(opening.bounds.min);
		let omax = Vec3::from(opening.bounds.max);
		let oc = (omin + omax) * 0.5;
		let tol = 0.45_f32;
		let x_span = (omax.x - omin.x).abs();
		let z_span = (omax.z - omin.z).abs();
		let mut best: Option<(Self, f32)> = None;
		let mut consider = |side: Self, on_plane: bool, dist: f32, along: f32, through: f32| {
			if !on_plane || along + 1e-3 < through {
				return;
			}
			if best.map(|(_, d)| dist < d - 1e-4).unwrap_or(true) {
				best = Some((side, dist));
			}
		};
		consider(
			Self::South,
			aabb_near_plane(omin.z, omax.z, bmin.z, tol),
			(oc.z - bmin.z).abs(),
			x_span,
			z_span,
		);
		consider(
			Self::North,
			aabb_near_plane(omin.z, omax.z, bmax.z, tol),
			(oc.z - bmax.z).abs(),
			x_span,
			z_span,
		);
		consider(
			Self::East,
			aabb_near_plane(omin.x, omax.x, bmax.x, tol),
			(oc.x - bmax.x).abs(),
			z_span,
			x_span,
		);
		consider(
			Self::West,
			aabb_near_plane(omin.x, omax.x, bmin.x, tol),
			(oc.x - bmin.x).abs(),
			z_span,
			x_span,
		);
		best.map(|(side, _)| side)
	}

	pub fn opening_along_len(self, opening: &Opening) -> f32 {
		let omin = Vec3::from(opening.bounds.min);
		let omax = Vec3::from(opening.bounds.max);
		match self {
			Self::South | Self::North => (omax.x - omin.x).abs(),
			Self::East | Self::West => (omax.z - omin.z).abs(),
		}
	}

	/// Band along this side of depth `depth`, covering `cover` fraction (centered).
	pub fn facade_band(self, bounds: &Aabb3d, depth: f32, cover: f32) -> Aabb3d {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let cover = cover.clamp(0.35, 0.95);
		let depth = depth
			.min(match self {
				Self::South | Self::North => (max.z - min.z) * 0.45,
				Self::East | Self::West => (max.x - min.x) * 0.45,
			})
			.max(0.35);
		match self {
			Self::South => {
				let span = (max.x - min.x) * cover;
				let x0 = (min.x + max.x) * 0.5 - span * 0.5;
				Aabb3d::from_min_max(
					Vec3::new(x0, min.y, min.z),
					Vec3::new(x0 + span, max.y, min.z + depth),
				)
			}
			Self::North => {
				let span = (max.x - min.x) * cover;
				let x0 = (min.x + max.x) * 0.5 - span * 0.5;
				Aabb3d::from_min_max(
					Vec3::new(x0, min.y, max.z - depth),
					Vec3::new(x0 + span, max.y, max.z),
				)
			}
			Self::East => {
				let span = (max.z - min.z) * cover;
				let z0 = (min.z + max.z) * 0.5 - span * 0.5;
				Aabb3d::from_min_max(
					Vec3::new(max.x - depth, min.y, z0),
					Vec3::new(max.x, max.y, z0 + span),
				)
			}
			Self::West => {
				let span = (max.z - min.z) * cover;
				let z0 = (min.z + max.z) * 0.5 - span * 0.5;
				Aabb3d::from_min_max(
					Vec3::new(min.x, min.y, z0),
					Vec3::new(min.x + depth, max.y, z0 + span),
				)
			}
		}
	}

	/// Band inset from this side by `offset`, depth `depth`, full edge cover.
	pub fn inset_band(self, bounds: &Aabb3d, offset: f32, depth: f32) -> Aabb3d {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let depth = depth.max(0.3);
		match self {
			Self::South => {
				let z0 = (min.z + offset).min(max.z - depth);
				Aabb3d::from_min_max(
					Vec3::new(min.x, min.y, z0),
					Vec3::new(max.x, max.y, z0 + depth),
				)
			}
			Self::North => {
				let z1 = (max.z - offset).max(min.z + depth);
				Aabb3d::from_min_max(
					Vec3::new(min.x, min.y, z1 - depth),
					Vec3::new(max.x, max.y, z1),
				)
			}
			Self::East => {
				let x1 = (max.x - offset).max(min.x + depth);
				Aabb3d::from_min_max(
					Vec3::new(x1 - depth, min.y, min.z),
					Vec3::new(x1, max.y, max.z),
				)
			}
			Self::West => {
				let x0 = (min.x + offset).min(max.x - depth);
				Aabb3d::from_min_max(
					Vec3::new(x0, min.y, min.z),
					Vec3::new(x0 + depth, max.y, max.z),
				)
			}
		}
	}

	/// Back third of the footprint opposite this side (office residual).
	pub fn back_third(self, bounds: &Aabb3d) -> Aabb3d {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		match self {
			Self::South => {
				let z0 = min.z + (max.z - min.z) * (2.0 / 3.0);
				Aabb3d::from_min_max(Vec3::new(min.x, min.y, z0), max)
			}
			Self::North => {
				let z1 = min.z + (max.z - min.z) / 3.0;
				Aabb3d::from_min_max(min, Vec3::new(max.x, max.y, z1))
			}
			Self::East => {
				let x1 = min.x + (max.x - min.x) / 3.0;
				Aabb3d::from_min_max(min, Vec3::new(x1, max.y, max.z))
			}
			Self::West => {
				let x0 = min.x + (max.x - min.x) * (2.0 / 3.0);
				Aabb3d::from_min_max(Vec3::new(x0, min.y, min.z), max)
			}
		}
	}

	/// Thin divider wall between office (back third) and sales floor.
	pub fn office_divider_wall(self, bounds: &Aabb3d, office: &Aabb3d) -> Option<Rectangle> {
		let face = match self {
			Self::South => FaceKind::Back,
			Self::North => FaceKind::Front,
			Self::East => FaceKind::Left,
			Self::West => FaceKind::Right,
		};
		let omin = Vec3::from(office.min);
		let omax = Vec3::from(office.max);
		let bmin = Vec3::from(bounds.min);
		let bmax = Vec3::from(bounds.max);
		let divider = match self {
			Self::South => Aabb3d::from_min_max(
				Vec3::new(bmin.x, bmin.y, omin.z - 0.05),
				Vec3::new(bmax.x, bmax.y, omin.z + 0.05),
			),
			Self::North => Aabb3d::from_min_max(
				Vec3::new(bmin.x, bmin.y, omax.z - 0.05),
				Vec3::new(bmax.x, bmax.y, omax.z + 0.05),
			),
			Self::East => Aabb3d::from_min_max(
				Vec3::new(omax.x - 0.05, bmin.y, bmin.z),
				Vec3::new(omax.x + 0.05, bmax.y, bmax.z),
			),
			Self::West => Aabb3d::from_min_max(
				Vec3::new(omin.x - 0.05, bmin.y, bmin.z),
				Vec3::new(omin.x + 0.05, bmax.y, bmax.z),
			),
		};
		face_rectangle(&divider, face, DEFAULT_PANEL_THICKNESS)
	}

	/// Sales floor = bounds minus `office` (back third opposite the façade).
	pub fn sales_minus_office(self, bounds: &Aabb3d, office: &Aabb3d) -> Aabb3d {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let omin = Vec3::from(office.min);
		let omax = Vec3::from(office.max);
		match self {
			Self::South => Aabb3d::from_min_max(
				min,
				Vec3::new(max.x, max.y, omin.z.max(min.z + 0.4)),
			),
			Self::North => Aabb3d::from_min_max(
				Vec3::new(min.x, min.y, omax.z.min(max.z - 0.4)),
				max,
			),
			Self::East => Aabb3d::from_min_max(
				Vec3::new(omax.x.min(max.x - 0.4), min.y, min.z),
				max,
			),
			Self::West => Aabb3d::from_min_max(
				min,
				Vec3::new(omin.x.max(min.x + 0.4), max.y, max.z),
			),
		}
	}
}

/// Convenience: [`StallSide::primary`].
pub fn primary_facade(confines: &Confines) -> (StallSide, f32) {
	StallSide::primary(confines)
}
