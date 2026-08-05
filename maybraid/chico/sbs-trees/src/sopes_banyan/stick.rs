//! Stick segment → [`StickNode`] emission.

use chico_sbs_geometry::BallStickSegment;
use chico_vegetation_components::{Placement, StickNode};

pub(crate) fn stick_node_for_segment(segment: &BallStickSegment<'_>) -> Option<StickNode> {
	let ray = segment.ray();
	let len_sq = ray.length_squared();
	if len_sq < 1e-12 {
		return None;
	}
	let length = len_sq.sqrt();
	let placement =
		Placement::stick_segment(segment.start.position, ray, length, segment.start.radius)?;
	Some(StickNode::segment(placement))
}
