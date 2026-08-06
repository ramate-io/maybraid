//! Stick segment → [`StickNode`] emission (with structural LOD phase filters).

use chico_sbs_geometry::{BallStickSegment, SopesBanyanChain, SopesBanyanPhase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StickLodRole {
	/// Strict stalk / trunk.
	Trunk,
	/// Descender aerial roots.
	Descender,
	/// Branch-out / flair sticks that connect successive ball-stick nodes.
	BetweenNodes,
}

impl StickLodRole {
	pub(crate) fn from_parent_phase(phase: &SopesBanyanPhase) -> Self {
		match phase {
			SopesBanyanPhase::Stalk(_) => Self::Trunk,
			SopesBanyanPhase::StartDescender(_) | SopesBanyanPhase::EndDescender(_) => {
				Self::Descender
			}
			SopesBanyanPhase::BranchOut(_)
			| SopesBanyanPhase::StartFlairUp(_)
			| SopesBanyanPhase::EndFlairUp(_) => Self::BetweenNodes,
		}
	}

	pub(crate) fn is_trunk(self) -> bool {
		matches!(self, Self::Trunk)
	}

	pub(crate) fn is_descender(self) -> bool {
		matches!(self, Self::Descender)
	}
}

/// Keep roughly this fraction of descender sticks on Low (stable every-Nth sample).
pub(crate) const LOW_DESCENDER_KEEP_EVERY: usize = 4;

pub(crate) fn stick_role_for_segment(
	_segment: &BallStickSegment<'_>,
	parent: &SopesBanyanChain,
) -> StickLodRole {
	StickLodRole::from_parent_phase(&parent.phase)
}

/// Medium: always keep trunk; keep other sticks only in the outer half of the footprint.
pub(crate) fn keep_stick_on_medium(
	role: StickLodRole,
	segment: &BallStickSegment<'_>,
	tree_radius: f32,
) -> bool {
	if role.is_trunk() {
		return true;
	}
	segment.horizontal_radius() >= tree_radius.max(1e-4) * 0.5
}

/// Low: trunk + a thinned subset of descenders (no branch / flair sticks).
pub(crate) fn keep_stick_on_low(role: StickLodRole, descender_index: &mut usize) -> bool {
	if role.is_trunk() {
		return true;
	}
	if !role.is_descender() {
		return false;
	}
	let keep = *descender_index % LOW_DESCENDER_KEEP_EVERY == 0;
	*descender_index += 1;
	keep
}
