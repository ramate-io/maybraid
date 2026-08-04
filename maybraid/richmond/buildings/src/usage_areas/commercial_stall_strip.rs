//! Strip of commercial stalls along a gallery band.
//!
//! Packing is driven by [`OpeningLabel::Passage`] openings: each stall owns at
//! least one passage uniquely. Bay spans are voronoi cells along the strip’s
//! long axis (merged when shorter than the sampled minimum bay width), which
//! keeps stalls large and door-aligned.

pub mod commercial_stall;

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec2;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, LabelNode, Layers};

use crate::fit::{aabb_xz_extent, aabb_xz_overlap_area, Confines, FillableRegions, Fit, FitError};
use crate::openings::{OpeningId, OpeningLabel, Openings};

pub use commercial_stall::{
	BitesSitdownStall, BitesStall, CommercialStall, CommercialStallInterior,
	CommercialStallParameterized, CommercialStallPlan, KnickKnackStall, MiniMart, PartsStall,
	PublicRestroom,
};

/// Prefer larger gallery bays; merge voronoi cells below this when sampled.
const MIN_STALL_ALONG: f32 = 3.5;
const MAX_STALL_ALONG: f32 = 9.0;
const MIN_STRIP_ALONG: f32 = 3.5;

/// Noise knobs for packing stalls along a strip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommercialStallStripParameterized {
	/// Minimum preferred bay width along the strip’s long axis (merge threshold).
	pub bay_width: f32,
}

impl CommercialStallStripParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Self {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let bay = cfg.sample_range_f32_4d(
			MIN_STALL_ALONG,
			MAX_STALL_ALONG,
			c.x,
			c.y,
			c.z,
			21.0,
		);
		Self { bay_width: bay }
	}
}

/// Strip plan: ordered stall cells along the long plan axis.
#[derive(Debug, Clone, PartialEq)]
pub struct CommercialStallStripPlan {
	pub parameterized: CommercialStallStripParameterized,
	pub stalls: Vec<CommercialStall>,
}

