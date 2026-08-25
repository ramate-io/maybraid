//! Shared VegetationComponents emission for palm trunks and frond crowns.

use bevy::prelude::*;
use chico_ball_components::frond::FrondCrownShape;
use chico_sbs_geometry::{BallStickChain, Hysteresis};
use chico_vegetation_components::{
	chico_leaf_material_ref, chico_stick_material_ref, FoliageNode, FrondCollection, FrondRun,
	Placement, StickGeometry, StickNode, StructuralLod,
};

/// Target fronds (runs) per [`FrondCollection`]. Batches stay small so UltraLow merge
/// cannot chord the whole crown; LOD probe is the parent crown, not this batch AABB.
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
///
/// `probe_center` / `probe_radius` are the parent crown (same unit as structural LOD).
pub(crate) fn frond_collection_nodes(
	rings: &[(Vec3, FrondCrownShape)],
	probe_center: Vec3,
	probe_radius: f32,
) -> Vec<FoliageNode> {
	let mut nodes = Vec::new();
	for (anchor, shape) in rings {
		let mut batch: Vec<FrondRun> = Vec::with_capacity(FRONDS_PER_COLLECTION);
		for run in shape.frond_runs_at(*anchor) {
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
					FrondCollection::new(std::mem::take(&mut batch))
						.with_probe(probe_center, probe_radius),
					Placement::IDENTITY,
				));
			}
		}
		if !batch.is_empty() {
			nodes.push(FoliageNode::frond_collection(
				FrondCollection::new(batch).with_probe(probe_center, probe_radius),
				Placement::IDENTITY,
			));
		}
	}
	nodes
}

/// Parent-crown LOD probe: AABB center, radius at least the crown half-extent.
///
/// When `footprint_and_height` is set, also floors radius with
/// [`StructuralLod::characteristic_radius`].
pub(crate) fn crown_lod_probe(
	rings: &[(Vec3, FrondCrownShape)],
	footprint_and_height: Option<(f32, f32)>,
) -> (Vec3, f32) {
	let (min, max) = crown_aabb_from_rings(rings);
	let center = (min + max) * 0.5;
	let crown_r = ((max - min) * 0.5).max_element().max(1e-3);
	let radius = match footprint_and_height {
		Some((footprint, height)) => {
			StructuralLod::characteristic_radius(footprint, height).max(crown_r)
		}
		None => crown_r,
	};
	(center, radius)
}

/// AABB of rachis polylines for the given ring shapes / anchors.
pub(crate) fn crown_aabb_from_rings(rings: &[(Vec3, FrondCrownShape)]) -> (Vec3, Vec3) {
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	let mut any = false;
	for (anchor, shape) in rings {
		for run in shape.frond_runs_at(*anchor) {
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
		)
		.with_material(chico_leaf_material_ref()),
		FoliageNode::layered_ball(
			Placement::new(center_b, yaw_b)
				.with_pitch(-0.28)
				.with_roll(0.4)
				.with_scale(scale),
		)
		.with_material(chico_leaf_material_ref()),
	]
}

/// One cheap-ball column along the grown trunk (Low / UltraLow silhouette).
///
/// Tall palms have no mid-tree canopy, so a crown-only Low reads as a floating tuft.
/// Kit \(+Y\) follows the base→tip chord so a slight Waialea arch still reads.
pub(crate) fn trunk_proxy_node<C: Hysteresis>(
	chain: &BallStickChain<C>,
	height: f32,
	base_radius: f32,
) -> FoliageNode {
	let start = chain.nodes.first().map(|n| n.position).unwrap_or(Vec3::ZERO);
	let end = chain.nodes.last().map(|n| n.position).unwrap_or(Vec3::Y * height.max(1e-4));
	let mid = (start + end) * 0.5;
	let delta = end - start;
	let half_len = (delta.length() * 0.5).max(height.max(1e-4) * 0.35);
	let radius = base_radius.max(height.max(1e-4) * 0.02);
	let mut placement = Placement::new(mid, 0.0).with_scale(Vec3::new(radius, half_len, radius));
	if let Some(dir) = delta.try_normalize() {
		placement = placement.with_rotation(Quat::from_rotation_arc(Vec3::Y, dir));
	}
	FoliageNode::cheap_ball(placement).with_material(chico_stick_material_ref())
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

#[cfg(test)]
pub(crate) fn assert_high_collections_match_structural_lod(
	built: &impl chico_vegetation_components::VegetationComponents,
) {
	use lod::gen::LodSceneLevel;

	let probe = built.structural_lod().expect("structural probe");
	let nodes = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
	assert!(!nodes.is_empty());
	for node in &nodes {
		let collection = node.geometry.as_frond_collection().expect("collection");
		let (center, radius) = collection.center_and_extent();
		assert!((center - probe.center).length() < 1e-4);
		assert!((radius - probe.tree_radius).abs() < 1e-4);
	}
}
