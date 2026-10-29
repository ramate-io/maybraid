//! Typed semantic graph for a single utterance.

use slotmap::{new_key_type, SlotMap};

use crate::concept::ConceptId;

new_key_type! {
	/// Stable handle for a discourse referent.
	pub struct ReferentId;
	/// Stable handle for a clausal predication.
	pub struct ClauseId;
}

/// Semantic graph: clauses, referents, and root predications.
#[derive(Clone, Debug, Default)]
pub struct Utterance {
	pub referents: SlotMap<ReferentId, Referent>,
	pub clauses: SlotMap<ClauseId, Clause>,
	pub roots: Vec<ClauseId>,
	pub information: InformationStructure,
}

impl Utterance {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_referent(&mut self, referent: Referent) -> ReferentId {
		self.referents.insert(referent)
	}

	pub fn add_clause(&mut self, clause: Clause) -> ClauseId {
		self.clauses.insert(clause)
	}

	pub fn push_root(&mut self, clause: ClauseId) {
		self.roots.push(clause);
	}

	pub fn with_topic(mut self, referent: ReferentId) -> Self {
		self.information.topic = Some(referent);
		self
	}

	pub fn with_focus(mut self, focus: FocusTarget) -> Self {
		self.information.focus = Some(focus);
		self
	}
}

/// Discourse-level topic/focus. Realization is a grammatical decision.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InformationStructure {
	pub topic: Option<ReferentId>,
	pub focus: Option<FocusTarget>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusTarget {
	Referent(ReferentId),
	Predicate(ClauseId),
}

/// Predication with semantic-role arguments.
#[derive(Clone, Debug)]
pub struct Clause {
	pub predicate: ConceptId,
	pub arguments: Vec<Argument>,
	pub tense: Tense,
	pub aspect: Aspect,
	pub mood: Mood,
	pub polarity: Polarity,
}

impl Clause {
	pub fn new(predicate: ConceptId) -> Self {
		Self {
			predicate,
			arguments: Vec::new(),
			tense: Tense::Unspecified,
			aspect: Aspect::Unspecified,
			mood: Mood::Indicative,
			polarity: Polarity::Affirmative,
		}
	}

	pub fn with_argument(mut self, role: SemanticRole, value: SemanticValue) -> Self {
		self.arguments.push(Argument { role, value });
		self
	}

	pub fn past(mut self) -> Self {
		self.tense = Tense::Past;
		self
	}

	pub fn negated(mut self) -> Self {
		self.polarity = Polarity::Negative;
		self
	}

	pub fn interrogative(mut self) -> Self {
		self.mood = Mood::Interrogative;
		self
	}

	pub fn has_role(&self, role: SemanticRole) -> bool {
		self.arguments.iter().any(|argument| argument.role == role)
	}
}

/// Role-bearing argument of a clause.
#[derive(Clone, Debug)]
pub struct Argument {
	pub role: SemanticRole,
	pub value: SemanticValue,
}

/// Clause argument payload. Coreference is repeated [`ReferentId`] use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticValue {
	Referent(ReferentId),
	Clause(ClauseId),
}

/// Discourse referent.
#[derive(Clone, Debug)]
pub struct Referent {
	pub concept: ConceptId,
	pub definiteness: Definiteness,
	pub number: Number,
	pub person: Person,
	pub modifiers: Vec<Modifier>,
	pub relative_clauses: Vec<ClauseId>,
}

impl Referent {
	pub fn new(concept: ConceptId) -> Self {
		Self {
			concept,
			definiteness: Definiteness::Indefinite,
			number: Number::Singular,
			person: Person::Unspecified,
			modifiers: Vec::new(),
			relative_clauses: Vec::new(),
		}
	}

	pub fn definite(mut self) -> Self {
		self.definiteness = Definiteness::Definite;
		self
	}

	pub fn proper(mut self) -> Self {
		self.definiteness = Definiteness::Proper;
		self
	}

	pub fn many(mut self) -> Self {
		self.number = Number::Many;
		self
	}

	pub fn plural(mut self) -> Self {
		self.number = Number::Plural;
		self
	}

	pub fn first_person(mut self) -> Self {
		self.person = Person::First;
		self
	}

	pub fn second_person(mut self) -> Self {
		self.person = Person::Second;
		self
	}

	pub fn third_person(mut self) -> Self {
		self.person = Person::Third;
		self
	}

	pub fn with_relative(mut self, clause: ClauseId) -> Self {
		self.relative_clauses.push(clause);
		self
	}
}

/// Concept-level modifier. Surface adjectives are a later grammar concern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Modifier {
	pub concept: ConceptId,
}

/// Semantic role. Subject/object are grammatical realizations, not roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticRole {
	Agent,
	Patient,
	Theme,
	Experiencer,
	Recipient,
	Beneficiary,
	Instrument,
	Location,
	Source,
	Goal,
	Possessor,
	Cause,
	Content,
	Classification,
	Manner,
	Rate,
	Time,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Tense {
	Past,
	Present,
	Future,
	#[default]
	Unspecified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Aspect {
	Simple,
	Progressive,
	Perfect,
	#[default]
	Unspecified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Mood {
	#[default]
	Indicative,
	Imperative,
	Interrogative,
	Unspecified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Polarity {
	#[default]
	Affirmative,
	Negative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Definiteness {
	Definite,
	#[default]
	Indefinite,
	Generic,
	Proper,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Person {
	First,
	Second,
	Third,
	#[default]
	Unspecified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Number {
	#[default]
	Singular,
	Plural,
	Many,
	Mass,
	Unspecified,
}
