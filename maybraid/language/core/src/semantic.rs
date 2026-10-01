//! English UD → [`Utterance`] marshalling.

use std::collections::HashMap;

use crate::concept::{ConceptId, ConceptUniverse, Pos};
use crate::error::LanguageError;
use crate::parse::{
	DependencyDocument, DependencySentence, DependencyToken, TokenId, UniversalPos,
};
use crate::utterance::{
	Aspect, Clause, Definiteness, Modifier, Mood, Number, Polarity, Referent, ReferentId,
	SemanticRole, SemanticValue, Tense, Utterance,
};
use crate::wordnet::normalize_lemma;

/// Syntax → semantic graph.
pub trait SemanticMarshaller {
	fn marshal(
		&self,
		document: &DependencyDocument,
		universe: &impl ConceptUniverse,
	) -> Result<Utterance, LanguageError>;
}

/// Curated predicate-frame lookup for the POC dialogue set.
#[derive(Clone, Debug, Default)]
pub struct PredicateFrameLexicon {
	frames: Vec<PredicateFrame>,
}

#[derive(Clone, Debug)]
pub struct PredicateFrame {
	pub lemma: &'static str,
	pub mappings: Vec<RoleMapping>,
}

#[derive(Clone, Copy, Debug)]
pub struct RoleMapping {
	pub relation: &'static str,
	pub case: Option<&'static str>,
	pub role: SemanticRole,
}

impl PredicateFrameLexicon {
	pub fn poc() -> Self {
		Self {
			frames: vec![
				frame(
					"give",
					&[
						map("nsubj", None, SemanticRole::Agent),
						map("obj", None, SemanticRole::Theme),
						map("iobj", None, SemanticRole::Recipient),
						map("obl", Some("to"), SemanticRole::Recipient),
					],
				),
				frame(
					"see",
					&[
						map("nsubj", None, SemanticRole::Experiencer),
						map("obj", None, SemanticRole::Theme),
					],
				),
				frame("fall", &[map("nsubj", None, SemanticRole::Theme)]),
				frame(
					"go",
					&[
						map("nsubj", None, SemanticRole::Agent),
						map("obl", Some("to"), SemanticRole::Goal),
						map("obl", Some("for"), SemanticRole::Goal),
						map("obl", None, SemanticRole::Goal),
					],
				),
				frame(
					"walk",
					&[
						map("nsubj", None, SemanticRole::Agent),
						map("obl", Some("to"), SemanticRole::Goal),
						map("obl", None, SemanticRole::Goal),
					],
				),
				frame(
					"tell",
					&[
						map("nsubj", None, SemanticRole::Agent),
						map("obj", None, SemanticRole::Recipient),
						map("iobj", None, SemanticRole::Recipient),
						map("obl", Some("to"), SemanticRole::Recipient),
						map("ccomp", None, SemanticRole::Content),
					],
				),
				frame(
					"steal",
					&[
						map("nsubj", None, SemanticRole::Agent),
						map("obj", None, SemanticRole::Theme),
					],
				),
				frame("leave", &[map("nsubj", None, SemanticRole::Agent)]),
				frame(
					"be",
					&[
						map("nsubj", None, SemanticRole::Theme),
						map("xcomp", None, SemanticRole::Classification),
						map("attr", None, SemanticRole::Classification),
					],
				),
			],
		}
	}

	pub fn mappings_for(&self, lemma: &str) -> &[RoleMapping] {
		self.frames
			.iter()
			.find(|frame| frame.lemma == lemma)
			.map(|frame| frame.mappings.as_slice())
			.unwrap_or(&[])
	}
}

fn frame(lemma: &'static str, mappings: &[RoleMapping]) -> PredicateFrame {
	PredicateFrame { lemma, mappings: mappings.to_vec() }
}

const fn map(relation: &'static str, case: Option<&'static str>, role: SemanticRole) -> RoleMapping {
	RoleMapping { relation, case, role }
}

/// Converts a [`DependencyDocument`] into an [`Utterance`].
#[derive(Clone, Debug)]
pub struct EnglishSemanticMarshaller {
	pub predicate_frames: PredicateFrameLexicon,
}

