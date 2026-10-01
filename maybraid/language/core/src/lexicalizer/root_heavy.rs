//! Language B: independent roots, weak reuse, occasional blends.

use crate::concept::{ConceptId, ConceptUniverse, RelationKind};
use crate::graph::{GraphDelta, LexicalContextGraph, LexicalContextGraphMut};
use crate::lexicalizer::phonology::{blend, coined_root, mix, Phonology};
use crate::lexicalizer::{
	any_term, established_term, filtered_delta, reinforce, CandidateContribution, LexicalCandidate,
	LexicalRealization, Lexicalizer,
};
use crate::profile::Profile;

/// Root-heavy lexicalizer. Related concepts need not share material.
#[derive(Clone, Debug)]
pub struct RootHeavyLexicalizer {
	pub seed: u64,
}

impl RootHeavyLexicalizer {
	pub const DEFAULT_SEED: u64 = 0xB22B_B22B_B22B_B22B;

	pub fn new(seed: u64) -> Self {
		Self { seed }
	}
}

impl Default for RootHeavyLexicalizer {
	fn default() -> Self {
		Self::new(Self::DEFAULT_SEED)
	}
}

impl Lexicalizer for RootHeavyLexicalizer {
	fn expand_context(
		&self,
		concept: ConceptId,
		universe: &impl ConceptUniverse,
		graph: &impl LexicalContextGraph,
	) -> GraphDelta {
		filtered_delta(
			concept,
			universe,
			graph,
			&[RelationKind::Hypernym, RelationKind::Associated, RelationKind::Synonym],
			6,
			|edge, universe| universe.concept(edge.target).is_some(),
		)
	}

	fn candidates(
		&self,
		concept: ConceptId,
		graph: &impl LexicalContextGraph,
	) -> Vec<LexicalCandidate> {
		let mut candidates = Vec::new();
		let self_term = any_term(concept, graph)
			.map(|(term, _)| term)
			.unwrap_or_else(|| coined_root(self.seed, concept, Phonology::root_heavy()));
		candidates.push(LexicalCandidate {
			term: self_term,
			source_concept: concept,
			relation: RelationKind::Synonym,
			usage: any_term(concept, graph).map(|(_, usage)| usage).unwrap_or_default(),
		});
		for edge in graph.neighbors(concept) {
			if let Some((term, usage)) = any_term(edge.target, graph) {
				candidates.push(LexicalCandidate {
					term,
					source_concept: edge.target,
					relation: edge.relation,
					usage,
				});
			}
		}
		candidates
	}

	fn resolve(
		&self,
		concept: ConceptId,
		candidates: &[LexicalCandidate],
		graph: &impl LexicalContextGraph,
		_profile: &Profile,
	) -> LexicalRealization {
		if let Some(term) = established_term(concept, graph) {
			return LexicalRealization {
				term,
				provenance: vec![CandidateContribution {
					source_concept: concept,
					contribution: 1.0,
				}],
			};
		}

		let coined = coined_root(self.seed, concept, Phonology::root_heavy());
		let neighbors: Vec<&LexicalCandidate> = candidates
			.iter()
			.filter(|candidate| candidate.source_concept != concept)
			.collect();
		let chance = mix(self.seed ^ concept.as_u64()) % 10;
		if chance == 0 {
			if let (Some(left), Some(right)) = (neighbors.first(), neighbors.get(1)) {
				return LexicalRealization {
					term: blend(left.term.ipa.as_str(), right.term.ipa.as_str()),
					provenance: vec![
						CandidateContribution {
							source_concept: left.source_concept,
							contribution: 0.5,
						},
						CandidateContribution {
							source_concept: right.source_concept,
							contribution: 0.5,
						},
					],
				};
			}
		}

		LexicalRealization {
			term: coined,
			provenance: vec![CandidateContribution { source_concept: concept, contribution: 1.0 }],
		}
	}

	fn post_resolution(
		&self,
		concept: ConceptId,
		realization: &LexicalRealization,
		graph: &mut impl LexicalContextGraphMut,
		_profile: &Profile,
	) {
		reinforce(concept, realization, graph);
	}
}
