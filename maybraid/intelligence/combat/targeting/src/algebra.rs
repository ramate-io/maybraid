/// Semantic inputs used to rank one active target.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TargetFactors {
	pub hostility: f32,
	pub threat: f32,
	pub opportunity: f32,
	pub continuity: f32,
	pub uncertainty: f32,
	pub bias: f32,
}

/// A factor that can receive a temporary influence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetFactor {
	Hostility,
	Threat,
	Opportunity,
	Continuity,
	Uncertainty,
	Bias,
}

impl TargetFactor {
	pub(crate) fn add_to(self, factors: &mut TargetFactors, value: f32) {
		match self {
			Self::Hostility => factors.hostility += value,
			Self::Threat => factors.threat += value,
			Self::Opportunity => factors.opportunity += value,
			Self::Continuity => factors.continuity += value,
			Self::Uncertainty => factors.uncertainty += value,
			Self::Bias => factors.bias += value,
		}
	}
}

/// Per-user coefficients for reducing target factors to one rank weight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetAlgebra {
	pub hostility: f32,
	pub threat: f32,
	pub opportunity: f32,
	pub continuity: f32,
	pub uncertainty: f32,
	pub bias: f32,
}

impl TargetAlgebra {
	/// Scores a target. Uncertainty is a cost; every other term is a benefit.
	pub fn score(self, factors: TargetFactors) -> f32 {
		self.hostility * factors.hostility
			+ self.threat * factors.threat
			+ self.opportunity * factors.opportunity
			+ self.continuity * factors.continuity
			+ self.bias * factors.bias
			- self.uncertainty * factors.uncertainty
	}
}

impl Default for TargetAlgebra {
	fn default() -> Self {
		Self {
			hostility: 4.0,
			threat: 2.0,
			opportunity: 3.0,
			continuity: 2.0,
			uncertainty: 2.5,
			bias: 1.0,
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{TargetAlgebra, TargetFactors};

	#[test]
	fn algebra_rewards_benefits_and_penalizes_uncertainty() -> anyhow::Result<()> {
		let algebra = TargetAlgebra::default();
		let favorable = TargetFactors {
			hostility: 1.0,
			opportunity: 1.0,
			continuity: 1.0,
			..Default::default()
		};
		let uncertain = TargetFactors { uncertainty: 2.0, ..favorable };

		assert_eq!(algebra.score(favorable), 9.0);
		assert_eq!(algebra.score(uncertain), 4.0);
		Ok(())
	}
}
