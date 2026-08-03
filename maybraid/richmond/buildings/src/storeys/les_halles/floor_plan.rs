//! Les Halles floor plan: gallery ring (walls + floor) and balcony floor ring.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError, StackRegion};
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::fitted_rectangle::FittedRectangle;
use crate::paneling::panel_complex::{PanelPoint, DEFAULT_PANEL_THICKNESS};
use crate::paneling::rectangle::Rectangle;
use crate::shells::ortho::{OrthoSide, PlanRect, EPS};
use crate::shells::rect_ring_floor::{
	RectRingFloor, RectRingFloorParams, RectRingFloorSide, RectRingFloorSlab,
};

use super::parameterized::{
	footprint_extents, LesHallesParameterized, LesHallesShaftPlacement, LesHallesStallDoor,
	SHAFT_SIDE,
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
	///
	/// Inbound [`OpeningLabel::Shaft`] openings keep their ids; their bounds are
	/// rewritten onto a fitted shaft via [`Self::map_inbound_shafts`].
	pub openings: Openings,
	/// Shaft volumes (same bounds as shaft openings / within cells).
	pub shaft_bounds: Vec<Aabb3d>,
	/// Inbound shaft [`OpeningId`]s remapped onto each [`Self::shaft_bounds`] slot.
	pub shaft_inbound: Vec<Vec<OpeningId>>,
	/// Walled gallery ring (outer + inner walls, floor, optional ceiling).
	pub gallery: RectRingFloor,
	/// Floor-only balcony annulus pieces (no walls).
	pub balcony_floors: Vec<FittedRectangle>,
	/// Radial walls sealing each shaft from the gallery (inner wall → outer wall).
	pub shaft_walls: Vec<Rectangle>,
}

impl LesHallesFloorPlan {
	/// Build the stall-door size catalog for [`LesHallesParameterized::doors`].
	///
	/// Prefers larger shop openings; noise perturbs widths / jambs slightly.
	/// [`LesHallesParameterized::fit_doors_on_run`] walks this list in order.
	pub fn generate_stall_doors(cfg: &NoiseConfig, center: Vec3) -> Vec<LesHallesStallDoor> {
		// Base catalog: many large stalls, then mid / small fallbacks.
		let bases: [(f32, f32, f32); 8] = [
			(4.2, 0.3, 0.4),
			(3.6, 0.28, 0.35),
			(3.2, 0.25, 0.3),
			(2.8, 0.25, 0.3),
			(2.4, 0.22, 0.25),
			(2.0, 0.2, 0.25),
			(1.7, 0.18, 0.2),
			(1.4, 0.15, 0.2),
		];
		bases
			.into_iter()
			.enumerate()
			.map(|(i, (w, j, e))| {
				let salt = 5.0 + i as f32;
				let dw = cfg.sample_range_f32_4d(
					(w - 0.25).max(1.2),
					w + 0.35,
					center.x,
					center.y,
					center.z,
					salt,
				);
				let jamb = cfg.sample_range_f32_4d(
					(j - 0.05).max(0.1),
					j + 0.1,
					center.x,
					center.y,
					center.z,
					salt + 0.5,
				);
				LesHallesStallDoor {
					door_width: dw,
					jamb_min: jamb,
					allowed_error: e,
				}
			})
			.collect()
	}

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
		let shaft_inbound = Self::map_inbound_shafts(
			&mut openings,
			center_xz,
			outer,
			params.shaft_placement,
			&shaft_bounds,
		);
		openings.extend(&Self::generated_openings(
			&params,
			center_xz,
			outer,
			gallery_inner,
			height,
			&shaft_bounds,
		));

