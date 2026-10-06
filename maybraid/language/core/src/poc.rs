//! POC utterance builders over WordNet senses.

use crate::concept::{ConceptId, EnglishSenseLookup};
use crate::error::LanguageError;
use crate::utterance::{Clause, Referent, SemanticRole, SemanticValue, Utterance};
use crate::wordnet::WordNetConceptUniverse;

/// Well-known senses used by the lexicalization POC.
pub struct PocLexicon {
	pub john: ConceptId,
	pub mary: ConceptId,
	pub book: ConceptId,
	pub give: ConceptId,
	pub witch: ConceptId,
	pub have: ConceptId,
	pub helper: ConceptId,
	pub classified_as: ConceptId,
	pub stream: ConceptId,
	pub carry: ConceptId,
	pub water: ConceptId,
	pub mountain: ConceptId,
	pub river: ConceptId,
}

impl PocLexicon {
	pub fn from_universe(universe: &WordNetConceptUniverse) -> Result<Self, LanguageError> {
		Ok(Self {
			john: universe
				.proper_name("John")
				.ok_or_else(|| LanguageError::MissingConcept("proper name John".to_owned()))?,
			mary: universe
				.proper_name("Mary")
				.ok_or_else(|| LanguageError::MissingConcept("proper name Mary".to_owned()))?,
			book: universe.noun_sense("book", 0)?,
			// give#v#3: transfer possession
			give: universe.verb_sense("give", 2)?,
			witch: universe.noun_sense("witch", 0)?,
			have: universe.verb_sense("have", 0)?,
			helper: universe.noun_sense("helper", 0)?,
			classified_as: universe.classified_as(),
			stream: universe.noun_sense("stream", 0)?,
			carry: universe.verb_sense("carry", 0)?,
			water: universe.noun_sense("water", 0)?,
			mountain: universe.noun_sense("mountain", 0)?,
			river: universe.noun_sense("river", 0)?,
		})
	}
}

impl Utterance {
	/// John gave the book to Mary.
	pub fn john_gave_the_book_to_mary(lex: &PocLexicon) -> Self {
		let mut utterance = Self::new();
		let john = utterance.add_referent(Referent::new(lex.john).proper());
		let book = utterance.add_referent(Referent::new(lex.book).definite());
		let mary = utterance.add_referent(Referent::new(lex.mary).proper());
		let give = utterance.add_clause(
			Clause::new(lex.give)
				.past()
				.with_argument(SemanticRole::Agent, SemanticValue::Referent(john))
				.with_argument(SemanticRole::Theme, SemanticValue::Referent(book))
				.with_argument(SemanticRole::Recipient, SemanticValue::Referent(mary)),
		);
		utterance.push_root(give);
		utterance
	}

	/// John gave the book to Mary, who was a witch that had many helpers.
	pub fn john_gave_the_book_to_mary_the_witch(lex: &PocLexicon) -> Self {
		let mut utterance = Self::new();
		let john = utterance.add_referent(Referent::new(lex.john).proper());
		let book = utterance.add_referent(Referent::new(lex.book).definite());
		let witch = utterance.add_referent(Referent::new(lex.witch));
		let helpers = utterance.add_referent(Referent::new(lex.helper).many());
		let mary = utterance.add_referent(Referent::new(lex.mary).proper());

		let classified = utterance.add_clause(
			Clause::new(lex.classified_as)
				.past()
				.with_argument(SemanticRole::Theme, SemanticValue::Referent(mary))
				.with_argument(SemanticRole::Classification, SemanticValue::Referent(witch)),
		);
		let have = utterance.add_clause(
			Clause::new(lex.have)
				.past()
				.with_argument(SemanticRole::Possessor, SemanticValue::Referent(witch))
				.with_argument(SemanticRole::Theme, SemanticValue::Referent(helpers)),
		);
		if let Some(witch_ref) = utterance.referents.get_mut(witch) {
			witch_ref.relative_clauses.push(have);
		}
		if let Some(mary_ref) = utterance.referents.get_mut(mary) {
			mary_ref.relative_clauses.push(classified);
		}

		let give = utterance.add_clause(
			Clause::new(lex.give)
				.past()
				.with_argument(SemanticRole::Agent, SemanticValue::Referent(john))
				.with_argument(SemanticRole::Theme, SemanticValue::Referent(book))
				.with_argument(SemanticRole::Recipient, SemanticValue::Referent(mary)),
		);
		utterance.push_root(give);
		utterance
	}

	/// The stream carried water from the mountain to the river.
	pub fn stream_carried_water(lex: &PocLexicon) -> Self {
		let mut utterance = Self::new();
		let stream = utterance.add_referent(Referent::new(lex.stream).definite());
		let water = utterance.add_referent(Referent::new(lex.water));
		let mountain = utterance.add_referent(Referent::new(lex.mountain).definite());
		let river = utterance.add_referent(Referent::new(lex.river).definite());
		let carry = utterance.add_clause(
			Clause::new(lex.carry)
				.past()
				.with_argument(SemanticRole::Agent, SemanticValue::Referent(stream))
				.with_argument(SemanticRole::Theme, SemanticValue::Referent(water))
				.with_argument(SemanticRole::Source, SemanticValue::Referent(mountain))
				.with_argument(SemanticRole::Goal, SemanticValue::Referent(river)),
		);
		utterance.push_root(carry);
		utterance
	}
}

/// Default POC universe: full WordNet 3.1 plus John/Mary proper names.
pub fn poc_universe() -> Result<WordNetConceptUniverse, LanguageError> {
	Ok(WordNetConceptUniverse::bundled()?.with_proper_names(["John", "Mary"]))
}
