//! Les Halles floor plan: gallery ring (walls + floor) and balcony floor ring.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError, StackRegion};
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::fitted_rectangle::FittedRectangle;
use crate::paneling::panel_complex::{PanelPoint, DEFAULT_PANEL_THICKNESS};
use crate::shells::ortho::{OrthoSide, PlanRect, EPS};
use crate::shells::rect_ring_floor::{
	RectRingFloor, RectRingFloorParams, RectRingFloorSide, RectRingFloorSlab,
};

use super::parameterized::{
	footprint_extents, LesHallesParameterized, LesHallesShaftPlacement, SHAFT_SIDE,
};
use super::SCOPE;

/// Structural Les Halles plan.
///
/// - **Gallery** — rectangular ring with outer facade walls, inner walls facing
///   the balcony, and a solid floor (commercial band). Ceiling is optional
///   ([`RectRingFloorSlab::None`] by default).
/// - **Balcony** — floor-only annulus between the gallery’s inner wall and the
///   courtyard (no walls).
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesFloorPlan {
	pub parameterized: LesHallesParameterized,
	/// Storey plan center; `y` is the floor elevation.
	pub center_xz: Vec3,
	/// Full outer footprint width (X) / depth (Z).
	pub outer: Vec2,
	/// Gallery inner / balcony outer — wall line between gallery and balcony.
	pub gallery_inner: Vec2,
	/// Open courtyard width (X) / depth (Z).
	pub courtyard: Vec2,
	pub storey_height: f32,
	pub roll: f32,
	/// Ceiling slab on the gallery ring ([`RectRingFloorSlab::None`] by default).
	pub ceiling: RectRingFloorSlab,
	/// Merged inbound + generated openings for the gallery shell.
	pub openings: Openings,
	/// Shaft volumes (same bounds as shaft openings / within cells).
	pub shaft_bounds: Vec<Aabb3d>,
	/// Walled gallery ring (outer + inner walls, floor, optional ceiling).
	pub gallery: RectRingFloor,
	/// Floor-only balcony annulus pieces (no walls).
	pub balcony_floors: Vec<FittedRectangle>,
}

impl LesHallesFloorPlan {
	/// Deterministic structure from already-sampled parameters (towering path).
	///
	/// Gallery ceiling defaults to [`RectRingFloorSlab::None`].
	pub fn from_parameterized(
		params: LesHallesParameterized,
		confines: &Confines,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_parameterized_with_ceiling(params, confines, RectRingFloorSlab::None)
	}

	/// Same as [`Self::from_parameterized`] with an explicit gallery ceiling.
	pub fn from_parameterized_with_ceiling(
		params: LesHallesParameterized,
		confines: &Confines,
		ceiling: RectRingFloorSlab,
	) -> Result<(Self, FillableRegions), FitError> {
		let (extent_x, extent_z, height) = footprint_extents(confines)?;
		let ring = params.ring_width();
		let courtyard_x = extent_x - 2.0 * ring;
		let courtyard_z = extent_z - 2.0 * ring;
		if courtyard_x < super::parameterized::MIN_COURTYARD
			|| courtyard_z < super::parameterized::MIN_COURTYARD
		{
			return Err(FitError::TooSmall { reason: "footprint" });
		}

		let center = confines.center();
		let y0 = confines.bounds.min.y;
		let center_xz = Vec3::new(center.x, y0, center.z);
		let outer = Vec2::new(extent_x, extent_z);
		let gallery_inner = Vec2::new(
			(extent_x - 2.0 * params.gallery_width).max(EPS),
			(extent_z - 2.0 * params.gallery_width).max(EPS),
		);
		let courtyard = Vec2::new(courtyard_x, courtyard_z);

		let shaft_bounds =
			Self::shaft_aabbs(center_xz, outer, params.gallery_width, height, params.shaft_placement);

		let mut openings = confines.openings.clone();
		openings.extend(&Self::generated_openings(
			center_xz,
			outer,
			gallery_inner,
			height,
			params.opening_density,
			&shaft_bounds,
		));

		let gallery = Self::build_gallery(center_xz, outer, gallery_inner, height, ceiling, &openings);
		let balcony_floors = Self::build_balcony_floors(center_xz, gallery_inner, courtyard, y0);

		let plan = Self {
			parameterized: params,
			center_xz,
			outer,
			gallery_inner,
			courtyard,
			storey_height: height,
			roll: confines.roll,
			ceiling,
			openings,
			shaft_bounds,
			gallery,
			balcony_floors,
		};

		let regions = plan.fillable_regions();
		Ok((plan, regions))
	}