impl Default for EnglishSemanticMarshaller {
	fn default() -> Self {
		Self { predicate_frames: PredicateFrameLexicon::poc() }
	}
}

impl SemanticMarshaller for EnglishSemanticMarshaller {
	fn marshal(
		&self,
		document: &DependencyDocument,
		universe: &impl ConceptUniverse,
	) -> Result<Utterance, LanguageError> {
		let mut utterance = Utterance::new();
		for sentence in &document.sentences {
			self.marshal_sentence(sentence, universe, &mut utterance)?;
		}
		if utterance.roots.is_empty() && !utterance.clauses.is_empty() {
			if let Some((id, _)) = utterance.clauses.iter().next() {
				utterance.push_root(id);
			}
		}
		Ok(utterance)
	}
}

impl EnglishSemanticMarshaller {
	fn marshal_sentence(
		&self,
		sentence: &DependencySentence,
		universe: &impl ConceptUniverse,
		utterance: &mut Utterance,
	) -> Result<(), LanguageError> {
		let predicates = predicate_tokens(sentence);
		let mut clauses = HashMap::<TokenId, crate::utterance::ClauseId>::new();
		for predicate in &predicates {
			let lemma = normalize_lemma(&predicate.lemma);
			let concept = resolve_predicate(universe, &lemma)?;
			let mut clause = Clause::new(concept);
			apply_clause_morph(&mut clause, predicate, sentence);
			clauses.insert(predicate.id, utterance.add_clause(clause));
		}

		let mut referents = HashMap::<TokenId, ReferentId>::new();
		for predicate in &predicates {
			let Some(&clause_id) = clauses.get(&predicate.id) else {
				continue;
			};
			let lemma = normalize_lemma(&predicate.lemma);
			for dependent in sentence.children(predicate.id) {
				if should_skip_dependent(dependent) {
					continue;
				}
				if let Some(nested) = clauses.get(&dependent.id).copied() {
					if let Some(role) = self.role_for(&lemma, predicate, dependent, sentence) {
						if let Some(clause) = utterance.clauses.get_mut(clause_id) {
							clause.arguments.push(crate::utterance::Argument {
								role,
								value: SemanticValue::Clause(nested),
							});
						}
					}
					continue;
				}
				let Some(nominal) = argument_nominal(dependent, sentence) else {
					continue;
				};
				let referent =
					self.ensure_referent(nominal, sentence, universe, utterance, &mut referents)?;
				let Some(role) = self.role_for(&lemma, predicate, dependent, sentence) else {
					continue;
				};
				if let Some(clause) = utterance.clauses.get_mut(clause_id) {
					if !clause.arguments.iter().any(|argument| argument.role == role) {
						clause
							.arguments
							.push(crate::utterance::Argument { role, value: SemanticValue::Referent(referent) });
					}
				}
			}
		}

		for token in &sentence.tokens {
			if token.relation.base() != "acl" {
				continue;
			}
			let Some(&clause_id) = clauses.get(&token.id) else {
				continue;
			};
			let Some(head) = token.head.and_then(|id| sentence.token(id)) else {
				continue;
			};
			let referent =
				self.ensure_referent(head, sentence, universe, utterance, &mut referents)?;
			if let Some(stored) = utterance.referents.get_mut(referent) {
				if !stored.relative_clauses.contains(&clause_id) {
					stored.relative_clauses.push(clause_id);
				}
			}
		}

		for token in &sentence.tokens {
			if token.relation.is("root") {
				if let Some(&clause_id) = clauses.get(&token.id) {
					utterance.push_root(clause_id);
				} else if let Some(verb) =
					sentence.children(token.id).find(|child| matches!(child.pos, UniversalPos::Verb))
				{
					if let Some(&clause_id) = clauses.get(&verb.id) {
						utterance.push_root(clause_id);
					}
				}
			}
		}
		Ok(())
	}

