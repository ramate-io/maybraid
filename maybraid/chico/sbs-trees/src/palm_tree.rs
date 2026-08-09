//! Shared VegetationComponents emission for palm trunks and frond crowns.

use bevy::prelude::*;
use chico_ball_components::frond::FrondCrownShape;
use chico_sbs_geometry::{BallStickChain, Hysteresis};
use chico_vegetation_components::{
	FoliageNode, FrondCollection, FrondRun, Placement, StickGeometry, StickNode,
	StructuralLod, STRUCTURAL_HIGH_FACTOR, STRUCTURAL_LOW_FACTOR,
	STRUCTURAL_MEDIUM_FACTOR,
};

/// Medium outer edge: default structural Medium × 3 (same as [`crate::PalmCrown`]).
pub(crate) const PALM_STRUCTURAL_MEDIUM_FACTOR: f32 = STRUCTURAL_MEDIUM_FACTOR * 3.0;
/// Keep Low beyond Medium so band ordering stays valid.
pub(crate) const PALM_STRUCTURAL_LOW_FACTOR: f32 = STRUCTURAL_LOW_FACTOR * 3.0;

/// Target fronds (runs) per [`FrondCollection`] — small groups keep merge extent
/// rachis-scale without the oversized UltraLow chord of a full ring.
pub(crate) const FRONDS_PER_COLLECTION: usize = 3;

/// Bake mesh-local frond shape into world units (keeps authored rachis segment count).
pub(crate) fn world_space_frond_shape(
	mut shape: FrondCrownShape,
	frond_world_scale: f32,
) -> FrondCrownShape {
	let s = frond_world_scale.max(1e-8);
	shape.length = (shape.length * s).max(1e-4);
	shape.width = (shape.width * s).max(1e-6);
	shape.droop *= s;
	shape.arch_lift *= s;
	shape.spine_segments = shape.spine_segments.max(1);
	shape
}

/// [`FrondCollection`]s of ~[`FRONDS_PER_COLLECTION`] fronds each at every ring anchor.
pub(crate) fn frond_collection_nodes(
	rings: impl IntoIterator<Item = (Vec3, FrondCrownShape)>,
) -> Vec<FoliageNode> {
	let mut nodes = Vec::new();
	for (anchor, shape) in rings {
		let mut batch: Vec<FrondRun> = Vec::with_capacity(FRONDS_PER_COLLECTION);
		for run in shape.frond_runs_at(anchor) {
			let placements: Vec<Placement> = run
				.into_iter()
				.filter_map(|seg| {
					Placement::frond_segment(seg.start, seg.direction, seg.length, seg.width)
				})
				.collect();
			if placements.is_empty() {
				continue;
			}
			batch.push(FrondRun::from_placements(placements));
			if batch.len() >= FRONDS_PER_COLLECTION {
				nodes.push(FoliageNode::frond_collection(
					FrondCollection::new(std::mem::take(&mut batch)).bake_bounds_from_runs(),
					Placement::IDENTITY,
				));
			}
		}
		if !batch.is_empty() {
			nodes.push(FoliageNode::frond_collection(
				FrondCollection::new(batch).bake_bounds_from_runs(),
				Placement::IDENTITY,
			));
		}
	}
	nodes
}

/// AABB of rachis polylines for the given ring shapes / anchors.
pub(crate) fn crown_aabb_from_rings(
	rings: impl IntoIterator<Item = (Vec3, FrondCrownShape)>,
) -> (Vec3, Vec3) {
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	let mut any = false;
	for (anchor, shape) in rings {
		for run in shape.frond_runs_at(anchor) {
			for seg in run {
				let tip = seg.start + seg.direction * seg.length;
				let half_w = seg.width * 0.5;
				for p in [seg.start, tip] {
					min = min.min(p - Vec3::splat(half_w));
					max = max.max(p + Vec3::splat(half_w));
					any = true;
				}
			}
		}
	}
	if !any {
		return (Vec3::splat(-0.5), Vec3::splat(0.5));
	}
	(min, max)
}

/// Two layered balls with rotated pose offsets for a denser Low silhouette.
pub(crate) fn layered_proxy_balls(min: Vec3, max: Vec3) -> Vec<FoliageNode> {
	let center = (min + max) * 0.5;
	let half_extents = ((max - min) * 0.5).max(Vec3::splat(1e-4));
	let scale = half_extents * 0.9;
	let offset = Vec3::new(half_extents.x * 0.12, half_extents.y * 0.04, 0.0);
	let yaw_b = std::f32::consts::FRAC_PI_2;
	let center_a = center + offset;
	let center_b = center + Quat::from_rotation_y(yaw_b) * offset;
	vec![
		FoliageNode::layered_ball(
			Placement::new(center_a, 0.0)
				.with_pitch(0.18)
				.with_roll(-0.22)
				.with_scale(scale),
		),
		FoliageNode::layered_ball(
			Placement::new(center_b, yaw_b)
				.with_pitch(-0.28)
				.with_roll(0.4)
				.with_scale(scale),
		),
	]
}

pub(crate) fn palm_structural_lod(
	center: Vec3,
	tree_radius: f32,
) -> StructuralLod {
	StructuralLod::new(center, tree_radius.max(1e-3)).with_factors(
		STRUCTURAL_HIGH_FACTOR,
		PALM_STRUCTURAL_MEDIUM_FACTOR,
		PALM_STRUCTURAL_LOW_FACTOR,
	)
}

/// All chain segments as trunk kits (date / Waialea columnar or arched trunks).
pub(crate) fn trunk_stick_nodes<C: Hysteresis>(chain: &BallStickChain<C>) -> Vec<StickNode> {
	chain
		.segments_with_hysteresis()
		.filter_map(|(segment, _, _)| {
			StickNode::from_segment_geometry(
				segment.start.position,
				segment.end.position,
				segment.start.radius,
				StickGeometry::Trunk,
			)
		})
		.collect()
}
