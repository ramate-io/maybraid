//! In-memory POC graph. Lexicalization must not depend on this storage.

use std::collections::{HashMap, HashSet};

use slotmap::SlotMap;

use crate::concept::ConceptId;
use crate::graph::{
	ConceptEdge, GraphDelta, LexicalContextGraph, LexicalContextGraphMut, LexicalEdge,
	LexicalUpdate,
};
use crate::term::{Term, TermId};

/// Hash-map backed lexical context.
#[derive(Clone, Debug, Default)]
pub struct InMemoryLexicalGraph {
	concepts: HashSet<ConceptId>,
	imported: HashSet<ConceptId>,
	edges: HashMap<ConceptId, Vec<ConceptEdge>>,
	lexicalizations: HashMap<ConceptId, Vec<LexicalEdge>>,
	terms: SlotMap<TermId, Term>,
	ipa_index: HashMap<String, TermId>,
}

impl InMemoryLexicalGraph {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn term_count(&self) -> usize {
		self.terms.len()
	}

	fn intern(&mut self, term: Term) -> TermId {
		if let Some(id) = self.ipa_index.get(term.ipa.as_str()) {
			return *id;
		}
		let ipa = term.ipa.0.clone();
		let id = self.terms.insert(term);
		self.ipa_index.insert(ipa, id);
		id
	}
}

impl LexicalContextGraph for InMemoryLexicalGraph {
	fn contains_concept(&self, concept: ConceptId) -> bool {
		self.concepts.contains(&concept)
	}

	fn neighborhood_imported(&self, concept: ConceptId) -> bool {
		self.imported.contains(&concept)
	}

	fn neighbors(&self, concept: ConceptId) -> impl Iterator<Item = ConceptEdge> + '_ {
		self.edges.get(&concept).into_iter().flatten().copied()
	}

	fn lexicalizations(&self, concept: ConceptId) -> impl Iterator<Item = LexicalEdge> + '_ {
		self.lexicalizations.get(&concept).into_iter().flatten().copied()
	}

	fn term(&self, id: TermId) -> Option<&Term> {
		self.terms.get(id)
	}
}

impl LexicalContextGraphMut for InMemoryLexicalGraph {
	fn apply(&mut self, delta: GraphDelta) {
		for concept in delta.concepts {
			self.concepts.insert(concept);
		}
		for edge in delta.edges {
			self.concepts.insert(edge.source);
			self.concepts.insert(edge.target);
			let bucket = self.edges.entry(edge.source).or_default();
			if !bucket.iter().any(|existing| {
				existing.target == edge.target && existing.relation == edge.relation
			}) {
				bucket.push(edge);
			}
		}
		for concept in delta.expanded {
			self.concepts.insert(concept);
			self.imported.insert(concept);
		}
	}

	fn apply_update(&mut self, update: LexicalUpdate) {
		match update {
			LexicalUpdate::Bind { concept, term, usage } => {
				self.concepts.insert(concept);
				let term_id = self.intern(term);
				let bucket = self.lexicalizations.entry(concept).or_default();
				if let Some(existing) = bucket.iter_mut().find(|edge| edge.term == term_id) {
					existing.usage = usage;
				} else {
					bucket.push(LexicalEdge { concept, term: term_id, usage });
				}
			}
		}
	}
}
