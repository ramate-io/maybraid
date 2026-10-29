//! Map semantic participants onto grammatical relations.

use maybraid_language_core::{GrammaticalRelation, SemanticRole};

/// Morphosyntactic alignment. Applied before constituent order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
	NominativeAccusative,
	ErgativeAbsolutive,
	/// Agentive participants are subjects; patientive ones are objects.
	ActiveStative,
	/// Agent, patient, and intransitive-like participants stay distinct.
	Tripartite,
	/// No privileged mapping of Agent onto Subject.
	Neutral,
}

impl Alignment {
	pub fn relation(self, role: SemanticRole) -> GrammaticalRelation {
		match self {
			Self::NominativeAccusative | Self::Neutral => nominative_relation(role),
			Self::ErgativeAbsolutive => match role {
				SemanticRole::Agent => GrammaticalRelation::Oblique,
				SemanticRole::Patient | SemanticRole::Theme | SemanticRole::Classification => {
					GrammaticalRelation::Subject
				}
				SemanticRole::Experiencer | SemanticRole::Possessor => GrammaticalRelation::Subject,
				SemanticRole::Recipient | SemanticRole::Beneficiary => {
					GrammaticalRelation::IndirectObject
				}
				_ => GrammaticalRelation::Oblique,
			},
			Self::ActiveStative => match role {
				SemanticRole::Agent => GrammaticalRelation::Subject,
				SemanticRole::Patient
				| SemanticRole::Theme
				| SemanticRole::Classification
				| SemanticRole::Experiencer => GrammaticalRelation::Object,
				SemanticRole::Possessor => GrammaticalRelation::Subject,
				SemanticRole::Recipient | SemanticRole::Beneficiary => {
					GrammaticalRelation::IndirectObject
				}
				_ => GrammaticalRelation::Oblique,
			},
			Self::Tripartite => match role {
				SemanticRole::Agent => GrammaticalRelation::Oblique,
				SemanticRole::Patient | SemanticRole::Theme | SemanticRole::Classification => {
					GrammaticalRelation::Object
				}
				SemanticRole::Experiencer | SemanticRole::Possessor => GrammaticalRelation::Subject,
				SemanticRole::Recipient | SemanticRole::Beneficiary => {
					GrammaticalRelation::IndirectObject
				}
				_ => GrammaticalRelation::Oblique,
			},
		}
	}

	pub fn case_for(self, relation: GrammaticalRelation, role: SemanticRole) -> CaseLabel {
		match self {
			Self::NominativeAccusative => match relation {
				GrammaticalRelation::Subject => CaseLabel::Nominative,
				GrammaticalRelation::Object => CaseLabel::Accusative,
				GrammaticalRelation::IndirectObject => CaseLabel::Dative,
				GrammaticalRelation::Oblique => case_for_oblique(role),
			},
			Self::ErgativeAbsolutive => match (relation, role) {
				(GrammaticalRelation::Oblique, SemanticRole::Agent) => CaseLabel::Ergative,
				(GrammaticalRelation::Subject, _) => CaseLabel::Absolutive,
				(GrammaticalRelation::IndirectObject, _) => CaseLabel::Dative,
				_ => case_for_oblique(role),
			},
			Self::ActiveStative => match role {
				SemanticRole::Agent => CaseLabel::Nominative,
				SemanticRole::Patient
				| SemanticRole::Theme
				| SemanticRole::Classification
				| SemanticRole::Experiencer => CaseLabel::Accusative,
				SemanticRole::Recipient | SemanticRole::Beneficiary => CaseLabel::Dative,
				_ => case_for_oblique(role),
			},
			Self::Tripartite => match role {
				SemanticRole::Agent => CaseLabel::Ergative,
				SemanticRole::Patient | SemanticRole::Theme | SemanticRole::Classification => {
					CaseLabel::Accusative
				}
				SemanticRole::Experiencer | SemanticRole::Possessor => CaseLabel::Nominative,
				SemanticRole::Recipient | SemanticRole::Beneficiary => CaseLabel::Dative,
				_ => case_for_oblique(role),
			},
			Self::Neutral => CaseLabel::Nominative,
		}
	}
}

fn nominative_relation(role: SemanticRole) -> GrammaticalRelation {
	match role {
		SemanticRole::Agent | SemanticRole::Experiencer | SemanticRole::Possessor => {
			GrammaticalRelation::Subject
		}
		SemanticRole::Patient | SemanticRole::Theme | SemanticRole::Classification => {
			GrammaticalRelation::Object
		}
		SemanticRole::Recipient | SemanticRole::Beneficiary => GrammaticalRelation::IndirectObject,
		_ => GrammaticalRelation::Oblique,
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseLabel {
	Nominative,
	Accusative,
	Ergative,
	Absolutive,
	Dative,
	Genitive,
	Instrumental,
	Locative,
	Ablative,
	Allative,
}

fn case_for_oblique(role: SemanticRole) -> CaseLabel {
	match role {
		SemanticRole::Instrument => CaseLabel::Instrumental,
		SemanticRole::Location => CaseLabel::Locative,
		SemanticRole::Source => CaseLabel::Ablative,
		SemanticRole::Goal => CaseLabel::Allative,
		SemanticRole::Possessor => CaseLabel::Genitive,
		_ => CaseLabel::Locative,
	}
}
