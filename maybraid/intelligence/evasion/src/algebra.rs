/// Inputs used to rank one assailant.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AssailantFactors {
	pub threat: f32,
	pub proximity: f32,
	pub uncertainty: f32,
	pub bias: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssailantFactor {
	Threat,
	Proximity,
	Uncertainty,
	Bias,
}

impl AssailantFactor {
	pub(crate) fn add_to(self, factors: &mut AssailantFactors, value: f32) {
		match self {
			Self::Threat => factors.threat += value,
			Self::Proximity => factors.proximity += value,
			Self::Uncertainty => factors.uncertainty += value,
			Self::Bias => factors.bias += value,
		}
	}
}

/// Per-user coefficients. Uncertainty is a cost; closer assailants should rank higher.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssailantAlgebra {
	pub threat: f32,
	pub proximity: f32,
	pub uncertainty: f32,
	pub bias: f32,
}

impl AssailantAlgebra {
	pub fn score(self, factors: AssailantFactors) -> f32 {
		self.threat * factors.threat + self.proximity * factors.proximity + self.bias * factors.bias
			- self.uncertainty * factors.uncertainty
	}
}

impl Default for AssailantAlgebra {
	fn default() -> Self {
		Self { threat: 4.0, proximity: 3.0, uncertainty: 2.0, bias: 1.0 }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn closer_and_more_threatening_ranks_higher() -> anyhow::Result<()> {
		let algebra = AssailantAlgebra::default();
		let near = AssailantFactors { threat: 1.0, proximity: 1.0, ..Default::default() };
		let far = AssailantFactors { threat: 1.0, proximity: 0.2, ..Default::default() };
		assert!(algebra.score(near) > algebra.score(far));
		Ok(())
	}
}