	fn ensure_referent(
		&self,
		token: &DependencyToken,
		sentence: &DependencySentence,
		universe: &impl ConceptUniverse,
		utterance: &mut Utterance,
		referents: &mut HashMap<TokenId, ReferentId>,
	) -> Result<ReferentId, LanguageError> {
		if let Some(id) = referents.get(&token.id) {
			return Ok(*id);
		}
		if is_relative_pronoun(token) {
			if let Some(head) = token.head.and_then(|id| sentence.token(id)) {
				if head.relation.base() == "acl" {
					if let Some(noun) = head.head.and_then(|id| sentence.token(id)) {
						return self.ensure_referent(noun, sentence, universe, utterance, referents);
					}
				}
			}
		}
		let lemma = referent_lemma(token);
		let prefer = match token.pos {
			UniversalPos::Adjective => Some(Pos::Adjective),
			UniversalPos::Verb => Some(Pos::Verb),
			_ => Some(Pos::Noun),
		};
		let concept = resolve_concept(universe, &lemma, prefer)?;
		let mut referent = Referent::new(concept);
		referent.definiteness = definiteness_of(token, sentence);
		referent.number = number_of(token, sentence);
		for child in sentence.children(token.id) {
			if child.relation.is("amod") {
				if let Ok(modifier) = resolve_concept(universe, &child.lemma, Some(Pos::Adjective)) {
					referent.modifiers.push(Modifier { concept: modifier });
				}
			}
		}
		let id = utterance.add_referent(referent);
		referents.insert(token.id, id);
		Ok(id)
	}

	fn role_for(
		&self,
		lemma: &str,
		predicate: &DependencyToken,
		dependent: &DependencyToken,
		sentence: &DependencySentence,
	) -> Option<SemanticRole> {
		if is_passive(predicate, sentence) {
			if dependent.relation.is("nsubj:pass") || dependent.relation.is("nsubj") {
				return Some(SemanticRole::Theme);
			}
			if dependent.relation.is("obl") && case_of(dependent, sentence) == Some("by") {
				return Some(SemanticRole::Agent);
			}
		}
		let case = case_of(dependent, sentence);
		let rel = dependent.relation.as_str();
		let base = dependent.relation.base();
		for mapping in self.predicate_frames.mappings_for(lemma) {
			if mapping.relation == rel || mapping.relation == base {
				if mapping.case.is_none() || mapping.case == case {
					return Some(mapping.role);
				}
			}
		}
		default_role(dependent, case)
	}
}

fn predicate_tokens(sentence: &DependencySentence) -> Vec<&DependencyToken> {
	sentence
		.tokens
		.iter()
		.filter(|token| is_predicate(token, sentence))
		.collect()
}

fn is_predicate(token: &DependencyToken, sentence: &DependencySentence) -> bool {
	if matches!(token.pos, UniversalPos::Verb) {
		return true;
	}
	if token.relation.is("root") {
		if matches!(token.pos, UniversalPos::Auxiliary) {
			return !sentence
				.children(token.id)
				.any(|child| matches!(child.pos, UniversalPos::Verb));
		}
		return matches!(token.pos, UniversalPos::Adjective);
	}
	false
}

fn should_skip_dependent(token: &DependencyToken) -> bool {
	matches!(
		token.relation.base(),
		"aux" | "cop" | "mark" | "punct" | "case" | "det" | "cc" | "expl" | "advmod"
	) && !token.relation.is("neg")
}

fn argument_nominal<'a>(
	dependent: &'a DependencyToken,
	sentence: &'a DependencySentence,
) -> Option<&'a DependencyToken> {
	if is_nominal(dependent) {
		return Some(dependent);
	}
	sentence.children(dependent.id).find(|child| is_nominal(child))
}

fn is_nominal(token: &DependencyToken) -> bool {
	matches!(
		token.pos,
		UniversalPos::Noun | UniversalPos::ProperNoun | UniversalPos::Pronoun | UniversalPos::Numeral
	)
}

fn is_relative_pronoun(token: &DependencyToken) -> bool {
	matches!(normalize_lemma(&token.lemma).as_str(), "who" | "that" | "which" | "whom")
}

fn is_aux(token: &DependencyToken) -> bool {
	token.relation.base() == "aux"
}

fn form_is(token: &DependencyToken, form: &str) -> bool {
	normalize_lemma(&token.lemma) == form || normalize_lemma(&token.text) == form
}

