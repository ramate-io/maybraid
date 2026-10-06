//! Expand context, gather candidates, resolve a base term, reinforce use.

use crate::concept::{ConceptId, ConceptUniverse, RelationKind};
use crate::graph::{
	ConceptEdge, GraphDelta, LexicalContextGraph, LexicalContextGraphMut, LexicalUpdate,
};
use crate::profile::Profile;
use crate::term::{Term, Usage};

mod compositional;
mod phonology;
mod root_heavy;

pub use compositional::CompositionalLexicalizer;
pub use phonology::{mix, GENERATOR_VERSION};
pub use root_heavy::RootHeavyLexicalizer;

/// Lexical material gathered from the language graph. Not a complete alternative.
#[derive(Clone, Debug, PartialEq)]
pub struct LexicalCandidate {
	pub term: Term,
	pub source_concept: ConceptId,
	pub relation: RelationKind,
	pub usage: Usage,
}

/// Chosen base term plus debug provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct LexicalRealization {
	pub term: Term,
	pub provenance: Vec<CandidateContribution>,
}

/// How much a candidate concept contributed to the realized form.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateContribution {
	pub source_concept: ConceptId,
	pub contribution: f32,
}

/// Procedural lexicalizer. Storage stays behind the graph traits.
pub trait Lexicalizer {
	/// Import enough external semantic context to reason about the concept.
	fn expand_context(
		&self,
		concept: ConceptId,
		universe: &impl ConceptUniverse,
		graph: &impl LexicalContextGraph,
	) -> GraphDelta;

	/// Gather lexical material available from the language graph.
	fn candidates(
		&self,
		concept: ConceptId,
		graph: &impl LexicalContextGraph,
	) -> Vec<LexicalCandidate>;

	/// Produce the base lexical term.
	fn resolve(
		&self,
		concept: ConceptId,
		candidates: &[LexicalCandidate],
		graph: &impl LexicalContextGraph,
		profile: &Profile,
	) -> LexicalRealization;

	/// Apply slow usage reinforcement after a term is actually used.
	fn post_resolution(
		&self,
		concept: ConceptId,
		realization: &LexicalRealization,
		graph: &mut impl LexicalContextGraphMut,
		profile: &Profile,
	);

	fn lexicalize(
		&self,
		concept: ConceptId,
		universe: &impl ConceptUniverse,
		graph: &mut impl LexicalContextGraphMut,
		profile: &Profile,
	) -> LexicalRealization {
		let delta = self.expand_context(concept, universe, graph);
		graph.apply(delta);
		let candidates = self.candidates(concept, graph);
		let realization = self.resolve(concept, &candidates, graph, profile);
		self.post_resolution(concept, &realization, graph, profile);
		realization
	}
}

pub(crate) fn established_term(
	concept: ConceptId,
	graph: &impl LexicalContextGraph,
) -> Option<Term> {
	graph
		.lexicalizations(concept)
		.filter(|edge| edge.usage.is_established())
		.max_by(|a, b| {
			a.usage
				.preference()
				.partial_cmp(&b.usage.preference())
				.unwrap_or(std::cmp::Ordering::Equal)
		})
		.and_then(|edge| graph.term(edge.term).cloned())
}

pub(crate) fn any_term(
	concept: ConceptId,
	graph: &impl LexicalContextGraph,
) -> Option<(Term, Usage)> {
	graph
		.lexicalizations(concept)
		.next()
		.and_then(|edge| graph.term(edge.term).cloned().map(|term| (term, edge.usage)))
}

pub(crate) fn reinforce(
	concept: ConceptId,
	realization: &LexicalRealization,
	graph: &mut impl LexicalContextGraphMut,
) {
	let usage = graph
		.lexicalizations(concept)
		.find(|edge| graph.term(edge.term).is_some_and(|term| term == &realization.term))
		.map(|edge| edge.usage.reinforced())
		.unwrap_or_else(|| Usage::novel().reinforced());
	graph.apply_update(LexicalUpdate::Bind { concept, term: realization.term.clone(), usage });
}

pub(crate) fn filtered_delta<U, F>(
	concept: ConceptId,
	universe: &U,
	graph: &impl LexicalContextGraph,
	kinds: &[RelationKind],
	max_neighbors: usize,
	keep: F,
) -> GraphDelta
where
	U: ConceptUniverse,
	F: Fn(&ConceptEdge, &U) -> bool,
{
	if graph.neighborhood_imported(concept) {
		return GraphDelta::empty();
	}
	let request = crate::concept::NeighborhoodRequest {
		max_neighbors,
		max_depth: 2,
		kinds: Some(kinds.to_vec()),
	};
	let mut concepts = vec![concept];
	let mut edges = Vec::new();
	for relation in universe.neighborhood(concept, request) {
		let edge = ConceptEdge {
			source: relation.source,
			target: relation.target,
			relation: relation.kind,
			weight: relation.weight,
		};
		if keep(&edge, universe) {
			if !concepts.contains(&edge.target) {
				concepts.push(edge.target);
			}
			edges.push(edge);
		}
	}
	GraphDelta { concepts, edges, expanded: vec![concept] }
}