impl CommercialStallStripPlan {
	pub fn from_parameterized(
		params: CommercialStallStripParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<Self, FitError> {
		let min = Vec3::from(confines.bounds.min);
		let max = Vec3::from(confines.bounds.max);
		let extent = aabb_xz_extent(&confines.bounds);
		let height = (max.y - min.y).max(1e-4);
		if height < 0.4 {
			return Err(FitError::TooSmall { reason: "height" });
		}
		let along_x = extent.x >= extent.y;
		let along = if along_x { extent.x } else { extent.y };
		let depth = if along_x { extent.y } else { extent.x };
		if along < MIN_STRIP_ALONG || depth < 0.4 {
			return Err(FitError::TooSmall { reason: "strip" });
		}

		let passages = collect_passages_along(&confines.openings, along_x, min, along);
		if passages.is_empty() {
			return Err(FitError::TooSmall {
				reason: "no passage",
			});
		}

		let min_bay = params
			.bay_width
			.clamp(MIN_STALL_ALONG, along.max(MIN_STALL_ALONG));
		let bays = partition_bays_for_passages(&passages, along, min_bay);
		let mut stalls = Vec::with_capacity(bays.len());
		for (i, bay) in bays.iter().enumerate() {
			let (smin, smax) = if along_x {
				(
					Vec3::new(min.x + bay.along0, min.y, min.z),
					Vec3::new(min.x + bay.along1, max.y, max.z),
				)
			} else {
				(
					Vec3::new(min.x, min.y, min.z + bay.along0),
					Vec3::new(max.x, max.y, min.z + bay.along1),
				)
			};
			let bay_bounds = Aabb3d::from_min_max(smin, smax);
			let cell = Confines::new(
				bay_bounds,
				confines.roll,
				openings_for_bay(&confines.openings, &bay_bounds, &bay.passage_ids),
			);
			let mut bay_noise = noise;
			bay_noise.seed = noise.seed.wrapping_add(i as i32 * 17);
			match CommercialStall::fit_to_confines(&cell, bay_noise) {
				Ok((stall, _)) => stalls.push(stall),
				Err(FitError::TooSmall { .. }) => continue,
				Err(err) => return Err(err),
			}
		}
		if stalls.is_empty() {
			return Err(FitError::TooSmall { reason: "stalls" });
		}
		Ok(Self {
			parameterized: params,
			stalls,
		})
	}
}

#[derive(Debug, Clone)]
struct PassageAlong {
	id: OpeningId,
	/// Center of the opening along the strip axis, relative to strip start.
	center: f32,
}

#[derive(Debug, Clone)]
struct StallBay {
	along0: f32,
	along1: f32,
	passage_ids: Vec<OpeningId>,
}

fn collect_passages_along(
	openings: &Openings,
	along_x: bool,
	strip_min: Vec3,
	along: f32,
) -> Vec<PassageAlong> {
	let origin = if along_x { strip_min.x } else { strip_min.z };
	let mut out = Vec::new();
	for (id, opening) in openings.iter() {
		if !matches!(opening.label, OpeningLabel::Passage) {
			continue;
		}
		let omin = Vec3::from(opening.bounds.min);
		let omax = Vec3::from(opening.bounds.max);
		let c = if along_x {
			(omin.x + omax.x) * 0.5 - origin
		} else {
			(omin.z + omax.z) * 0.5 - origin
		};
		if c < -0.5 || c > along + 0.5 {
			continue;
		}
		out.push(PassageAlong {
			id: id.clone(),
			center: c.clamp(0.0, along),
		});
	}
	out.sort_by(|a, b| {
		a.center
			.partial_cmp(&b.center)
			.unwrap_or(std::cmp::Ordering::Equal)
			.then_with(|| a.id.as_str().cmp(b.id.as_str()))
	});
	out
}

/// Voronoi partition of `[0, along]` by passage centers, then merge short cells.
fn partition_bays_for_passages(
	passages: &[PassageAlong],
	along: f32,
	min_bay: f32,
) -> Vec<StallBay> {
	debug_assert!(!passages.is_empty());
	let n = passages.len();
	let mut edges = Vec::with_capacity(n + 1);
	edges.push(0.0);
	for i in 0..n.saturating_sub(1) {
		edges.push((passages[i].center + passages[i + 1].center) * 0.5);
	}
	edges.push(along);

	let mut bays: Vec<StallBay> = (0..n)
		.map(|i| StallBay {
			along0: edges[i],
			along1: edges[i + 1],
			passage_ids: vec![passages[i].id.clone()],
		})
		.collect();

	// Merge left→right while a bay is shorter than the preferred minimum.
	let mut i = 0;
	while i < bays.len() {
		let w = bays[i].along1 - bays[i].along0;
		if w + 1e-4 >= min_bay || bays.len() == 1 {
			i += 1;
			continue;
		}
		if i + 1 < bays.len() {
			let right = bays.remove(i + 1);
			bays[i].along1 = right.along1;
			bays[i].passage_ids.extend(right.passage_ids);
		} else if i > 0 {
			let cur = bays.remove(i);
			let prev = &mut bays[i - 1];
			prev.along1 = cur.along1;
			prev.passage_ids.extend(cur.passage_ids);
		} else {
			break;
		}
	}
	bays
}

/// Owned passages (unique) plus intersecting non-passage opens for the bay.
fn openings_for_bay(
	openings: &Openings,
	bounds: &Aabb3d,
	owned_passages: &[OpeningId],
) -> Openings {
	let region = Aabb2d {
		min: Vec2::new(Vec3::from(bounds.min).x, Vec3::from(bounds.min).z),
		max: Vec2::new(Vec3::from(bounds.max).x, Vec3::from(bounds.max).z),
	};
	let y0 = Vec3::from(bounds.min).y;
	let y1 = Vec3::from(bounds.max).y;
	let mut out = Openings::new();
	for id in owned_passages {
		if let Some(opening) = openings.get(id) {
			out.insert(id.clone(), opening.clone());
		}
	}
	for (id, opening) in openings.iter() {
		if matches!(opening.label, OpeningLabel::Passage | OpeningLabel::Shaft) {
			continue;
		}
		if aabb_xz_overlap_area(&opening.bounds, &region) <= 1e-4 {
			continue;
		}
		let omin = Vec3::from(opening.bounds.min);
		let omax = Vec3::from(opening.bounds.max);
		if omax.y < y0 - 1e-3 || omin.y > y1 + 1e-3 {
			continue;
		}
		out.insert(id.clone(), opening.clone());
	}
	out
}

/// Full commercial stall strip.
#[derive(Debug, Clone, PartialEq)]
pub struct CommercialStallStrip {
	pub plan: CommercialStallStripPlan,
}

impl CommercialStallStrip {
	pub fn from_plan(plan: CommercialStallStripPlan) -> Self {
		Self { plan }
	}

