//! Typological building-block grammars. Not simulations of particular languages.

use maybraid_language_core::{ModifierPlacement, RelativePlacement};

use crate::agreement::AgreementStrategy;
use crate::alignment::Alignment;
use crate::clause::WordOrder;
use crate::composite::CompositeGrammar;
use crate::copula::CopulaStrategy;
use crate::information::InformationStrategy;
use crate::linking::ClauseLinkStrategy;
use crate::marking::{AdpositionStrategy, CaseStrategy, Marking, PolarityStrategy, TamStrategy};
use crate::nominal::{DefinitenessStrategy, NumberStrategy};
use crate::predicate::{PredicateStrategy, SerialStrategy};
use crate::question::QuestionStrategy;

pub fn isolating_svo() -> CompositeGrammar {
	CompositeGrammar {
		alignment: Alignment::NominativeAccusative,
		order: WordOrder::Svo,
		relative: RelativePlacement::AfterHead,
		modifiers: ModifierPlacement::BeforeNoun,
		modifier_linker: Some("de"),
		case: CaseStrategy::unmarked(),
		adpositions: AdpositionStrategy::Preposition,
		number: NumberStrategy::Particle { form: "men" },
		definiteness: DefinitenessStrategy::Article { definite: "li", indefinite: "" },
		tam: TamStrategy {
			past: Marking::Particle { form: "le", before: false },
			future: Marking::Particle { form: "hui", before: true },
			progressive: Marking::Particle { form: "zai", before: true },
			perfect: Marking::Zero,
			imperative: Marking::Zero,
		},
		polarity: PolarityStrategy::preverbal("bu"),
		predicate: PredicateStrategy::Simple,
		serial: SerialStrategy::None,
		information: InformationStrategy::unmarked(),
		question: QuestionStrategy::Particle { form: "ma", before: false },
		copula: CopulaStrategy::Zero,
		agreement: AgreementStrategy::unmarked(),
		linking: ClauseLinkStrategy::Complementizer { form: "shuo", before: true },
		coordinator: None,
	}
}

pub fn agglutinative_sov() -> CompositeGrammar {
	CompositeGrammar {
		alignment: Alignment::NominativeAccusative,
		order: WordOrder::Sov,
		relative: RelativePlacement::BeforeHead,
		modifiers: ModifierPlacement::BeforeNoun,
		modifier_linker: None,
		case: CaseStrategy {
			nominative: Marking::Zero,
			accusative: Marking::Suffix { form: "o" },
			ergative: Marking::Zero,
			absolutive: Marking::Zero,
			dative: Marking::Suffix { form: "e" },
			genitive: Marking::Suffix { form: "n" },
			instrumental: Marking::Suffix { form: "de" },
			locative: Marking::Suffix { form: "da" },
			ablative: Marking::Suffix { form: "dan" },
			allative: Marking::Suffix { form: "a" },
		},
		adpositions: AdpositionStrategy::Postposition,
		number: NumberStrategy::Suffix { form: "lar" },
		definiteness: DefinitenessStrategy::Zero,
		tam: TamStrategy {
			past: Marking::Suffix { form: "di" },
			future: Marking::Suffix { form: "acak" },
			progressive: Marking::Suffix { form: "iyor" },
			perfect: Marking::Zero,
			imperative: Marking::Zero,
		},
		polarity: PolarityStrategy::postverbal("ma"),
		predicate: PredicateStrategy::Simple,
		serial: SerialStrategy::None,
		information: InformationStrategy::unmarked(),
		question: QuestionStrategy::Suffix { form: "mi" },
		copula: CopulaStrategy::Stative,
		agreement: AgreementStrategy::subject_number(Marking::Suffix { form: "ler" }),
		linking: ClauseLinkStrategy::Converb { suffix: "te" },
		coordinator: None,
	}
}

pub fn fusional_svo() -> CompositeGrammar {
	CompositeGrammar {
		alignment: Alignment::NominativeAccusative,
		order: WordOrder::Svo,
		relative: RelativePlacement::AfterHead,
		modifiers: ModifierPlacement::BeforeNoun,
		modifier_linker: None,
		case: CaseStrategy {
			nominative: Marking::Zero,
			accusative: Marking::Suffix { form: "m" },
			ergative: Marking::Zero,
			absolutive: Marking::Zero,
			dative: Marking::Suffix { form: "i" },
			..CaseStrategy::unmarked()
		},
		adpositions: AdpositionStrategy::None,
		number: NumberStrategy::Suffix { form: "s" },
		definiteness: DefinitenessStrategy::Article { definite: "el", indefinite: "un" },
		tam: TamStrategy {
			past: Marking::Suffix { form: "o" },
			future: Marking::Suffix { form: "a" },
			progressive: Marking::Zero,
			perfect: Marking::Zero,
			imperative: Marking::Zero,
		},
		polarity: PolarityStrategy::preverbal("no"),
		predicate: PredicateStrategy::Simple,
		serial: SerialStrategy::None,
		information: InformationStrategy::unmarked(),
		question: QuestionStrategy::InvertWithParticle { form: "li", before: true },
		copula: CopulaStrategy::Obligatory { form: "est" },
		agreement: AgreementStrategy {
			first: Marking::Suffix { form: "o" },
			third: Marking::Suffix { form: "t" },
			..AgreementStrategy::subject_number(Marking::Suffix { form: "n" })
		},
		linking: ClauseLinkStrategy::Complementizer { form: "ke", before: true },
		coordinator: Some("et"),
	}
}