fn form_in(token: &DependencyToken, forms: &[&str]) -> bool {
	forms.iter().any(|form| form_is(token, form))
}

fn is_past_aux(token: &DependencyToken) -> bool {
	token.features.has("Tense", "Past") || form_in(token, &["did", "was", "were", "had"])
}

fn is_present_aux(token: &DependencyToken) -> bool {
	token.features.has("Tense", "Pres") || form_in(token, &["am", "is", "are", "do", "does"])
}

fn is_passive(predicate: &DependencyToken, sentence: &DependencySentence) -> bool {
	predicate.features.has("Voice", "Pass")
		|| sentence.children(predicate.id).any(|child| child.relation.is("aux:pass"))
}

fn case_of<'a>(token: &'a DependencyToken, sentence: &'a DependencySentence) -> Option<&'a str> {
	sentence
		.children(token.id)
		.find(|child| child.relation.is("case"))
		.map(|child| child.lemma.as_str())
}

fn default_role(dependent: &DependencyToken, case: Option<&str>) -> Option<SemanticRole> {
	match dependent.relation.as_str() {
		"nsubj" | "nsubj:pass" if dependent.relation.is("nsubj:pass") => Some(SemanticRole::Theme),
		"nsubj" => Some(SemanticRole::Agent),
		"obj" => Some(SemanticRole::Theme),
		"iobj" => Some(SemanticRole::Recipient),
		"ccomp" | "xcomp" => Some(SemanticRole::Content),
		"obl" if case == Some("to") => Some(SemanticRole::Goal),
		"obl" if case == Some("for") => Some(SemanticRole::Goal),
		"obl" if case == Some("by") => Some(SemanticRole::Agent),
		"obl" | "nmod" => Some(SemanticRole::Location),
		_ if dependent.relation.base() == "obl" => Some(SemanticRole::Goal),
		_ => None,
	}
}

fn apply_clause_morph(
	clause: &mut Clause,
	predicate: &DependencyToken,
	sentence: &DependencySentence,
) {
	if predicate.features.has("Tense", "Past")
		|| sentence.children(predicate.id).any(|child| is_aux(child) && is_past_aux(child))
	{
		clause.tense = Tense::Past;
	} else if sentence.children(predicate.id).any(|child| is_aux(child) && form_is(child, "will"))
	{
		clause.tense = Tense::Future;
	} else if predicate.features.has("Tense", "Pres")
		|| sentence.children(predicate.id).any(|child| is_aux(child) && is_present_aux(child))
	{
		clause.tense = Tense::Present;
	}

	if predicate.features.has("VerbForm", "Ger")
		|| (predicate.features.has("VerbForm", "Part") && !is_passive(predicate, sentence))
		|| sentence.children(predicate.id).any(|child| {
			is_aux(child) && form_in(child, &["am", "is", "are", "was", "were", "be"])
		}) {
		clause.aspect = Aspect::Progressive;
	}
	if sentence.children(predicate.id).any(|child| form_in(child, &["have", "has", "had"])) {
		clause.aspect = Aspect::Perfect;
	}

	if sentence.children(predicate.id).any(|child| {
		child.relation.is("advmod") && normalize_lemma(&child.lemma) == "not"
			|| child.features.has("Polarity", "Neg")
			|| normalize_lemma(&child.text) == "n't"
	}) {
		clause.polarity = Polarity::Negative;
	}
	if predicate.features.has("Mood", "Imp") {
		clause.mood = Mood::Imperative;
	}
}

fn referent_lemma(token: &DependencyToken) -> String {
	match normalize_lemma(&token.lemma).as_str() {
		"i" | "me" | "we" | "us" | "myself" => "speaker".to_owned(),
		"you" => "listener".to_owned(),
		other => other.to_owned(),
	}
}

fn definiteness_of(token: &DependencyToken, sentence: &DependencySentence) -> Definiteness {
	if matches!(token.pos, UniversalPos::ProperNoun) {
		return Definiteness::Proper;
	}
	for child in sentence.children(token.id) {
		if child.relation.is("det") {
			return match normalize_lemma(&child.lemma).as_str() {
				"the" | "this" | "that" | "these" | "those" => Definiteness::Definite,
				_ => Definiteness::Indefinite,
			};
		}
	}
	if matches!(normalize_lemma(&token.lemma).as_str(), "speaker" | "listener" | "i" | "you") {
		Definiteness::Definite
	} else {
		Definiteness::Indefinite
	}
}