	/// Residual gallery / balcony / shaft confines and stack region.
	pub fn fillable_regions(&self) -> FillableRegions {
		let y0 = self.center_xz.y;
		let y1 = y0 + self.storey_height;
		let ox0 = self.center_xz.x - self.outer.x * 0.5;
		let ox1 = self.center_xz.x + self.outer.x * 0.5;
		let oz0 = self.center_xz.z - self.outer.y * 0.5;
		let oz1 = self.center_xz.z + self.outer.y * 0.5;
		let gx = self.parameterized.gallery_width;
		let bx = self.parameterized.balcony_width;
		let ix0 = self.center_xz.x - self.courtyard.x * 0.5;
		let ix1 = self.center_xz.x + self.courtyard.x * 0.5;
		let iz0 = self.center_xz.z - self.courtyard.y * 0.5;
		let iz1 = self.center_xz.z + self.courtyard.y * 0.5;

		let mut within = Vec::new();

		// Gallery bands (outer commercial ring), four sides.
		within.push(Self::band_confine(ox0, ox1, oz0, oz0 + gx, y0, y1, self.roll));
		within.push(Self::band_confine(ox0, ox1, oz1 - gx, oz1, y0, y1, self.roll));
		within.push(Self::band_confine(ox1 - gx, ox1, oz0 + gx, oz1 - gx, y0, y1, self.roll));
		within.push(Self::band_confine(ox0, ox0 + gx, oz0 + gx, oz1 - gx, y0, y1, self.roll));

		// Balcony bands (inner walking ring), four sides.
		within.push(Self::band_confine(ix0 - bx, ix1 + bx, iz0 - bx, iz0, y0, y1, self.roll));
		within.push(Self::band_confine(ix0 - bx, ix1 + bx, iz1, iz1 + bx, y0, y1, self.roll));
		within.push(Self::band_confine(ix1, ix1 + bx, iz0, iz1, y0, y1, self.roll));
		within.push(Self::band_confine(ix0 - bx, ix0, iz0, iz1, y0, y1, self.roll));

		for (i, shaft) in self.shaft_bounds.iter().enumerate() {
			let mut openings = Openings::new();
			openings.insert(
				OpeningId::scoped(SCOPE, "shaft", i.to_string()),
				Opening::new(*shaft, OpeningLabel::Shaft),
			);
			within.push(Confines::new(*shaft, self.roll, openings));
		}

		let atop = vec![StackRegion {
			bounds: Aabb2d {
				min: Vec2::new(ox0, oz0),
				max: Vec2::new(ox1, oz1),
			},
			height: self.storey_height,
			roll: self.roll,
			openings: self.openings.clone(),
		}];

		FillableRegions { within, atop }
	}

	/// Build a residual [`Confines`] cell for one axis-aligned band of the ring.
	///
	/// The gallery and balcony are each split into four side bands (S/N/E/W).
	/// Those bands are not mesh geometry — they are the `within` fill slots that a
	/// later Full\* pass (shops, furniture, stairs) consumes. Empty `openings` here
	/// means “no extra voids inside this cell yet”; shaft cells carry their own
	/// shaft opening instead.
	fn band_confine(
		min_x: f32,
		max_x: f32,
		min_z: f32,
		max_z: f32,
		y0: f32,
		y1: f32,
		roll: f32,
	) -> Confines {
		Confines::new(
			Aabb3d::from_min_max(Vec3::new(min_x, y0, min_z), Vec3::new(max_x, y1, max_z)),
			roll,
			Openings::new(),
		)
	}

