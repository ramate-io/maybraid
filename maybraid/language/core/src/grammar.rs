//! Stub surface linearizer.
//!
//! This is not a grammar core. It has no agreement, case, conjugation, or
//! sandhi. It only orders already-chosen base terms into a
//! [`GrammaticalOutput`]. [`IpaUtterance`] is the phonemic view of that output.

use crate::marshall::SemanticNode;
use crate::output::LexicalOutput;
use crate::utterance::{
	Clause, ClauseId, Number, Polarity, Referent, ReferentId, SemanticRole, SemanticValue,
};

/// Linearized grammatical form. Morphology and a fuller grammar can extend this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrammaticalOutput {
	pub words: Vec<String>,
}

impl GrammaticalOutput {
	pub fn new(words: Vec<String>) -> Self {
		Self { words }
	}

	/// Phonemic rendering of this linearization.
	pub fn ipa(&self) -> IpaUtterance {
		IpaUtterance::from(self)
	}
}

impl std::fmt::Display for GrammaticalOutput {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.words.join(" "))
	}
}

/// `/word word word/` view of a [`GrammaticalOutput`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpaUtterance {
	pub words: Vec<String>,
}

impl IpaUtterance {
	pub fn new(words: Vec<String>) -> Self {
		Self { words }
	}
}

impl From<&GrammaticalOutput> for IpaUtterance {
	fn from(output: &GrammaticalOutput) -> Self {
		Self { words: output.words.clone() }
	}
}

impl std::fmt::Display for IpaUtterance {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "/{}/", self.words.join(" "))
	}
}

/// Role-bearing function words. A later grammar issue should generate these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleParticles {
	pub recipient: &'static str,
	pub source: &'static str,
	pub goal: &'static str,
	pub plural: &'static str,
	pub negative: &'static str,
}

impl RoleParticles {
	pub fn compositional() -> Self {
		Self { recipient: "tʰə", source: "kə", goal: "ŋə", plural: "ɲi", negative: "ma" }
	}

	pub fn root_heavy() -> Self {
		Self { recipient: "ʔu", source: "ħe", goal: "ɡo", plural: "riː", negative: "nu" }
	}
}

/// Where the predicate sits relative to its arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClauseOrder {
	/// Agent — predicate — complements (SVO-like).
	VerbMedial,
	/// Agent — complements — predicate (SOV-like).
	VerbFinal,
}

/// Where a relative clause sits relative to its head noun.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativePlacement {
	AfterHead,
	BeforeHead,
}

/// Where modifiers sit relative to the noun.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifierPlacement {
	BeforeNoun,
	AfterNoun,
}

/// Fixed linearization rules for one POC language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceGrammar {
	pub clause_order: ClauseOrder,
	pub relative: RelativePlacement,
	pub modifier: ModifierPlacement,
	pub particles: RoleParticles,
}

impl SurfaceGrammar {
	pub fn compositional() -> Self {
		Self {
			clause_order: ClauseOrder::VerbMedial,
			relative: RelativePlacement::AfterHead,
			modifier: ModifierPlacement::BeforeNoun,
			particles: RoleParticles::compositional(),
		}
	}

	pub fn root_heavy() -> Self {
		Self {
			clause_order: ClauseOrder::VerbFinal,
			relative: RelativePlacement::BeforeHead,
			modifier: ModifierPlacement::BeforeNoun,
			particles: RoleParticles::root_heavy(),
		}
	}

	pub fn realize(self, output: &LexicalOutput) -> GrammaticalOutput {
		let mut words = Vec::new();
		for (index, root) in output.utterance.roots.iter().enumerate() {
			if index > 0 {
				words.push("‖".to_owned());
			}
			self.emit_clause(&mut words, output, *root, None);
		}
		GrammaticalOutput::new(words)
	}

	fn emit_clause(
		&self,
		words: &mut Vec<String>,
		output: &LexicalOutput,
		clause_id: ClauseId,
		bound: Option<ReferentId>,
	) {
		let Some(clause) = output.utterance.clauses.get(clause_id) else {
			return;
		};
		let mut subject = Vec::new();
		let mut complements = Vec::new();
		self.collect_arguments(&mut subject, &mut complements, output, clause, bound);

		let predicate =
			output.ipa_for(SemanticNode::Predicate(clause_id)).unwrap_or("…").to_owned();

		match self.clause_order {
			ClauseOrder::VerbMedial => {
				words.extend(subject);
				if clause.polarity == Polarity::Negative {
					words.push(self.particles.negative.to_owned());
				}
				words.push(predicate);
				words.extend(complements);
			}
			ClauseOrder::VerbFinal => {
				words.extend(subject);
				words.extend(complements);
				if clause.polarity == Polarity::Negative {
					words.push(self.particles.negative.to_owned());
				}
				words.push(predicate);
			}
		}
	}