	pub fn stalls(&self) -> &[CommercialStall] {
		&self.plan.stalls
	}
}

impl Fit for CommercialStallStrip {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = CommercialStallStripParameterized::sample(confines, noise);
		let plan = CommercialStallStripPlan::from_parameterized(params, confines, noise)?;
		Ok((Self::from_plan(plan), FillableRegions::empty()))
	}
}

impl BuildingComponents for CommercialStallStrip {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for stall in &self.plan.stalls {
			out.extend(stall.panel_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		for stall in &self.plan.stalls {
			out.extend(stall.label_nodes_for_level(level));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use crate::openings::Opening;

	fn strip_with_doors(door_mids: &[f32], extent_x: f32) -> Confines {
		let mut openings = Openings::new();
		for (i, &mid) in door_mids.iter().enumerate() {
			openings.insert(
				OpeningId::new(format!("door_{i}")),
				Opening::passage(Aabb3d::from_min_max(
					Vec3::new(mid - 0.6, 0.0, -0.2),
					Vec3::new(mid + 0.6, 2.2, 0.2),
				)),
			);
		}
		Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(extent_x, 3.5, 5.0)),
			0.0,
			openings,
		)
	}

	#[test]
	fn strip_packs_one_stall_per_passage() {
		let confines = strip_with_doors(&[2.5, 7.5, 12.5], 15.0);
		// Explicit min bay below voronoi cell width (~5m) so cells do not merge.
		let params = CommercialStallStripParameterized { bay_width: 3.5 };
		let plan =
			CommercialStallStripPlan::from_parameterized(params, &confines, NoiseParams::default())
				.unwrap();
		assert_eq!(plan.stalls.len(), 3);
		let strip = CommercialStallStrip::from_plan(plan);
		assert!(!strip
			.label_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.is_empty());
		assert!(!strip
			.panel_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.is_empty());
	}

	#[test]
	fn each_bay_owns_passages_uniquely() {
		let confines = strip_with_doors(&[3.0, 9.0, 15.0], 18.0);
		let passages = collect_passages_along(&confines.openings, true, Vec3::ZERO, 18.0);
		let bays = partition_bays_for_passages(&passages, 18.0, 3.5);
		assert_eq!(bays.len(), 3);
		let mut seen = std::collections::HashSet::new();
		for bay in &bays {
			assert!(!bay.passage_ids.is_empty());
			for id in &bay.passage_ids {
				assert!(seen.insert(id.clone()), "passage {} claimed twice", id.as_str());
			}
			let bay_bounds = Aabb3d::from_min_max(
				Vec3::new(bay.along0, 0.0, 0.0),
				Vec3::new(bay.along1, 3.5, 5.0),
			);
			let cell_openings =
				openings_for_bay(&confines.openings, &bay_bounds, &bay.passage_ids);
			let cell_passages: std::collections::HashSet<_> = cell_openings
				.iter()
				.filter(|(_, o)| matches!(o.label, OpeningLabel::Passage))
				.map(|(id, _)| id.clone())
				.collect();
			let owned: std::collections::HashSet<_> = bay.passage_ids.iter().cloned().collect();
			assert_eq!(cell_passages, owned);
		}
	}

	#[test]
	fn close_passages_merge_into_larger_stall() {
		// Three doors clustered; min bay 6m on a 12m strip → fewer than 3 stalls.
		let confines = strip_with_doors(&[2.0, 3.5, 5.0], 12.0);
		let params = CommercialStallStripParameterized { bay_width: 6.0 };
		let plan =
			CommercialStallStripPlan::from_parameterized(params, &confines, NoiseParams::default())
				.unwrap();
		assert!(plan.stalls.len() < 3);
		assert!(!plan.stalls.is_empty());
	}

	#[test]
	fn without_passage_strip_fails() {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(12.0, 3.5, 5.0),
		));
		let err = CommercialStallStrip::fit_to_confines(&confines, NoiseParams::default())
			.unwrap_err();
		assert!(matches!(err, FitError::TooSmall { reason } if reason.contains("passage")));
	}
}