fn number_of(token: &DependencyToken, sentence: &DependencySentence) -> Number {
	if token.features.has("Number", "Plur") {
		return Number::Plural;
	}
	if sentence.children(token.id).any(|child| child.relation.is("nummod")) {
		return Number::Many;
	}
	Number::Singular
}

fn resolve_predicate(
	universe: &impl ConceptUniverse,
	lemma: &str,
) -> Result<ConceptId, LanguageError> {
	if matches!(lemma, "be" | "am" | "is" | "are" | "was" | "were") {
		return resolve_concept(universe, "be", Some(Pos::Verb));
	}
	resolve_concept(universe, lemma, Some(Pos::Verb))
}

fn resolve_concept(
	universe: &impl ConceptUniverse,
	name: &str,
	prefer: Option<Pos>,
) -> Result<ConceptId, LanguageError> {
	let lemma = normalize_lemma(name);
	let aliases = match lemma.as_str() {
		"i" | "me" | "we" | "us" | "myself" | "speaker" | "self" => {
			vec!["speaker".to_owned(), "person".to_owned()]
		}
		"you" | "listener" => vec!["listener".to_owned(), "person".to_owned()],
		_ => vec![lemma],
	};
	for alias in aliases {
		let ids = universe.resolve_english(&alias);
		if let Some(id) = pick_concept(&ids, prefer) {
			return Ok(id);
		}
	}
	Err(LanguageError::MissingConcept(name.to_owned()))
}