	fn collect_arguments(
		&self,
		subject: &mut Vec<String>,
		complements: &mut Vec<String>,
		output: &LexicalOutput,
		clause: &Clause,
		bound: Option<ReferentId>,
	) {
		for role in [SemanticRole::Agent, SemanticRole::Experiencer, SemanticRole::Possessor] {
			if let Some(value) = clause_value(clause, role) {
				self.emit_value(subject, output, value, bound);
			}
		}

		for (role, particle) in [
			(SemanticRole::Patient, None),
			(SemanticRole::Theme, None),
			(SemanticRole::Classification, None),
			(SemanticRole::Content, None),
			(SemanticRole::Recipient, Some(self.particles.recipient)),
			(SemanticRole::Beneficiary, Some(self.particles.recipient)),
			(SemanticRole::Source, Some(self.particles.source)),
			(SemanticRole::Goal, Some(self.particles.goal)),
			(SemanticRole::Instrument, None),
			(SemanticRole::Location, None),
			(SemanticRole::Manner, None),
			(SemanticRole::Rate, None),
			(SemanticRole::Time, None),
			(SemanticRole::Cause, None),
		] {
			if let Some(value) = clause_value(clause, role) {
				if let Some(particle) = particle {
					if !is_bound(value, bound) {
						complements.push(particle.to_owned());
					}
				}
				self.emit_value(complements, output, value, bound);
			}
		}
	}

	fn emit_value(
		&self,
		words: &mut Vec<String>,
		output: &LexicalOutput,
		value: SemanticValue,
		bound: Option<ReferentId>,
	) {
		match value {
			SemanticValue::Referent(referent) if Some(referent) == bound => {}
			SemanticValue::Referent(referent) => {
				self.emit_referent(words, output, referent);
			}
			SemanticValue::Clause(clause) => {
				self.emit_clause(words, output, clause, bound);
			}
		}
	}

	fn emit_referent(
		&self,
		words: &mut Vec<String>,
		output: &LexicalOutput,
		referent_id: ReferentId,
	) {
		let Some(referent) = output.utterance.referents.get(referent_id) else {
			return;
		};
		let mut head = Vec::new();
		self.emit_noun_phrase(&mut head, output, referent_id, referent);

		let mut relatives = Vec::new();
		for (index, clause) in referent.relative_clauses.iter().enumerate() {
			if index > 0 || !relatives.is_empty() {
				relatives.push("|".to_owned());
			}
			self.emit_clause(&mut relatives, output, *clause, Some(referent_id));
		}

		match self.relative {
			RelativePlacement::AfterHead => {
				words.extend(head);
				if !relatives.is_empty() {
					words.push("|".to_owned());
					words.extend(relatives);
				}
			}
			RelativePlacement::BeforeHead => {
				if !relatives.is_empty() {
					words.extend(relatives);
					words.push("|".to_owned());
				}
				words.extend(head);
			}
		}
	}

	fn emit_noun_phrase(
		&self,
		words: &mut Vec<String>,
		output: &LexicalOutput,
		referent_id: ReferentId,
		referent: &Referent,
	) {
		let mut modifiers = Vec::new();
		for index in 0..referent.modifiers.len() {
			if let Some(ipa) =
				output.ipa_for(SemanticNode::Modifier { referent: referent_id, index })
			{
				modifiers.push(ipa.to_owned());
			}
		}
		let noun = output.ipa_for(SemanticNode::Referent(referent_id)).unwrap_or("…").to_owned();
		match self.modifier {
			ModifierPlacement::BeforeNoun => {
				words.extend(modifiers);
				words.push(noun);
			}
			ModifierPlacement::AfterNoun => {
				words.push(noun);
				words.extend(modifiers);
			}
		}
		if matches!(referent.number, Number::Plural | Number::Many) {
			words.push(self.particles.plural.to_owned());
		}
	}
}

impl LexicalOutput {
	pub fn realize(&self, grammar: SurfaceGrammar) -> GrammaticalOutput {
		grammar.realize(self)
	}

	pub fn ipa_for(&self, node: SemanticNode) -> Option<&str> {
		self.lexicalizations
			.iter()
			.find(|item| item.source.source == node)
			.map(|item| item.realization.term.ipa.as_str())
	}
}

fn clause_value(clause: &Clause, role: SemanticRole) -> Option<SemanticValue> {
	clause
		.arguments
		.iter()
		.find(|argument| argument.role == role)
		.map(|argument| argument.value)
}

fn is_bound(value: SemanticValue, bound: Option<ReferentId>) -> bool {
	matches!(value, SemanticValue::Referent(referent) if Some(referent) == bound)
}