		let gallery = Self::build_gallery(center_xz, outer, gallery_inner, height, ceiling, &openings);
		let balcony_floors = Self::build_balcony_floors(center_xz, gallery_inner, courtyard, y0);
		let shaft_walls =
			Self::build_shaft_walls(center_xz, outer, gallery_inner, height, &shaft_bounds);

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
			shaft_inbound,
			gallery,
			balcony_floors,
			shaft_walls,
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
			if let Some(ids) = self.shaft_inbound.get(i) {
				for id in ids {
					openings.insert(id.clone(), Opening::new(*shaft, OpeningLabel::Shaft));
				}
			}
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

	/// Shaft AABBs spanning gallery depth toward the inner wall.
	///
	/// Inset from the outer facade so the outer wall stays solid behind each
	/// shaft ([`OpeningLabel::Shaft`] is connectable and would otherwise punch
	/// that face).
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
		let gw = gallery_width.max(EPS);
		// Keep outer facade solid behind the shaft.
		let outer_inset = (DEFAULT_PANEL_THICKNESS + 0.15).min(gw * 0.35);

		match placement {
			// Corner gallery cells: reach both abutting inner walls, leave outer faces.
			LesHallesShaftPlacement::Corners => vec![
				Aabb3d::from_min_max(
					Vec3::new(ox0 + outer_inset, y0, oz0 + outer_inset),
					Vec3::new(ox0 + gw, y1, oz0 + gw),
				),
				Aabb3d::from_min_max(
					Vec3::new(ox1 - gw, y0, oz0 + outer_inset),
					Vec3::new(ox1 - outer_inset, y1, oz0 + gw),
				),
				Aabb3d::from_min_max(
					Vec3::new(ox1 - gw, y0, oz1 - gw),
					Vec3::new(ox1 - outer_inset, y1, oz1 - outer_inset),
				),
				Aabb3d::from_min_max(
					Vec3::new(ox0 + outer_inset, y0, oz1 - gw),
					Vec3::new(ox0 + gw, y1, oz1 - outer_inset),
				),
			],
			// Mid-side: full gallery depth minus outer inset, `SHAFT_SIDE` along wall.
			LesHallesShaftPlacement::MidSides => vec![
				Aabb3d::from_min_max(
					Vec3::new(center_xz.x - half, y0, oz0 + outer_inset),
					Vec3::new(center_xz.x + half, y1, oz0 + gw),
				),
				Aabb3d::from_min_max(
					Vec3::new(ox1 - gw, y0, center_xz.z - half),
					Vec3::new(ox1 - outer_inset, y1, center_xz.z + half),
				),
				Aabb3d::from_min_max(
					Vec3::new(center_xz.x - half, y0, oz1 - gw),
					Vec3::new(center_xz.x + half, y1, oz1 - outer_inset),
				),
				Aabb3d::from_min_max(
					Vec3::new(ox0 + outer_inset, y0, center_xz.z - half),
					Vec3::new(ox0 + gw, y1, center_xz.z + half),
				),
			],
		}
	}

	/// Plan-space (XZ) regions that claim inbound shafts for each fitted slot.
	///
	/// - **Corners:** four quadrants about the footprint center (SW, SE, NE, NW).
	/// - **MidSides:** N/S end bands (full width) and E/W middle bands (half width),
	///   an exclusive partition of the outer footprint.
	///
	/// Region order matches [`Self::shaft_aabbs`].
	fn shaft_mapping_regions(
		center_xz: Vec3,
		outer: Vec2,
		placement: LesHallesShaftPlacement,
	) -> Vec<Aabb2d> {
		let ox0 = center_xz.x - outer.x * 0.5;
		let ox1 = center_xz.x + outer.x * 0.5;
		let oz0 = center_xz.z - outer.y * 0.5;
		let oz1 = center_xz.z + outer.y * 0.5;
		let cx = center_xz.x;
		let cz = center_xz.z;

		match placement {
			LesHallesShaftPlacement::Corners => vec![
				// SW, SE, NE, NW
				Aabb2d {
					min: Vec2::new(ox0, oz0),
					max: Vec2::new(cx, cz),
				},
				Aabb2d {
					min: Vec2::new(cx, oz0),
					max: Vec2::new(ox1, cz),
				},
				Aabb2d {
					min: Vec2::new(cx, cz),
					max: Vec2::new(ox1, oz1),
				},
				Aabb2d {
					min: Vec2::new(ox0, cz),
					max: Vec2::new(cx, oz1),
				},
			],
			LesHallesShaftPlacement::MidSides => {
				// End thirds (N/S, full X) + middle third split E/W.
				let z_lo = oz0 + (oz1 - oz0) / 3.0;
				let z_hi = oz1 - (oz1 - oz0) / 3.0;
				vec![
					// S, E, N, W
					Aabb2d {
						min: Vec2::new(ox0, oz0),
						max: Vec2::new(ox1, z_lo),
					},
					Aabb2d {
						min: Vec2::new(cx, z_lo),
						max: Vec2::new(ox1, z_hi),
					},
					Aabb2d {
						min: Vec2::new(ox0, z_hi),
						max: Vec2::new(ox1, oz1),
					},
					Aabb2d {
						min: Vec2::new(ox0, z_lo),
						max: Vec2::new(cx, z_hi),
					},
				]
			}
		}
	}

	/// Rewrite inbound [`OpeningLabel::Shaft`] bounds onto fitted shaft slots.
	///
	/// Picks the mapping region with greatest XZ overlap; ties / no-overlap fall
	/// back to the region whose center is closest to the request center. Returns
	/// inbound ids per `shaft_bounds` index (empty slots stay empty).
	pub fn map_inbound_shafts(
		openings: &mut Openings,
		center_xz: Vec3,
		outer: Vec2,
		placement: LesHallesShaftPlacement,
		shaft_bounds: &[Aabb3d],
	) -> Vec<Vec<OpeningId>> {
		let regions = Self::shaft_mapping_regions(center_xz, outer, placement);
		debug_assert_eq!(regions.len(), shaft_bounds.len());
		let mut inbound: Vec<Vec<OpeningId>> = (0..shaft_bounds.len()).map(|_| Vec::new()).collect();

		let shaft_ids: Vec<OpeningId> = openings
			.iter()
			.filter(|(_, o)| matches!(o.label, OpeningLabel::Shaft))
			.map(|(id, _)| id.clone())
			.collect();

		for id in shaft_ids {
			let Some(opening) = openings.openings.get_mut(&id) else {
				continue;
			};
			let Some(slot) = best_shaft_slot(&opening.bounds, &regions) else {
				continue;
			};
			opening.bounds = shaft_bounds[slot];
			inbound[slot].push(id);
		}
		inbound
	}

	/// Gallery facade + balcony-facing openings, plus shaft voids / clears.
	///
	/// - **Outer walls:** apertures only (no doors); skipped behind shafts.
	/// - **Inner walls:** stall doors only (packed per straight section), plus
	///   floor-to-ceiling shaft clears.
	fn generated_openings(
		params: &LesHallesParameterized,
		center_xz: Vec3,
		outer: Vec2,
		gallery_inner: Vec2,
		height: f32,
		shaft_bounds: &[Aabb3d],
	) -> Openings {
		let mut openings = Openings::new();
		let door_h = (height * 0.72).clamp(2.0, height.max(2.0));
		let win_h = (height * 0.4).clamp(1.0, height.max(1.0));
		let sill = (height * 0.3).clamp(0.7, height * 0.45);
		let win_w = 1.4;
		let extra_wins = ((params.opening_density * 3.0).floor() as usize).min(2);

		for side in RectRingFloorSide::all() {
			let slot = side_slot(side);
			let shaft_spans = Self::shaft_along_spans(center_xz, outer, side, shaft_bounds);

			// Outer facade: apertures only; leave solid wall behind shafts.
			let mut outer_offsets = vec![-2.8_f32, 2.8];
			for k in 0..extra_wins {
				outer_offsets.push(2.8 + (k + 1) as f32 * 2.4);
				outer_offsets.push(-2.8 - (k + 1) as f32 * 2.4);
			}
			let mut placed = 0_usize;
			for along in outer_offsets {
				if along_overlaps_spans(along, win_w, &shaft_spans) {
					continue;
				}
				let mut opening = RectRingFloor::side_aperture_opening(
					side,
					center_xz,
					outer,
					win_w,
					win_h,
					sill,
				);
				opening.bounds = offset_opening_along_side(opening.bounds, side, along);
				openings.insert(
					OpeningId::scoped(SCOPE, "outer_aperture", format!("{slot}_{placed}")),
					opening,
				);
				placed += 1;
			}
		}

		// Inner stall doors on each straight section between shaft clears.
		let sections = Self::inner_straight_sections(center_xz, gallery_inner, shaft_bounds);
		let expected = params.expected_inner_section_count();
		debug_assert_eq!(
			sections.len(),
			expected,
			"inner straight sections: got {} expected {} ({:?})",
			sections.len(),
			expected,
			params.shaft_placement
		);
		assert!(
			sections.len() == expected,
			"Les Halles inner wall: expected {expected} straight sections for {:?}, got {}",
			params.shaft_placement,
			sections.len()
		);

		for (si, section) in sections.iter().enumerate() {
			let run = (section.along1 - section.along0).max(0.0);
			let placed = params.fit_doors_on_run(run);
			assert!(
				!placed.is_empty(),
				"Les Halles inner wall section {si} ({:?}) has no door on run {run:.2}",
				section.side
			);
			for (di, door) in placed.iter().enumerate() {
				let along_mid = section.along0 + door.along + door.width * 0.5;
				let mut opening = RectRingFloor::side_passage_opening(
					section.side,
					center_xz,
					gallery_inner,
					door.width,
					door_h,
				);
				opening.bounds = offset_opening_along_side(opening.bounds, section.side, along_mid);
				openings.insert(
					OpeningId::scoped(SCOPE, "inner_door", format!("{si}_{di}")),
					opening,
				);
			}
		}

		for (i, shaft) in shaft_bounds.iter().enumerate() {
			openings.insert(
				OpeningId::scoped(SCOPE, "shaft", format!("{i}")),
				Opening::new(*shaft, OpeningLabel::Shaft),
			);
			let smin = Vec3::from(shaft.min);
			let smax = Vec3::from(shaft.max);
			for (side, along) in Self::shaft_inner_sides(center_xz, gallery_inner, *shaft) {
				let clear_w = match side {
					OrthoSide::North | OrthoSide::South => (smax.x - smin.x).max(1.2),
					OrthoSide::East | OrthoSide::West => (smax.z - smin.z).max(1.2),
				};
				let mut clear = RectRingFloor::side_passage_opening(
					side,
					center_xz,
					gallery_inner,
					clear_w,
					height,
				);
				clear.bounds = offset_opening_along_side(clear.bounds, side, along);
				openings.insert(
					OpeningId::scoped(SCOPE, "shaft_clear", format!("{i}_{}", side_slot(side))),
					clear,
				);
			}
		}

		openings
	}

	/// Free inner-wall runs between shaft clears (one section per run).
	fn inner_straight_sections(
		center_xz: Vec3,
		gallery_inner: Vec2,
		shaft_bounds: &[Aabb3d],
	) -> Vec<InnerSection> {
		let mut sections = Vec::new();
		for side in RectRingFloorSide::all() {
			let half = match side {
				OrthoSide::North | OrthoSide::South => gallery_inner.x * 0.5,
				OrthoSide::East | OrthoSide::West => gallery_inner.y * 0.5,
			};
			let mut occupied =
				Self::shaft_along_spans(center_xz, gallery_inner, side, shaft_bounds);
			occupied.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
			let mut cursor = -half;
			for (lo, hi) in occupied {
				let lo = lo.clamp(-half, half);
				let hi = hi.clamp(-half, half);
				if lo > cursor + 0.4 {
					sections.push(InnerSection {
						side,
						along0: cursor,
						along1: lo,
					});
				}
				cursor = cursor.max(hi);
			}
			if half > cursor + 0.4 {
				sections.push(InnerSection {
					side,
					along0: cursor,
					along1: half,
				});
			}
		}
		sections
	}

	/// Along-wall spans (relative to side midpoint) occupied by shafts on an outer side.
	fn shaft_along_spans(
		center_xz: Vec3,
		outer: Vec2,
		side: OrthoSide,
		shaft_bounds: &[Aabb3d],
	) -> Vec<(f32, f32)> {
		let ox0 = center_xz.x - outer.x * 0.5;
		let ox1 = center_xz.x + outer.x * 0.5;
		let oz0 = center_xz.z - outer.y * 0.5;
		let oz1 = center_xz.z + outer.y * 0.5;
		let band = 1.0_f32;
		let mut spans = Vec::new();
		for shaft in shaft_bounds {
			let smin = Vec3::from(shaft.min);
			let smax = Vec3::from(shaft.max);
			let near = match side {
				OrthoSide::South => smin.z <= oz0 + band,
				OrthoSide::North => smax.z >= oz1 - band,
				OrthoSide::East => smax.x >= ox1 - band,
				OrthoSide::West => smin.x <= ox0 + band,
			};
			if !near {
				continue;
			}
			let (lo, hi) = match side {
				OrthoSide::North | OrthoSide::South => {
					(smin.x - center_xz.x, smax.x - center_xz.x)
				}
				OrthoSide::East | OrthoSide::West => {
					(smin.z - center_xz.z, smax.z - center_xz.z)
				}
			};
			spans.push((lo.min(hi), lo.max(hi)));
		}
		spans
	}

	/// Which gallery-inner sides a shaft abuts, with along-side offset from mid.
	fn shaft_inner_sides(
		center_xz: Vec3,
		gallery_inner: Vec2,
		shaft: Aabb3d,
	) -> Vec<(OrthoSide, f32)> {
		let mid = Vec3::from((shaft.min + shaft.max) * 0.5);
		let smin = Vec3::from(shaft.min);
		let smax = Vec3::from(shaft.max);
		let gx0 = center_xz.x - gallery_inner.x * 0.5;
		let gx1 = center_xz.x + gallery_inner.x * 0.5;
		let gz0 = center_xz.z - gallery_inner.y * 0.5;
		let gz1 = center_xz.z + gallery_inner.y * 0.5;
		// Shaft sits in the gallery band; treat as abutting if its AABB reaches the
		// inner wall plane (within a small thickness tolerance).
		let tol = 0.35_f32;

		let mut out = Vec::new();
		if aabb_near_plane(smin.z, smax.z, gz0, tol) {
			out.push((OrthoSide::South, mid.x - center_xz.x));
		}
		if aabb_near_plane(smin.z, smax.z, gz1, tol) {
			out.push((OrthoSide::North, mid.x - center_xz.x));
		}
		if aabb_near_plane(smin.x, smax.x, gx1, tol) {
			out.push((OrthoSide::East, mid.z - center_xz.z));
		}
		if aabb_near_plane(smin.x, smax.x, gx0, tol) {
			out.push((OrthoSide::West, mid.z - center_xz.z));
		}
		if out.is_empty() {
			let dists = [
				(OrthoSide::South, (mid.z - gz0).abs(), mid.x - center_xz.x),
				(OrthoSide::North, (mid.z - gz1).abs(), mid.x - center_xz.x),
				(OrthoSide::East, (mid.x - gx1).abs(), mid.z - center_xz.z),
				(OrthoSide::West, (mid.x - gx0).abs(), mid.z - center_xz.z),
			];
			let best = dists
				.into_iter()
				.min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
				.unwrap();
			out.push((best.0, best.2));
		}
		out
	}

	/// Radial partitions from gallery inner wall to outer wall at each shaft.
	fn build_shaft_walls(
		center_xz: Vec3,
		outer: Vec2,
		gallery_inner: Vec2,
		height: f32,
		shaft_bounds: &[Aabb3d],
	) -> Vec<Rectangle> {
		let y0 = center_xz.y;
		let ox0 = center_xz.x - outer.x * 0.5;
		let ox1 = center_xz.x + outer.x * 0.5;
		let oz0 = center_xz.z - outer.y * 0.5;
		let oz1 = center_xz.z + outer.y * 0.5;
		let gx0 = center_xz.x - gallery_inner.x * 0.5;
		let gx1 = center_xz.x + gallery_inner.x * 0.5;
		let gz0 = center_xz.z - gallery_inner.y * 0.5;
		let gz1 = center_xz.z + gallery_inner.y * 0.5;
		let t = DEFAULT_PANEL_THICKNESS;
		let mut walls = Vec::new();

		for shaft in shaft_bounds {
			let smin = Vec3::from(shaft.min);
			let smax = Vec3::from(shaft.max);
			let sides = Self::shaft_inner_sides(center_xz, gallery_inner, *shaft);
			for (side, _) in sides {
				match side {
					OrthoSide::South => {
						// Radials at shaft east/west faces, outer south → inner south.
						walls.push(radial_wall(
							Vec3::new(smin.x, y0, oz0),
							Vec3::new(0.0, 0.0, gz0 - oz0),
							height,
							t,
						));
						walls.push(radial_wall(
							Vec3::new(smax.x, y0, oz0),
							Vec3::new(0.0, 0.0, gz0 - oz0),
							height,
							t,
						));
					}
					OrthoSide::North => {
						walls.push(radial_wall(
							Vec3::new(smin.x, y0, oz1),
							Vec3::new(0.0, 0.0, gz1 - oz1),
							height,
							t,
						));
						walls.push(radial_wall(
							Vec3::new(smax.x, y0, oz1),
							Vec3::new(0.0, 0.0, gz1 - oz1),
							height,
							t,
						));
					}
					OrthoSide::East => {
						walls.push(radial_wall(
							Vec3::new(ox1, y0, smin.z),
							Vec3::new(gx1 - ox1, 0.0, 0.0),
							height,
							t,
						));
						walls.push(radial_wall(
							Vec3::new(ox1, y0, smax.z),
							Vec3::new(gx1 - ox1, 0.0, 0.0),
							height,
							t,
						));
					}
					OrthoSide::West => {
						walls.push(radial_wall(
							Vec3::new(ox0, y0, smin.z),
							Vec3::new(gx0 - ox0, 0.0, 0.0),
							height,
							t,
						));
						walls.push(radial_wall(
							Vec3::new(ox0, y0, smax.z),
							Vec3::new(gx0 - ox0, 0.0, 0.0),
							height,
							t,
						));
					}
				}
			}
		}
		walls.retain(|w| w.edge.length() > EPS);
		walls
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
		for wall in &self.shaft_walls {
			out.extend(wall.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.gallery.joint_nodes_for_level(level)
	}
}

/// One free straight run on the gallery inner wall (along coords vs side mid).
#[derive(Debug, Clone, Copy)]
struct InnerSection {
	side: OrthoSide,
	along0: f32,
	along1: f32,
}

fn side_slot(side: OrthoSide) -> &'static str {
	match side {
		OrthoSide::South => "s",
		OrthoSide::East => "e",
		OrthoSide::North => "n",
		OrthoSide::West => "w",
	}
}

