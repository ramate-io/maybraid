//! Walk an utterance and collect concepts that need lexicalization.

use crate::concept::ConceptId;
use crate::utterance::{ClauseId, ReferentId, SemanticValue, Utterance};

/// One concept occurrence in the semantic graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConceptUse {
	pub concept: ConceptId,
	pub source: SemanticNode,
}

/// Where a concept was attached in the utterance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticNode {
	Predicate(ClauseId),
	Referent(ReferentId),
	Modifier { referent: ReferentId, index: usize },
}

/// Collects concept uses. Repeated concepts stay listed; context expansion is separate.
pub trait ConceptMarshaller {
	fn concepts(&self, utterance: &Utterance) -> Vec<ConceptUse>;
}

/// Depth-first walk from utterance roots, then remaining clauses and referents.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultMarshaller;

impl ConceptMarshaller for DefaultMarshaller {
	fn concepts(&self, utterance: &Utterance) -> Vec<ConceptUse> {
		let mut uses = Vec::new();
		let mut seen_clauses = Vec::new();
		let mut seen_referents = Vec::new();

		for root in &utterance.roots {
			walk_clause(utterance, *root, &mut uses, &mut seen_clauses, &mut seen_referents);
		}
		for (clause_id, _) in &utterance.clauses {
			walk_clause(utterance, clause_id, &mut uses, &mut seen_clauses, &mut seen_referents);
		}
		for (referent_id, _) in &utterance.referents {
			walk_referent(
				utterance,
				referent_id,
				&mut uses,
				&mut seen_clauses,
				&mut seen_referents,
			);
		}
		uses
	}
}

fn walk_clause(
	utterance: &Utterance,
	clause_id: ClauseId,
	uses: &mut Vec<ConceptUse>,
	seen_clauses: &mut Vec<ClauseId>,
	seen_referents: &mut Vec<ReferentId>,
) {
	if seen_clauses.contains(&clause_id) {
		return;
	}
	seen_clauses.push(clause_id);
	let Some(clause) = utterance.clauses.get(clause_id) else {
		return;
	};
	uses.push(ConceptUse { concept: clause.predicate, source: SemanticNode::Predicate(clause_id) });
	for argument in &clause.arguments {
		match argument.value {
			SemanticValue::Referent(referent) => {
				walk_referent(utterance, referent, uses, seen_clauses, seen_referents);
			}
			SemanticValue::Clause(nested) => {
				walk_clause(utterance, nested, uses, seen_clauses, seen_referents);
			}
		}
	}
}

fn walk_referent(
	utterance: &Utterance,
	referent_id: ReferentId,
	uses: &mut Vec<ConceptUse>,
	seen_clauses: &mut Vec<ClauseId>,
	seen_referents: &mut Vec<ReferentId>,
) {
	if seen_referents.contains(&referent_id) {
		return;
	}
	seen_referents.push(referent_id);
	let Some(referent) = utterance.referents.get(referent_id) else {
		return;
	};
	uses.push(ConceptUse {
		concept: referent.concept,
		source: SemanticNode::Referent(referent_id),
	});
	for (index, modifier) in referent.modifiers.iter().enumerate() {
		uses.push(ConceptUse {
			concept: modifier.concept,
			source: SemanticNode::Modifier { referent: referent_id, index },
		});
	}
	for clause in &referent.relative_clauses {
		walk_clause(utterance, *clause, uses, seen_clauses, seen_referents);
	}
}
