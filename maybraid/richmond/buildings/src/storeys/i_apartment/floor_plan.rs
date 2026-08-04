//! I-Apartment floor plan: fit an [`IFloor`] and expose its 1–3 primary rectangles.

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::{plan_to_aabb3, PlanAxes};
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::{LabelNode, LabelStyle};
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{
	aabb_xz_extent, Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind, StackRegion,
};
use crate::openings::Openings;
use crate::shells::{IFloor, IFloorParams, IFloorPlanRect, IFloorSlab};

use super::parameterized::{IApartmentParameterized, MIN_STEM_WIDTH, MIN_STOREY_HEIGHT};
use super::SCOPE;

/// I-Apartment floor plan: I-frame shell + primary rectangular regions.
#[derive(Debug, Clone, PartialEq)]
pub struct IApartmentFloorPlan {
	pub parameterized: IApartmentParameterized,
	pub center_xz: Vec3,
	pub storey_height: f32,
	pub roll: f32,
	pub shell: IFloor,
	/// Natural I-frame rectangles (stem + optional flange bars), 1–3.
	pub primary_rects: Vec<IFloorPlanRect>,
}

impl IApartmentFloorPlan {
	pub fn from_parameterized(
		params: IApartmentParameterized,
		confines: &Confines,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_parameterized_with_ceiling(params, confines, IFloorSlab::None)
	}

	pub fn from_parameterized_with_ceiling(
		params: IApartmentParameterized,
		confines: &Confines,
		ceiling: IFloorSlab,
	) -> Result<(Self, FillableRegions), FitError> {
		let height = (confines.bounds.max.y - confines.bounds.min.y).max(0.0);
		if height < MIN_STOREY_HEIGHT {
			return Err(FitError::TooSmall { reason: "height" });
		}
		let y0 = confines.bounds.min.y;
		let center = confines.center();
		let center_xz = Vec3::new(center.x, y0, center.z);

		let ifloor_params = derive_ifloor_params(&params, confines, center_xz, height, ceiling)?;
		let primary_rects = ifloor_params.plan_rects();
		if primary_rects.is_empty() {
			return Err(FitError::TooSmall { reason: "i_rects" });
		}

		let shell = IFloor::new(IFloorParams {
			openings: confines.openings.clone(),
			..ifloor_params
		});

		let plan = Self {
			parameterized: params,
			center_xz,
			storey_height: height,
			roll: confines.roll,
			shell,
			primary_rects,
		};
		let regions = plan.fillable_regions();
		Ok((plan, regions))
	}

	/// Primary I-frame rectangles as typed residuals (one confine per rect).
	pub fn fillable_regions(&self) -> FillableRegions {
		let y0 = self.center_xz.y;
		let y1 = y0 + self.storey_height;
		let host = Aabb3d::from_min_max(
			Vec3::new(
				self.primary_rects
					.iter()
					.map(|r| r.min_x)
					.fold(f32::INFINITY, f32::min),
				y0,
				self.primary_rects
					.iter()
					.map(|r| r.min_z)
					.fold(f32::INFINITY, f32::min),
			),
			Vec3::new(
				self.primary_rects
					.iter()
					.map(|r| r.max_x)
					.fold(f32::NEG_INFINITY, f32::max),
				y1,
				self.primary_rects
					.iter()
					.map(|r| r.max_z)
					.fold(f32::NEG_INFINITY, f32::max),
			),
		);

		let mut within = Vec::new();
		for (i, rect) in self.primary_rects.iter().enumerate() {
			let bounds = plan_to_aabb3(
				&host,
				rect.to_aabb2(),
				PlanAxes::XZ,
			);
			within.push(FillRegion::new(
				SpaceKind::Custom(format!("{SCOPE}_rect_{i}")),
				Confines::new(bounds, self.roll, Openings::new()),
			));
		}

		let atop_bounds = primary_union_aabb2(&self.primary_rects).unwrap_or(Aabb2d {
			min: Vec2::new(self.center_xz.x - 1.0, self.center_xz.z - 1.0),
			max: Vec2::new(self.center_xz.x + 1.0, self.center_xz.z + 1.0),
		});
		let atop = vec![StackRegion {
			bounds: atop_bounds,
			height: self.storey_height,
			roll: self.roll,
			openings: self.shell.params().openings.clone(),
		}];

		FillableRegions { within, atop }
	}
}