fn aabb_near_plane(lo: f32, hi: f32, plane: f32, tol: f32) -> bool {
	lo <= plane + tol && hi >= plane - tol
}

fn along_overlaps_spans(along: f32, width: f32, spans: &[(f32, f32)]) -> bool {
	let half = width * 0.5;
	let a0 = along - half;
	let a1 = along + half;
	spans.iter().any(|&(lo, hi)| a0 <= hi && a1 >= lo)
}

/// Greatest XZ overlap with a mapping region; closest region center if no overlap.
fn best_shaft_slot(request: &Aabb3d, regions: &[Aabb2d]) -> Option<usize> {
	if regions.is_empty() {
		return None;
	}
	let rmin = Vec3::from(request.min);
	let rmax = Vec3::from(request.max);
	let rcx = (rmin.x + rmax.x) * 0.5;
	let rcz = (rmin.z + rmax.z) * 0.5;

	let mut best_i = 0usize;
	let mut best_area = -1.0_f32;
	let mut best_dist = f32::INFINITY;
	for (i, region) in regions.iter().enumerate() {
		let area = xz_overlap_area(rmin.x, rmax.x, rmin.z, rmax.z, region);
		let cx = (region.min.x + region.max.x) * 0.5;
		let cz = (region.min.y + region.max.y) * 0.5;
		let dist = (rcx - cx).hypot(rcz - cz);
		let better = area > best_area + 1e-6
			|| ((area - best_area).abs() <= 1e-6 && dist < best_dist - 1e-6);
		if better {
			best_i = i;
			best_area = area;
			best_dist = dist;
		}
	}
	Some(best_i)
}

