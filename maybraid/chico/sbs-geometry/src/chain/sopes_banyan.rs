use crate::{chain::Hysteresis, BallStickNode};
use procedural_common::NoiseConfig;

/// Marks a flair up segment.
pub struct StartFlairUp {
	projection: ProjectionHelper,
	/// When projection helper is already implemented we won't need a ball stick node,
	/// Projection helper should embed one as field.
	node: BallStickNode,
}

impl StartFlairUp {
	pub fn sample_from_candidate(
		candidate: SopesBanyanPhase,
		noise: &NoiseConfig,
	) -> SopesBanyanPhase {
		if candidate.is_branch_out() && candidate.budget_remaining() < 2 {
			SopesBanyanPhase::StartFlairUp(StartFlairUp {
				projection: ProjectionHelper::up()
					.with_degrees_freedom(candidate.degrees_freedom()),
				node: candidate.clone(),
			})
		} else {
			candidate
		}
	}

    fn project_to_end(&self) -> EndFlairUp {
        EndFlairUp {
            node: self.projection.project()
        }
    }
}

/// Simply marks the node at which a flair up segment ends.
pub struct EndFlairUp {
	node: BallStickNode,
}

/// Marks a node that has been descended (as part of a banyan descender)
pub struct StartDescender {
	projection: ProjectionHelper,
	node: BallStickNode,
}

impl StartDescender {
	pub fn sample_from_candidate(
		candidate: SopesBanyanPhase,
		noise: &NoiseConfig,
		banyan_height: f32,
		descender_threshold: f32,
	) -> SopesBanyanPhase {
		let node = candidate.node();
		if candidate.is_branch_out()
			&& noise.sample_unit_3d(node.position.x, node.position.y, node.position.z)
				< descender_threshold
		{
			SopesBanyanPhase::StartDescender(StartDescender {
				projection: ProjectionHelper::straight_down(node.position, banyan_height * 2.0),
				node: node.clone(),
			})
		} else {
			candidate
		}
	}

    fn project_to_end(&self) -> EndDescender {
        EndDescender {
            node: self.projection.project()
        }
    }
}

/// Simply marks the node at which a descender segment ends.
pub struct EndDescender {
	node: BallStickNode,
}

impl SopesBanyanDescender {
	pub fn sample_from_candidate(
		candidate: &BallStickNode,
		banyan_height: f32,
		descender_threshold: f32,
	) -> Self {
		Self { node: candidate.clone() }
	}
}

pub enum SopesBanyanPhase {
	BranchOut(DepthBudget<BranchOut>),
	StartFlairUp(StartFlairUp),
	EndFlairUp(EndFlairUp),
	StartDescender(StartDescender),
	EndDescender(EndDescender),
}

impl SopesBanyanPhase {
	fn node(&self) -> &BallStickNode {
		match self {
			Self::BranchOut(branch_out) => &branch_out.node,
			Self::StartFlairUp(start_flair_up) => &start_flair_up.node,
			Self::EndFlairUp(end_flair_up) => &end_flair_up.node,
			Self::StartDescender(start_descender) => &start_descender.node,
			Self::EndDescender(end_descender) => &end_descender.node,
		}
	}

	fn candidate_into(
		self,
		candidate: &BallStickNode,
		noise: &NoiseConfig,
		banyan_height: f32,
		descender_threshold: f32,
	) -> Self {
		match self {
			Self::BranchOut(branch_out) => {
				let maybe_flair = StartFlairUp::sample_from_candidate(self, noise);
				let maybe_descender = StartDescender::sample_from_candidate(
					maybe_flair,
					noise,
					banyan_height,
					descender_threshold,
				);
				maybe_descender.map(SopesBanyanPhase::StartDescender)
			}
			_ => self,
		}
	}
}

pub struct SopesBanyanChain {
	noise: NoiseConfig,
	banyan_height: f32,
	descender_threshold: f32,
	phase: SopesBanyanPhase,
}

impl Hysteresis for SopesBanyanChain {
	fn ball_stick_node(&self) -> BallStickNode {
		self.phase.node().clone()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		match self.phase {
			SopesBanyanPhase::BranchOut(branch_out) => branch_out
				.next_hysteresis()
				.map(SopesBanyanPhase::BranchOut)
				.map(|phase| Self { phase.candidate_into(noise, banyan_height, descender_threshold), ..self.clone() }),
			SopesBanyanPhase::StartFlairUp(start_flair_up) => vec![Self { phase: start_flair_up.project_to_end(), ..self.clone() }],
			SopesBanyanPhase::EndFlairUp(end_flair_up) => vec![],
			SopesBanyanPhase::StartDescender(start_descender) => vec![Self { phase: start_descender.project_to_end(), ..self.clone() }],
			SopesBanyanPhase::EndDescender(end_descender) => vec![],
		}
	}
}
