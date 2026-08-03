//! Les Halles floor plan: ring shell, shafts, and residual fill regions.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError, StackRegion};
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::shells::rect_ring_floor::{RectRingFloor, RectRingFloorParams, RectRingFloorSlab};

use super::parameterized::{
	footprint_extents, LesHallesParameterized, LesHallesShaftPlacement, SHAFT_SIDE,
};
use super::SCOPE;

/// Structural Les Halles plan: rectangular-ring shell plus residual fill cells.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesFloorPlan {
	pub parameterized: LesHallesParameterized,
	/// Storey plan center; `y` is the floor elevation.
	pub center_xz: Vec3,
	/// Full outer width (X) / depth (Z).
	pub outer: Vec2,
	/// Courtyard width (X) / depth (Z).
	pub inner: Vec2,
	pub storey_height: f32,
	pub roll: f32,
	/// Merged inbound + generated openings for the ring shell.
	pub openings: Openings,
	/// Shaft volumes (same bounds as shaft openings / within cells).
	pub shaft_bounds: Vec<Aabb3d>,
	/// Outer/inner gallery walls + frame slabs for this plan.
	pub shell: RectRingFloor,
}

impl LesHallesFloorPlan {
	/// Deterministic structure from already-sampled parameters (towering path).
	pub fn from_parameterized(
		params: LesHallesParameterized,
		confines: &Confines,
	) -> Result<(Self, FillableRegions), FitError> {
		let (extent_x, extent_z, height) = footprint_extents(confines)?;
		let ring = params.ring_width();
		let min_courtyard_x = extent_x - 2.0 * ring;
		let min_courtyard_z = extent_z - 2.0 * ring;
		if min_courtyard_x < super::parameterized::MIN_COURTYARD
			|| min_courtyard_z < super::parameterized::MIN_COURTYARD
		{
			return Err(FitError::TooSmall { reason: "footprint" });
		}

		let center = confines.center();
		let y0 = confines.bounds.min.y;
		let center_xz = Vec3::new(center.x, y0, center.z);
		let outer = Vec2::new(extent_x, extent_z);
		let inner = Vec2::new(min_courtyard_x, min_courtyard_z);

		let shaft_bounds =
			Self::shaft_aabbs(center_xz, outer, params.gallery_width, height, params.shaft_placement);

		let mut openings = confines.openings.clone();
		openings.extend(&Self::generated_openings(
			center_xz,
			outer,
			height,
			params.opening_density,
			&shaft_bounds,
		));

		let shell = Self::build_shell(center_xz, outer, inner, height, &openings);

		let plan = Self {
			parameterized: params,
			center_xz,
			outer,
			inner,
			storey_height: height,
			roll: confines.roll,
			openings,
			shaft_bounds,
			shell,
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
		let ix0 = self.center_xz.x - self.inner.x * 0.5;
		let ix1 = self.center_xz.x + self.inner.x * 0.5;
		let iz0 = self.center_xz.z - self.inner.y * 0.5;
		let iz1 = self.center_xz.z + self.inner.y * 0.5;

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
		// Place shafts in the gallery band, inset from the outer wall.
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

	/// Plan-authored gallery passages / apertures plus shaft voids.
	fn generated_openings(
		center_xz: Vec3,
		outer: Vec2,
		height: f32,
		opening_density: f32,
		shaft_bounds: &[Aabb3d],
	) -> Openings {
		let mut openings = Openings::new();
		let y0 = center_xz.y;
		let door_h = (height * 0.72).clamp(2.0, height.max(2.0));
		let win_h0 = y0 + height * 0.35;
		let win_h1 = y0 + height * 0.75;
		let ox0 = center_xz.x - outer.x * 0.5;
		let ox1 = center_xz.x + outer.x * 0.5;
		let oz0 = center_xz.z - outer.y * 0.5;
		let oz1 = center_xz.z + outer.y * 0.5;
		let wall_t = 0.35;

		// Always: one passage mid-south on the outer gallery wall.
		openings.insert(
			OpeningId::scoped(SCOPE, "gallery_passage", "s"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(center_xz.x - 0.7, y0, oz0 - wall_t * 0.5),
				Vec3::new(center_xz.x + 0.7, y0 + door_h, oz0 + wall_t * 0.5),
			)),
		);

		// Density-driven extra apertures on outer N/E/W.
		let extra = ((opening_density * 3.0).floor() as usize).min(3);
		let sides: [(&str, Vec3, Vec3); 3] = [
			(
				"n",
				Vec3::new(center_xz.x - 0.6, win_h0, oz1 - wall_t * 0.5),
				Vec3::new(center_xz.x + 0.6, win_h1, oz1 + wall_t * 0.5),
			),
			(
				"e",
				Vec3::new(ox1 - wall_t * 0.5, win_h0, center_xz.z - 0.6),
				Vec3::new(ox1 + wall_t * 0.5, win_h1, center_xz.z + 0.6),
			),
			(
				"w",
				Vec3::new(ox0 - wall_t * 0.5, win_h0, center_xz.z - 0.6),
				Vec3::new(ox0 + wall_t * 0.5, win_h1, center_xz.z + 0.6),
			),
		];
		for (i, (slot, min, max)) in sides.iter().enumerate() {
			if i < extra {
				openings.insert(
					OpeningId::scoped(SCOPE, "gallery_aperture", slot),
					Opening::aperture(Aabb3d::from_min_max(*min, *max)),
				);
			}
		}

		for (i, shaft) in shaft_bounds.iter().enumerate() {
			openings.insert(
				OpeningId::scoped(SCOPE, "shaft", format!("{i}")),
				Opening::new(*shaft, OpeningLabel::Shaft),
			);
		}

		openings
	}

	fn build_shell(
		center_xz: Vec3,
		outer: Vec2,
		inner: Vec2,
		storey_height: f32,
		openings: &Openings,
	) -> RectRingFloor {
		RectRingFloor::new(
			RectRingFloorParams::new(center_xz, outer, inner, storey_height)
				.floor(RectRingFloorSlab::Solid)
				.ceiling(RectRingFloorSlab::Solid)
				.openings(openings.clone()),
		)
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
		self.shell.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.shell.joint_nodes_for_level(level)
	}
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
		assert!(plan.shell.wall_count() >= 4);
		assert!(regions.within.len() >= 8 + 4); // galleries + balconies + shafts
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
		assert_eq!(via_fit.inner, via_params.inner);
		assert_eq!(
			via_fit.parameterized.shaft_placement,
			via_params.parameterized.shaft_placement
		);
		assert_eq!(via_fit.shaft_bounds.len(), 4);
	}
}