fn xz_overlap_area(ax0: f32, ax1: f32, az0: f32, az1: f32, region: &Aabb2d) -> f32 {
	let x0 = ax0.max(region.min.x);
	let x1 = ax1.min(region.max.x);
	let z0 = az0.max(region.min.y);
	let z1 = az1.min(region.max.y);
	(x1 - x0).max(0.0) * (z1 - z0).max(0.0)
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

fn radial_wall(origin: Vec3, edge: Vec3, height: f32, thickness: f32) -> Rectangle {
	Rectangle::rough_stone(origin, edge, height, thickness, 0.0)
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
	use procedural_common::{NoiseConfig, NoiseParams};

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

	fn fixed_params(placement: LesHallesShaftPlacement) -> LesHallesParameterized {
		LesHallesParameterized {
			gallery_width: 3.0,
			balcony_width: 1.5,
			shaft_placement: placement,
			opening_density: 0.5,
			doors: LesHallesFloorPlan::generate_stall_doors(
				&NoiseConfig::new(NoiseParams::default()),
				Vec3::ZERO,
			),
		}
	}

	fn aabb_xz_near(a: &Aabb3d, b: &Aabb3d) -> bool {
		let amin = Vec3::from(a.min);
		let amax = Vec3::from(a.max);
		let bmin = Vec3::from(b.min);
		let bmax = Vec3::from(b.max);
		(amin.x - bmin.x).abs() < 1e-4
			&& (amin.z - bmin.z).abs() < 1e-4
			&& (amax.x - bmax.x).abs() < 1e-4
			&& (amax.z - bmax.z).abs() < 1e-4
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
	fn emits_outer_apertures_and_inner_doors_per_section() {
		let (plan, _) =
			LesHallesFloorPlan::fit_to_confines(&nominal_confines(), NoiseParams::default()).unwrap();
		// Outer: apertures only.
		assert!(plan
			.openings
			.iter()
			.all(|(id, o)| !id.as_str().contains("outer_passage")
				&& !(id.as_str().contains("outer_") && matches!(o.label, OpeningLabel::Passage))));
		assert!(plan
			.openings
			.iter()
			.any(|(id, o)| id.as_str().contains("outer_aperture")
				&& matches!(o.label, OpeningLabel::Aperture)));
		// Inner: doors only (no shop apertures); at least one per straight section.
		assert!(plan
			.openings
			.iter()
			.all(|(id, _)| !id.as_str().contains("inner_aperture")));
		let expected = plan.parameterized.expected_inner_section_count();
		let section_doors: std::collections::HashSet<usize> = plan
			.openings
			.iter()
			.filter_map(|(id, _)| {
				let s = id.as_str();
				let rest = s.strip_prefix("les_halles_inner_door_")?;
				let (si, _) = rest.split_once('_')?;
				si.parse().ok()
			})
			.collect();
		assert_eq!(section_doors.len(), expected);
		use crate::openings::MapsOpenings;
		let outer_id = plan
			.openings
			.iter()
			.find(|(id, _)| id.as_str().contains("outer_aperture_s_"))
			.map(|(id, _)| id.clone())
			.expect("south outer aperture");
		assert!(plan.gallery.mapped_opening(&outer_id).is_some());
		assert!(!plan.shaft_walls.is_empty());
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

	#[test]
	fn corner_shaft_mapping_rewrites_inbound_by_quadrant() {
		// Request in SE quadrant (and one straddling SE/NE — SE wins by overlap).
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("req_se"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(2.0, 0.0, -6.0), Vec3::new(4.0, 3.0, -4.0)),
				OpeningLabel::Shaft,
			),
		);
		openings.insert(
			OpeningId::new("req_straddle"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(1.0, 0.0, -1.0), Vec3::new(5.0, 3.0, 3.0)),
				OpeningLabel::Shaft,
			),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::new(-10.0, 0.0, -8.0), Vec3::new(10.0, 3.5, 8.0)),
			0.0,
			openings,
		);
		let (plan, regions) = LesHallesFloorPlan::from_parameterized(
			fixed_params(LesHallesShaftPlacement::Corners),
			&confines,
		)
		.unwrap();

		let se_open = plan.openings.get(&OpeningId::new("req_se")).unwrap();
		assert!(aabb_xz_near(&se_open.bounds, &plan.shaft_bounds[1]));
		assert!(plan.shaft_inbound[1].contains(&OpeningId::new("req_se")));

		let straddle = plan.openings.get(&OpeningId::new("req_straddle")).unwrap();
		// SE∩request area 4, NE∩request area 12 → NE wins.
		assert!(aabb_xz_near(&straddle.bounds, &plan.shaft_bounds[2]));
		assert!(plan.shaft_inbound[2].contains(&OpeningId::new("req_straddle")));
		assert!(regions.within.iter().any(|c| {
			c.openings.get(&OpeningId::new("req_se")).is_some()
		}));
	}

	#[test]
	fn midside_shaft_mapping_uses_end_and_middle_bands() {
		let mut openings = Openings::new();
		// South end band
		openings.insert(
			OpeningId::new("req_s"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(-1.0, 0.0, -7.5), Vec3::new(1.0, 3.0, -6.0)),
				OpeningLabel::Shaft,
			),
		);
		// East middle band
		openings.insert(
			OpeningId::new("req_e"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(6.0, 0.0, -1.0), Vec3::new(9.0, 3.0, 1.0)),
				OpeningLabel::Shaft,
			),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::new(-10.0, 0.0, -8.0), Vec3::new(10.0, 3.5, 8.0)),
			0.0,
			openings,
		);
		let (plan, _) = LesHallesFloorPlan::from_parameterized(
			fixed_params(LesHallesShaftPlacement::MidSides),
			&confines,
		)
		.unwrap();

		assert!(aabb_xz_near(
			&plan.openings.get(&OpeningId::new("req_s")).unwrap().bounds,
			&plan.shaft_bounds[0]
		));
		assert!(aabb_xz_near(
			&plan.openings.get(&OpeningId::new("req_e")).unwrap().bounds,
			&plan.shaft_bounds[1]
		));
		assert!(plan.shaft_inbound[0].contains(&OpeningId::new("req_s")));
		assert!(plan.shaft_inbound[1].contains(&OpeningId::new("req_e")));
	}
}