pub fn particle_heavy_topic_prominent() -> CompositeGrammar {
	CompositeGrammar {
		alignment: Alignment::NominativeAccusative,
		order: WordOrder::Svo,
		relative: RelativePlacement::AfterHead,
		modifiers: ModifierPlacement::BeforeNoun,
		modifier_linker: None,
		case: CaseStrategy::unmarked(),
		adpositions: AdpositionStrategy::Postposition,
		number: NumberStrategy::Particle { form: "tachi" },
		definiteness: DefinitenessStrategy::Zero,
		tam: TamStrategy {
			past: Marking::Particle { form: "ta", before: false },
			future: Marking::Zero,
			progressive: Marking::Particle { form: "teiru", before: false },
			perfect: Marking::Zero,
			imperative: Marking::Zero,
		},
		polarity: PolarityStrategy::postverbal("nai"),
		predicate: PredicateStrategy::Simple,
		serial: SerialStrategy::None,
		information: InformationStrategy {
			topic_particle: Some("wa"),
			focus_particle: Some("ga"),
			front_topic: true,
		},
		question: QuestionStrategy::Particle { form: "ka", before: false },
		copula: CopulaStrategy::Zero,
		agreement: AgreementStrategy::unmarked(),
		linking: ClauseLinkStrategy::SubordinateParticle { form: "to", before: true },
		coordinator: None,
	}
}

pub fn serial_verb() -> CompositeGrammar {
	CompositeGrammar {
		alignment: Alignment::NominativeAccusative,
		order: WordOrder::Svo,
		relative: RelativePlacement::AfterHead,
		modifiers: ModifierPlacement::BeforeNoun,
		modifier_linker: None,
		case: CaseStrategy::unmarked(),
		adpositions: AdpositionStrategy::None,
		number: NumberStrategy::Zero,
		definiteness: DefinitenessStrategy::Zero,
		tam: TamStrategy::unmarked(),
		polarity: PolarityStrategy::unmarked(),
		predicate: PredicateStrategy::Simple,
		serial: SerialStrategy::InstrumentAsTake { take: "gba" },
		information: InformationStrategy::unmarked(),
		question: QuestionStrategy::unmarked(),
		copula: CopulaStrategy::Stative,
		agreement: AgreementStrategy::unmarked(),
		linking: ClauseLinkStrategy::Chain,
		coordinator: None,
	}
}

pub fn ergative_vso() -> CompositeGrammar {
	CompositeGrammar {
		alignment: Alignment::ErgativeAbsolutive,
		order: WordOrder::Vso,
		relative: RelativePlacement::AfterHead,
		modifiers: ModifierPlacement::AfterNoun,
		modifier_linker: None,
		case: CaseStrategy {
			ergative: Marking::Particle { form: "ek", before: false },
			absolutive: Marking::Zero,
			..CaseStrategy::unmarked()
		},
		adpositions: AdpositionStrategy::Preposition,
		number: NumberStrategy::Particle { form: "n" },
		definiteness: DefinitenessStrategy::Zero,
		tam: TamStrategy {
			past: Marking::Particle { form: "do", before: true },
			..TamStrategy::unmarked()
		},
		polarity: PolarityStrategy::preverbal("cha"),
		predicate: PredicateStrategy::Simple,
		serial: SerialStrategy::None,
		information: InformationStrategy::unmarked(),
		question: QuestionStrategy::Particle { form: "ne", before: false },
		copula: CopulaStrategy::Locative { form: "ta" },
		agreement: AgreementStrategy::unmarked(),
		linking: ClauseLinkStrategy::zero(),
		coordinator: None,
	}
}

pub fn separable_svo() -> CompositeGrammar {
	CompositeGrammar {
		alignment: Alignment::NominativeAccusative,
		order: WordOrder::Svo,
		relative: RelativePlacement::AfterHead,
		modifiers: ModifierPlacement::BeforeNoun,
		modifier_linker: None,
		case: CaseStrategy::unmarked(),
		adpositions: AdpositionStrategy::Preposition,
		number: NumberStrategy::Zero,
		definiteness: DefinitenessStrategy::Zero,
		tam: TamStrategy::unmarked(),
		polarity: PolarityStrategy::unmarked(),
		predicate: PredicateStrategy::Separable { particle: "an" },
		serial: SerialStrategy::None,
		information: InformationStrategy::unmarked(),
		question: QuestionStrategy::unmarked(),
		copula: CopulaStrategy::Stative,
		agreement: AgreementStrategy::unmarked(),
		linking: ClauseLinkStrategy::zero(),
		coordinator: None,
	}
}

/// Same particle inventory as [`maybraid_language_core::SurfaceGrammar::compositional`].
pub fn basic_compositional() -> CompositeGrammar {
	use maybraid_language_core::RoleParticles;
	let particles = RoleParticles::compositional();
	CompositeGrammar {
		alignment: Alignment::NominativeAccusative,
		order: WordOrder::Svo,
		relative: RelativePlacement::AfterHead,
		modifiers: ModifierPlacement::BeforeNoun,
		modifier_linker: None,
		case: CaseStrategy {
			dative: Marking::Particle { form: particles.recipient, before: true },
			allative: Marking::Particle { form: particles.goal, before: true },
			ablative: Marking::Particle { form: particles.source, before: true },
			..CaseStrategy::unmarked()
		},
		adpositions: AdpositionStrategy::None,
		number: NumberStrategy::Particle { form: particles.plural },
		definiteness: DefinitenessStrategy::Zero,
		tam: TamStrategy::unmarked(),
		polarity: PolarityStrategy::preverbal(particles.negative),
		predicate: PredicateStrategy::Simple,
		serial: SerialStrategy::None,
		information: InformationStrategy::unmarked(),
		question: QuestionStrategy::unmarked(),
		copula: CopulaStrategy::Stative,
		agreement: AgreementStrategy::unmarked(),
		linking: ClauseLinkStrategy::zero(),
		coordinator: None,
	}
}
