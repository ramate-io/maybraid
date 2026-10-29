//! Storage-agnostic lexical context graph.

use crate::concept::{ConceptId, RelationKind};
use crate::term::{Term, TermId, Usage};

mod memory;

pub use memory::InMemoryLexicalGraph;

/// Directed concept-to-concept edge inside one language graph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConceptEdge {
	pub source: ConceptId,
	pub target: ConceptId,
	pub relation: RelationKind,
	pub weight: f32,
}

/// Concept-to-term binding inside one language graph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LexicalEdge {
	pub concept: ConceptId,
	pub term: TermId,
	pub usage: Usage,
}

/// Monotonic neighborhood import.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphDelta {
	pub concepts: Vec<ConceptId>,
	pub edges: Vec<ConceptEdge>,
	/// Concepts whose external neighborhood was imported by this delta.
	pub expanded: Vec<ConceptId>,
}

impl GraphDelta {
	pub fn empty() -> Self {
		Self::default()
	}

	pub fn is_empty(&self) -> bool {
		self.concepts.is_empty() && self.edges.is_empty() && self.expanded.is_empty()
	}
}

/// Mutation produced after a term is actually used.
#[derive(Clone, Debug, PartialEq)]
pub enum LexicalUpdate {
	Bind { concept: ConceptId, term: Term, usage: Usage },
}

/// Read API for a language's sparse lexical graph.
pub trait LexicalContextGraph {
	fn contains_concept(&self, concept: ConceptId) -> bool;

	/// True once this concept has been the subject of a neighborhood import.
	fn neighborhood_imported(&self, concept: ConceptId) -> bool;

	fn neighbors(&self, concept: ConceptId) -> impl Iterator<Item = ConceptEdge> + '_;

	fn lexicalizations(&self, concept: ConceptId) -> impl Iterator<Item = LexicalEdge> + '_;

	fn term(&self, id: TermId) -> Option<&Term>;
}

/// Write API. Storage remains implementation-defined.
pub trait LexicalContextGraphMut: LexicalContextGraph {
	fn apply(&mut self, delta: GraphDelta);

	fn apply_update(&mut self, update: LexicalUpdate);
}