impl Fit for IApartmentFloorPlan {
	fn fit_to_confines(
		confines: &Confines,
		noise: procedural_common::NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = IApartmentParameterized::sample(confines, noise)?;
		Self::from_parameterized(params, confines)
	}
}

impl BuildingComponents for IApartmentFloorPlan {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.shell.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.shell.joint_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		let y0 = self.center_xz.y;
		let y1 = y0 + self.storey_height;
		for (i, rect) in self.primary_rects.iter().enumerate() {
			let bounds = Aabb3d::from_min_max(
				Vec3::new(rect.min_x, y0, rect.min_z),
				Vec3::new(rect.max_x, y1, rect.max_z),
			);
			let label = match (i, self.primary_rects.len()) {
				(0, _) => "Stem",
				(1, 2) => "Flange",
				(1, _) => "Top flange",
				(2, _) => "Bottom flange",
				_ => "Rect",
			};
			out.push_free(label_filling_aabb(
				LabelStyle::Blue,
				&format!("{label} {i}"),
				&bounds,
				self.roll,
			));
		}
		out
	}
}

fn derive_ifloor_params(
	params: &IApartmentParameterized,
	confines: &Confines,
	center_xz: Vec3,
	height: f32,
	ceiling: IFloorSlab,
) -> Result<IFloorParams, FitError> {
	let footprint = aabb_xz_extent(&confines.bounds);
	let short = footprint.x.min(footprint.y);
	let stem_w = (short * params.stem_width_frac).max(MIN_STEM_WIDTH);
	// Flange thickness equals stem width in IFloor geometry.
	let flange_bars = (params.has_top_flange as u8) + (params.has_bottom_flange as u8);
	let min_depth = stem_w * flange_bars as f32 + 2.0;
	if footprint.y < min_depth {
		return Err(FitError::TooSmall { reason: "stem_depth" });
	}
	let central_depth = footprint.y - stem_w * flange_bars as f32;
	if central_depth < 2.0 {
		return Err(FitError::TooSmall { reason: "stem_depth" });
	}
	let (tl, tr, bl, br) = params.flange_lengths(footprint, stem_w);
	Ok(IFloorParams {
		center_xz,
		top_left_length: tl,
		top_right_length: tr,
		central_rectangle: Vec2::new(stem_w, central_depth),
		bottom_left_length: bl,
		bottom_right_length: br,
		storey_height: height,
		openings: Openings::new(),
		floor: IFloorSlab::Solid,
		ceiling,
		style: PanelStyle::RoughStonework,
		joint_thickness: crate::paneling::DEFAULT_PANEL_THICKNESS,
	})
}

fn primary_union_aabb2(rects: &[IFloorPlanRect]) -> Option<Aabb2d> {
	let mut iter = rects.iter();
	let first = iter.next()?;
	let mut min = Vec2::new(first.min_x, first.min_z);
	let mut max = Vec2::new(first.max_x, first.max_z);
	for r in iter {
		min.x = min.x.min(r.min_x);
		min.y = min.y.min(r.min_z);
		max.x = max.x.max(r.max_x);
		max.y = max.y.max(r.max_z);
	}
	Some(Aabb2d { min, max })
}

fn label_filling_aabb(style: LabelStyle, text: &str, aabb: &Aabb3d, yaw: f32) -> LabelNode {
	let center = Vec3::from(aabb.center());
	let extents = Vec3::from(aabb.max - aabb.min).max(Vec3::splat(1e-4));
	LabelNode::rectangle(style, text, center, extents, yaw)
}

#[cfg(test)]
mod tests {
	use super::*;
	use procedural_common::NoiseParams;

	fn large_confines() -> Confines {
		Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-22.0, 0.0, -18.0),
			Vec3::new(22.0, 3.5, 18.0),
		))
	}

	#[test]
	fn emits_one_to_three_primary_rects() {
		let confines = large_confines();
		let params = IApartmentParameterized::sample(&confines, NoiseParams::default()).unwrap();
		let (plan, regions) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		assert!((1..=3).contains(&plan.primary_rects.len()));
		assert_eq!(regions.within.len(), plan.primary_rects.len());
		assert!(!plan
			.label_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.is_empty());
	}
}