fn pick_concept(ids: &[ConceptId], prefer: Option<Pos>) -> Option<ConceptId> {
	if let Some(pos) = prefer {
		if let Some(id) = ids.iter().copied().find(|id| id.pos() == Some(pos)) {
			return Some(id);
		}
	}
	ids.first().copied()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::parse::{DependencyRelation, MorphFeatures, TokenId};
	use crate::poc::poc_universe;

	fn tok(
		id: u32,
		text: &str,
		lemma: &str,
		pos: UniversalPos,
		head: u32,
		rel: &str,
		feats: &str,
	) -> DependencyToken {
		DependencyToken {
			id: TokenId(id),
			text: text.to_owned(),
			lemma: lemma.to_owned(),
			pos,
			features: MorphFeatures::parse(feats),
			head: if head == 0 { None } else { Some(TokenId(head)) },
			relation: DependencyRelation::parse(rel),
		}
	}

	fn sentence(tokens: Vec<DependencyToken>) -> DependencyDocument {
		DependencyDocument { sentences: vec![DependencySentence { tokens }] }
	}

	fn universe() -> crate::wordnet::WordNetConceptUniverse {
		poc_universe()
			.expect("wordnet")
			.with_proper_names(["John", "Mary", "Alice", "speaker", "listener"])
	}

	fn roles(utterance: &Utterance) -> Vec<SemanticRole> {
		utterance
			.clauses
			.values()
			.next()
			.map(|clause| clause.arguments.iter().map(|argument| argument.role).collect())
			.unwrap_or_default()
	}

	fn has_role(utterance: &Utterance, role: SemanticRole) -> bool {
		utterance
			.clauses
			.values()
			.any(|clause| clause.arguments.iter().any(|argument| argument.role == role))
	}

	#[test]
	fn john_gave_mary_the_book() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "John", "John", UniversalPos::ProperNoun, 2, "nsubj", ""),
			tok(2, "gave", "give", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(3, "Mary", "Mary", UniversalPos::ProperNoun, 2, "iobj", ""),
			tok(4, "the", "the", UniversalPos::Determiner, 5, "det", ""),
			tok(5, "book", "book", UniversalPos::Noun, 2, "obj", ""),
			tok(6, ".", ".", UniversalPos::Punctuation, 2, "punct", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe())?;
		assert!(has_role(&utterance, SemanticRole::Agent));
		assert!(has_role(&utterance, SemanticRole::Theme));
		assert!(has_role(&utterance, SemanticRole::Recipient));
		assert_eq!(utterance.roots.len(), 1);
		Ok(())
	}

	#[test]
	fn john_gave_the_book_to_mary() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "John", "John", UniversalPos::ProperNoun, 2, "nsubj", ""),
			tok(2, "gave", "give", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(3, "the", "the", UniversalPos::Determiner, 4, "det", ""),
			tok(4, "book", "book", UniversalPos::Noun, 2, "obj", ""),
			tok(5, "to", "to", UniversalPos::Adposition, 6, "case", ""),
			tok(6, "Mary", "Mary", UniversalPos::ProperNoun, 2, "obl", ""),
			tok(7, ".", ".", UniversalPos::Punctuation, 2, "punct", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe())?;
		assert!(has_role(&utterance, SemanticRole::Recipient));
		Ok(())
	}

	#[test]
	fn mary_saw_john() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "Mary", "Mary", UniversalPos::ProperNoun, 2, "nsubj", ""),
			tok(2, "saw", "see", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(3, "John", "John", UniversalPos::ProperNoun, 2, "obj", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe())?;
		assert!(roles(&utterance).contains(&SemanticRole::Experiencer));
		assert!(roles(&utterance).contains(&SemanticRole::Theme));
		Ok(())
	}

	#[test]
	fn book_was_given_to_mary_by_john() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "The", "the", UniversalPos::Determiner, 2, "det", ""),
			tok(2, "book", "book", UniversalPos::Noun, 4, "nsubj:pass", ""),
			tok(3, "was", "be", UniversalPos::Auxiliary, 4, "aux:pass", "Tense=Past"),
			tok(4, "given", "give", UniversalPos::Verb, 0, "root", "Voice=Pass"),
			tok(5, "to", "to", UniversalPos::Adposition, 6, "case", ""),
			tok(6, "Mary", "Mary", UniversalPos::ProperNoun, 4, "obl", ""),
			tok(7, "by", "by", UniversalPos::Adposition, 8, "case", ""),
			tok(8, "John", "John", UniversalPos::ProperNoun, 4, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe())?;
		assert!(has_role(&utterance, SemanticRole::Theme));
		assert!(has_role(&utterance, SemanticRole::Agent));
		assert!(has_role(&utterance, SemanticRole::Recipient));
		Ok(())
	}

	#[test]
	fn mary_told_john_that_alice_left() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "Mary", "Mary", UniversalPos::ProperNoun, 2, "nsubj", ""),
			tok(2, "told", "tell", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(3, "John", "John", UniversalPos::ProperNoun, 2, "obj", ""),
			tok(4, "that", "that", UniversalPos::SubordinatingConjunction, 6, "mark", ""),
			tok(5, "Alice", "Alice", UniversalPos::ProperNoun, 6, "nsubj", ""),
			tok(6, "left", "leave", UniversalPos::Verb, 2, "ccomp", "Tense=Past"),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe())?;
		assert_eq!(utterance.clauses.len(), 2);
		assert!(has_role(&utterance, SemanticRole::Content));
		Ok(())
	}

	#[test]
	fn mary_saw_the_man_who_stole_the_book() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "Mary", "Mary", UniversalPos::ProperNoun, 2, "nsubj", ""),
			tok(2, "saw", "see", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(3, "the", "the", UniversalPos::Determiner, 4, "det", ""),
			tok(4, "man", "man", UniversalPos::Noun, 2, "obj", ""),
			tok(5, "who", "who", UniversalPos::Pronoun, 6, "nsubj", ""),
			tok(6, "stole", "steal", UniversalPos::Verb, 4, "acl:relcl", "Tense=Past"),
			tok(7, "the", "the", UniversalPos::Determiner, 8, "det", ""),
			tok(8, "book", "book", UniversalPos::Noun, 6, "obj", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe())?;
		assert_eq!(utterance.clauses.len(), 2);
		assert!(utterance.referents.values().any(|referent| !referent.relative_clauses.is_empty()));
		Ok(())
	}

	#[test]
	fn john_is_walking_to_the_river() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "John", "John", UniversalPos::ProperNoun, 3, "nsubj", ""),
			tok(2, "is", "be", UniversalPos::Auxiliary, 3, "aux", "Tense=Pres"),
			tok(3, "walking", "walk", UniversalPos::Verb, 0, "root", "VerbForm=Ger"),
			tok(4, "to", "to", UniversalPos::Adposition, 6, "case", ""),
			tok(5, "the", "the", UniversalPos::Determiner, 6, "det", ""),
			tok(6, "river", "river", UniversalPos::Noun, 3, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe())?;
		let clause = utterance.clauses.values().next().expect("clause");
		assert_eq!(clause.aspect, Aspect::Progressive);
		assert_eq!(clause.tense, Tense::Present);
		assert!(has_role(&utterance, SemanticRole::Goal));
		Ok(())
	}

	#[test]
	fn john_did_not_see_mary() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "John", "John", UniversalPos::ProperNoun, 4, "nsubj", ""),
			tok(2, "did", "do", UniversalPos::Auxiliary, 4, "aux", "Tense=Past"),
			tok(3, "not", "not", UniversalPos::Particle, 4, "advmod", "Polarity=Neg"),
			tok(4, "see", "see", UniversalPos::Verb, 0, "root", ""),
			tok(5, "Mary", "Mary", UniversalPos::ProperNoun, 4, "obj", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe())?;
		let clause = utterance.clauses.values().next().expect("clause");
		assert_eq!(clause.polarity, Polarity::Negative);
		assert_eq!(clause.tense, Tense::Past);
		Ok(())
	}

	#[test]
	fn i_am_going_for_a_walk() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "I", "I", UniversalPos::Pronoun, 3, "nsubj", ""),
			tok(2, "am", "be", UniversalPos::Auxiliary, 3, "aux", "Tense=Pres"),
			tok(3, "going", "go", UniversalPos::Verb, 0, "root", "VerbForm=Ger"),
			tok(4, "for", "for", UniversalPos::Adposition, 6, "case", ""),
			tok(5, "a", "a", UniversalPos::Determiner, 6, "det", ""),
			tok(6, "walk", "walk", UniversalPos::Noun, 3, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe())?;
		assert!(has_role(&utterance, SemanticRole::Agent));
		assert!(has_role(&utterance, SemanticRole::Goal));
		let clause = utterance.clauses.values().next().expect("clause");
		assert_eq!(clause.aspect, Aspect::Progressive);
		Ok(())
	}

	#[test]
	fn live_udpipe_i_am_going_for_a_walk() -> Result<(), LanguageError> {
		let Ok(parser) = crate::udpipe::UdpipeEnglishParser::bundled() else {
			return Ok(());
		};
		let document = crate::parse::EnglishDependencyParser::parse(&parser, "I am going for a walk.")?;
		let utterance = EnglishSemanticMarshaller::default().marshal(&document, &universe())?;
		assert!(!utterance.roots.is_empty());
		assert!(has_role(&utterance, SemanticRole::Agent));
		Ok(())
	}

	#[test]
	fn old_woman_gave_three_books_to_the_child() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "The", "the", UniversalPos::Determiner, 3, "det", ""),
			tok(2, "old", "old", UniversalPos::Adjective, 3, "amod", ""),
			tok(3, "woman", "woman", UniversalPos::Noun, 4, "nsubj", ""),
			tok(4, "gave", "give", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(5, "three", "three", UniversalPos::Numeral, 6, "nummod", "NumType=Card"),
			tok(6, "books", "book", UniversalPos::Noun, 4, "obj", "Number=Plur"),
			tok(7, "to", "to", UniversalPos::Adposition, 9, "case", ""),
			tok(8, "the", "the", UniversalPos::Determiner, 9, "det", ""),
			tok(9, "child", "child", UniversalPos::Noun, 4, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe())?;
		assert!(utterance.referents.values().any(|referent| !referent.modifiers.is_empty()));
		assert!(utterance.referents.values().any(|referent| referent.number == Number::Plural));
		assert!(has_role(&utterance, SemanticRole::Recipient));
		Ok(())
	}

}