	/// Shaft AABBs in the gallery band (corners or mid-sides).
	fn shaft_aabbs(
		center_xz: Vec3,
		outer: Vec2,
		gallery_width: f32,
		height: f32,
		placement: LesHallesShaftPlacement,
	) -> Vec<Aabb3d> {
		let y0 = center_xz.y;
		let y1 = y0 + height;
		let half = SHAFT_SIDE * 0.5;
		let ox0 = center_xz.x - outer.x * 0.5;
		let ox1 = center_xz.x + outer.x * 0.5;
		let oz0 = center_xz.z - outer.y * 0.5;
		let oz1 = center_xz.z + outer.y * 0.5;
		let inset = gallery_width * 0.5;

		let centers: Vec<Vec2> = match placement {
			LesHallesShaftPlacement::Corners => vec![
				Vec2::new(ox0 + inset, oz0 + inset),
				Vec2::new(ox1 - inset, oz0 + inset),
				Vec2::new(ox1 - inset, oz1 - inset),
				Vec2::new(ox0 + inset, oz1 - inset),
			],
			LesHallesShaftPlacement::MidSides => vec![
				Vec2::new(center_xz.x, oz0 + inset),
				Vec2::new(ox1 - inset, center_xz.z),
				Vec2::new(center_xz.x, oz1 - inset),
				Vec2::new(ox0 + inset, center_xz.z),
			],
		};

		centers
			.into_iter()
			.map(|c| {
				Aabb3d::from_min_max(
					Vec3::new(c.x - half, y0, c.y - half),
					Vec3::new(c.x + half, y1, c.y + half),
				)
			})
			.collect()
	}

	/// Gallery facade + balcony-facing openings, plus shaft voids.
	///
	/// Uses [`RectRingFloor`]'s side helpers so AABBs land on the correct outer
	/// or inner wall edges. Density controls how many extra windows per outer side.
	fn generated_openings(
		center_xz: Vec3,
		outer: Vec2,
		gallery_inner: Vec2,
		height: f32,
		opening_density: f32,
		shaft_bounds: &[Aabb3d],
	) -> Openings {
		let mut openings = Openings::new();
		let door_h = (height * 0.72).clamp(2.0, height.max(2.0));
		let win_h = (height * 0.35).clamp(0.9, height.max(0.9));
		let sill = (height * 0.35).clamp(0.8, height * 0.5);
		let door_w = 1.4;
		let win_w = 1.2;
		// Extra outer windows beyond the first on each side (0..=2).
		let extra_wins = ((opening_density * 3.0).floor() as usize).min(2);

		for side in RectRingFloorSide::all() {
			let slot = side_slot(side);
			openings.insert(
				OpeningId::scoped(SCOPE, "outer_passage", slot),
				RectRingFloor::side_passage_opening(side, center_xz, outer, door_w, door_h),
			);
			openings.insert(
				OpeningId::scoped(SCOPE, "outer_aperture", format!("{slot}_0")),
				RectRingFloor::side_aperture_opening(side, center_xz, outer, win_w, win_h, sill),
			);
			for k in 0..extra_wins {
				// Offset extras along the side by shifting after helper construction.
				let mut opening = RectRingFloor::side_aperture_opening(
					side,
					center_xz,
					outer,
					win_w,
					win_h,
					sill,
				);
				opening.bounds = offset_opening_along_side(opening.bounds, side, (k + 1) as f32 * 2.2);
				openings.insert(
					OpeningId::scoped(SCOPE, "outer_aperture", format!("{slot}_{}", k + 1)),
					opening,
				);
			}

			// Balcony → shop doors on the gallery’s inner wall.
			openings.insert(
				OpeningId::scoped(SCOPE, "inner_passage", slot),
				RectRingFloor::side_passage_opening(side, center_xz, gallery_inner, door_w, door_h),
			);
			openings.insert(
				OpeningId::scoped(SCOPE, "inner_aperture", slot),
				RectRingFloor::side_aperture_opening(
					side,
					center_xz,
					gallery_inner,
					win_w,
					win_h,
					sill,
				),
			);
		}

		for (i, shaft) in shaft_bounds.iter().enumerate() {
			openings.insert(
				OpeningId::scoped(SCOPE, "shaft", format!("{i}")),
				Opening::new(*shaft, OpeningLabel::Shaft),
			);
		}

		openings
	}

