//! Language A: reuse, then compound, then coin.

use crate::concept::{ConceptId, ConceptUniverse, RelationKind};
use crate::graph::{LexicalContextGraph, LexicalContextGraphMut};
use crate::lexicalizer::phonology::{coined_root, compound, Phonology};
use crate::lexicalizer::{
	any_term, established_term, filtered_delta, reinforce, CandidateContribution, LexicalCandidate,
	LexicalRealization, Lexicalizer,
};
use crate::profile::{Profile, Register};
use crate::term::{Term, Usage};

/// Compositional lexicalizer. Biases toward families of related forms.
#[derive(Clone, Debug)]
pub struct CompositionalLexicalizer {
	pub seed: u64,
}

impl CompositionalLexicalizer {
	pub const DEFAULT_SEED: u64 = 0xA11A_A11A_A11A_A11A;

	pub fn new(seed: u64) -> Self {
		Self { seed }
	}

	fn useful_neighbor(edge: &crate::graph::ConceptEdge, universe: &impl ConceptUniverse) -> bool {
		if edge.weight < 0.35 {
			return false;
		}
		let Some(concept) = universe.concept(edge.target) else {
			return false;
		};
		let gloss = concept.primary_gloss();
		if matches!(edge.relation, RelationKind::Hyponym) {
			return is_compact_gloss(gloss);
		}
		true
	}

	fn material_for(
		&self,
		concept: ConceptId,
		graph: &impl LexicalContextGraph,
	) -> (Term, Usage, RelationKind) {
		if let Some((term, usage)) = any_term(concept, graph) {
			return (term, usage, RelationKind::Synonym);
		}
		(
			coined_root(self.seed, concept, Phonology::compositional()),
			Usage::novel(),
			RelationKind::Associated,
		)
	}
}

impl Default for CompositionalLexicalizer {
	fn default() -> Self {
		Self::new(Self::DEFAULT_SEED)
	}
}

impl Lexicalizer for CompositionalLexicalizer {
	fn expand_context(
		&self,
		concept: ConceptId,
		universe: &impl ConceptUniverse,
		graph: &impl LexicalContextGraph,
	) -> crate::graph::GraphDelta {
		filtered_delta(
			concept,
			universe,
			graph,
			&[
				RelationKind::Synonym,
				RelationKind::Hypernym,
				RelationKind::Hyponym,
				RelationKind::PartOf,
				RelationKind::Associated,
			],
			10,
			Self::useful_neighbor,
		)
	}

	fn candidates(
		&self,
		concept: ConceptId,
		graph: &impl LexicalContextGraph,
	) -> Vec<LexicalCandidate> {
		let mut candidates = Vec::new();
		let (self_term, self_usage, relation) = self.material_for(concept, graph);
		candidates.push(LexicalCandidate {
			term: self_term,
			source_concept: concept,
			relation,
			usage: self_usage,
		});
		let mut neighbors: Vec<_> = graph.neighbors(concept).collect();
		neighbors
			.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
		for edge in neighbors {
			let (term, usage, _) = self.material_for(edge.target, graph);
			candidates.push(LexicalCandidate {
				term,
				source_concept: edge.target,
				relation: edge.relation,
				usage,
			});
		}
		candidates
	}

	fn resolve(
		&self,
		concept: ConceptId,
		candidates: &[LexicalCandidate],
		graph: &impl LexicalContextGraph,
		profile: &Profile,
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

		let self_term = candidates
			.iter()
			.find(|candidate| candidate.source_concept == concept)
			.map(|candidate| candidate.term.clone())
			.unwrap_or_else(|| coined_root(self.seed, concept, Phonology::compositional()));

		let mut donors: Vec<&LexicalCandidate> = candidates
			.iter()
			.filter(|candidate| candidate.source_concept != concept)
			.filter(|candidate| {
				matches!(
					candidate.relation,
					RelationKind::Hypernym
						| RelationKind::Hyponym
						| RelationKind::PartOf
						| RelationKind::Associated
						| RelationKind::Synonym
				)
			})
			.collect();
		donors.sort_by(|a, b| {
			let aw = donor_score(a);
			let bw = donor_score(b);
			bw.partial_cmp(&aw).unwrap_or(std::cmp::Ordering::Equal)
		});

		let prefer_compound = matches!(profile.register, Register::Formal | Register::Neutral);
		if prefer_compound && donors.len() >= 2 {
			let left = &donors[0];
			let right = &donors[1];
			let term = compound(left.term.ipa.as_str(), right.term.ipa.as_str());
			return LexicalRealization {
				term,
				provenance: vec![
					CandidateContribution {
						source_concept: left.source_concept,
						contribution: 0.55,
					},
					CandidateContribution {
						source_concept: right.source_concept,
						contribution: 0.45,
					},
				],
			};
		}
		if let Some(donor) = donors.first() {
			if matches!(profile.register, Register::Familiar) {
				return LexicalRealization {
					term: donor.term.clone(),
					provenance: vec![CandidateContribution {
						source_concept: donor.source_concept,
						contribution: 1.0,
					}],
				};
			}
		}

		LexicalRealization {
			term: self_term,
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
		for contribution in &realization.provenance {
			if contribution.source_concept == concept {
				continue;
			}
			let (term, usage, _) = self.material_for(contribution.source_concept, graph);
			if !usage.is_established() {
				graph.apply_update(crate::graph::LexicalUpdate::Bind {
					concept: contribution.source_concept,
					term,
					usage: Usage::novel().reinforced(),
				});
			}
		}
	}
}

fn is_compact_gloss(gloss: &str) -> bool {
	!gloss.contains('_') && gloss.chars().count() <= 10
}

fn donor_score(candidate: &LexicalCandidate) -> f32 {
	let compactness = if candidate.term.ipa.as_str().chars().count() <= 6 { 0.25 } else { 0.0 };
	let relation_bias = match candidate.relation {
		RelationKind::Hypernym | RelationKind::PartOf => 0.2,
		RelationKind::Associated | RelationKind::Synonym => 0.15,
		RelationKind::Hyponym => 0.0,
		RelationKind::Antonym => -0.2,
	};
	candidate.usage.preference() + candidate.relation.default_weight() + compactness + relation_bias
}
