//! Interrogative realization: particles, inversion, and verbal marking.

use maybraid_language_core::{Mood, ParticleDomain, SemanticNode, SurfaceConstituent};

use crate::clause::WordOrder;
use crate::marking::Marking;

/// How a grammar marks [`Mood::Interrogative`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestionStrategy {
	Unmarked,
	Particle { form: &'static str, before: bool },
	Invert,
	InvertWithParticle { form: &'static str, before: bool },
	Suffix { form: &'static str },
}

impl QuestionStrategy {
	pub fn unmarked() -> Self {
		Self::Unmarked
	}

	pub fn inverts(self) -> bool {
		matches!(self, Self::Invert | Self::InvertWithParticle { .. })
	}

	pub fn order(self, base: WordOrder, mood: Mood) -> WordOrder {
		if mood == Mood::Interrogative && self.inverts() {
			WordOrder::Vso
		} else {
			base
		}
	}

	pub fn apply_predicate(
		self,
		predicate: &mut Vec<SurfaceConstituent>,
		mood: Mood,
		host: SemanticNode,
	) {
		if mood != Mood::Interrogative {
			return;
		}
		if let Self::Suffix { form } = self {
			Marking::Suffix { form }.apply(predicate, ParticleDomain::Mood, Some(host));
		}
	}

	pub fn apply_clause(self, words: &mut Vec<SurfaceConstituent>, mood: Mood) {
		if mood != Mood::Interrogative {
			return;
		}
		let (form, before) = match self {
			Self::Particle { form, before } | Self::InvertWithParticle { form, before } => {
				(form, before)
			}
			_ => return,
		};
		Marking::Particle { form, before }.apply(words, ParticleDomain::Mood, None);
	}
}