	fn build_gallery(
		center_xz: Vec3,
		outer: Vec2,
		gallery_inner: Vec2,
		storey_height: f32,
		ceiling: RectRingFloorSlab,
		openings: &Openings,
	) -> RectRingFloor {
		RectRingFloor::new(
			RectRingFloorParams::new(center_xz, outer, gallery_inner, storey_height)
				.floor(RectRingFloorSlab::Solid)
				.ceiling(ceiling)
				.openings(openings.clone()),
		)
	}

	/// Floor-only balcony annulus between `gallery_inner` and `courtyard`.
	fn build_balcony_floors(
		center_xz: Vec3,
		gallery_inner: Vec2,
		courtyard: Vec2,
		y0: f32,
	) -> Vec<FittedRectangle> {
		let gx0 = center_xz.x - gallery_inner.x * 0.5;
		let gx1 = center_xz.x + gallery_inner.x * 0.5;
		let gz0 = center_xz.z - gallery_inner.y * 0.5;
		let gz1 = center_xz.z + gallery_inner.y * 0.5;
		let cx0 = center_xz.x - courtyard.x * 0.5;
		let cx1 = center_xz.x + courtyard.x * 0.5;
		let cz0 = center_xz.z - courtyard.y * 0.5;
		let cz1 = center_xz.z + courtyard.y * 0.5;
		let t = DEFAULT_PANEL_THICKNESS;
		let style = PanelStyle::RoughStonework;

		let mut out = Vec::new();
		// North / South take full gallery-inner width; East / West take courtyard depth only.
		if gz1 - cz1 > EPS {
			out.push(floor_rect(style, gx0, gx1, cz1, gz1, y0, t));
		}
		if cz0 - gz0 > EPS {
			out.push(floor_rect(style, gx0, gx1, gz0, cz0, y0, t));
		}
		if cx0 - gx0 > EPS {
			out.push(floor_rect(style, gx0, cx0, cz0, cz1, y0, t));
		}
		if gx1 - cx1 > EPS {
			out.push(floor_rect(style, cx1, gx1, cz0, cz1, y0, t));
		}
		out
	}
}

impl Fit for LesHallesFloorPlan {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = LesHallesParameterized::sample(confines, noise)?;
		Self::from_parameterized(params, confines)
	}
}

impl BuildingComponents for LesHallesFloorPlan {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.gallery.panel_nodes_for_level(level);
		for floor in &self.balcony_floors {
			out.extend(floor.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.gallery.joint_nodes_for_level(level)
	}
}

fn side_slot(side: OrthoSide) -> &'static str {
	match side {
		OrthoSide::South => "s",
		OrthoSide::East => "e",
		OrthoSide::North => "n",
		OrthoSide::West => "w",
	}
}

fn offset_opening_along_side(bounds: Aabb3d, side: OrthoSide, delta: f32) -> Aabb3d {
	let (dx, dz) = match side {
		OrthoSide::North | OrthoSide::South => (delta, 0.0),
		OrthoSide::East | OrthoSide::West => (0.0, delta),
	};
	let min = Vec3::from(bounds.min) + Vec3::new(dx, 0.0, dz);
	let max = Vec3::from(bounds.max) + Vec3::new(dx, 0.0, dz);
	Aabb3d::from_min_max(min, max)
}

fn floor_rect(
	style: PanelStyle,
	min_x: f32,
	max_x: f32,
	min_z: f32,
	max_z: f32,
	y: f32,
	thickness: f32,
) -> FittedRectangle {
	let plan = PlanRect::new(
		Vec3::new(0.5 * (min_x + max_x), y, 0.5 * (min_z + max_z)),
		(max_x - min_x).max(EPS),
		(max_z - min_z).max(EPS),
	);
	let t = thickness.max(1e-4);
	FittedRectangle::new(
		style,
		PanelPoint::new(plan.sw(), t),
		PanelPoint::new(plan.se(), t),
		PanelPoint::new(plan.nw(), t),
		PanelPoint::new(plan.ne(), t),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::fit::Fit;
	use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;

	fn nominal_confines() -> Confines {
		Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-10.0, 0.0, -8.0),
			Vec3::new(10.0, 3.5, 8.0),
		))
	}

	fn tiny_confines() -> Confines {
		Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(3.0, 3.0, 3.0),
		))
	}

	#[test]
	fn too_small_footprint_rejects() {
		let err = LesHallesFloorPlan::fit_to_confines(&tiny_confines(), NoiseParams::default())
			.expect_err("tiny footprint");
		assert!(matches!(err, FitError::TooSmall { reason: "footprint" }));
	}

	#[test]
	fn sample_is_deterministic() {
		let c = nominal_confines();
		let noise = NoiseParams {
			seed: 42,
			..NoiseParams::default()
		};
		let a = LesHallesParameterized::sample(&c, noise).unwrap();
		let b = LesHallesParameterized::sample(&c, noise).unwrap();
		assert_eq!(a.shaft_placement, b.shaft_placement);
		assert!((a.gallery_width - b.gallery_width).abs() < 1e-5);
		assert!((a.balcony_width - b.balcony_width).abs() < 1e-5);
	}

	#[test]
	fn gallery_has_no_ceiling_by_default_and_balcony_has_floors() {
		let (plan, _) =
			LesHallesFloorPlan::fit_to_confines(&nominal_confines(), NoiseParams::default()).unwrap();
		assert!(!plan.gallery.has_ceiling());
		assert!(plan.gallery.has_floor());
		assert!(!plan.balcony_floors.is_empty());
		assert!(plan.gallery_inner.x > plan.courtyard.x);
		assert!(plan.outer.x > plan.gallery_inner.x);
	}

	#[test]
	fn emits_outer_and_inner_gallery_openings() {
		let (plan, _) =
			LesHallesFloorPlan::fit_to_confines(&nominal_confines(), NoiseParams::default()).unwrap();
		assert!(plan
			.openings
			.get(&OpeningId::scoped(SCOPE, "outer_passage", "s"))
			.is_some());
		assert!(plan
			.openings
			.get(&OpeningId::scoped(SCOPE, "inner_passage", "n"))
			.is_some());
		assert!(plan
			.openings
			.get(&OpeningId::scoped(SCOPE, "outer_aperture", "e_0"))
			.is_some());
		assert!(plan
			.openings
			.get(&OpeningId::scoped(SCOPE, "inner_aperture", "w"))
			.is_some());
	}

	#[test]
	fn preserves_inbound_opening_ids_and_emits_scoped_shafts() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("inbound_door"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(-0.5, 0.0, -8.2),
				Vec3::new(0.5, 2.2, -7.8),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::new(-10.0, 0.0, -8.0), Vec3::new(10.0, 3.5, 8.0)),
			0.0,
			openings,
		);
		let (plan, regions) =
			LesHallesFloorPlan::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(plan.openings.get(&OpeningId::new("inbound_door")).is_some());
		assert!(plan
			.openings
			.get(&OpeningId::scoped(SCOPE, "shaft", "0"))
			.is_some());
		assert!(matches!(
			plan.openings
				.get(&OpeningId::scoped(SCOPE, "shaft", "0"))
				.unwrap()
				.label,
			OpeningLabel::Shaft
		));
		assert!(plan.gallery.wall_count() >= 4);
		assert!(regions.within.len() >= 8 + 4);
		assert_eq!(regions.atop.len(), 1);
	}

	#[test]
	fn from_parameterized_matches_fit_structure() {
		let c = nominal_confines();
		let noise = NoiseParams {
			seed: 7,
			..NoiseParams::default()
		};
		let params = LesHallesParameterized::sample(&c, noise).unwrap();
		let (via_fit, _) = LesHallesFloorPlan::fit_to_confines(&c, noise).unwrap();
		let (via_params, _) = LesHallesFloorPlan::from_parameterized(params, &c).unwrap();
		assert_eq!(via_fit.outer, via_params.outer);
		assert_eq!(via_fit.gallery_inner, via_params.gallery_inner);
		assert_eq!(via_fit.courtyard, via_params.courtyard);
		assert_eq!(
			via_fit.parameterized.shaft_placement,
			via_params.parameterized.shaft_placement
		);
		assert_eq!(via_fit.shaft_bounds.len(), 4);
	}
}
