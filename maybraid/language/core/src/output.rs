//! Language output retains the semantic graph plus base lexical terms.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::concept::{ConceptId, ConceptUniverse};
use crate::graph::LexicalContextGraph;
use crate::lexicalizer::{LexicalRealization, Lexicalizer};
use crate::marshall::{ConceptMarshaller, ConceptUse, DefaultMarshaller};
use crate::profile::Profile;
use crate::utterance::Utterance;

/// One concept occurrence after lexicalization.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedConcept {
	pub source: ConceptUse,
	pub realization: LexicalRealization,
}

/// Semantic graph plus per-use base terms. Syntax/morphology come later.
#[derive(Clone, Debug)]
pub struct LanguageOutput {
	pub utterance: Utterance,
	pub lexicalizations: Vec<ResolvedConcept>,
}

impl LanguageOutput {
	pub fn render(
		utterance: Utterance,
		lexicalizer: &impl Lexicalizer,
		universe: &impl ConceptUniverse,
		graph: &mut impl crate::graph::LexicalContextGraphMut,
		profile: &Profile,
	) -> Self {
		Self::render_with(utterance, lexicalizer, universe, graph, profile, &DefaultMarshaller)
	}

	pub fn render_with(
		utterance: Utterance,
		lexicalizer: &impl Lexicalizer,
		universe: &impl ConceptUniverse,
		graph: &mut impl crate::graph::LexicalContextGraphMut,
		profile: &Profile,
		marshaller: &impl ConceptMarshaller,
	) -> Self {
		let uses = marshaller.concepts(&utterance);
		let mut cache: HashMap<ConceptId, LexicalRealization> = HashMap::new();
		let mut lexicalizations = Vec::new();
		for source in uses {
			let realization = cache.entry(source.concept).or_insert_with(|| {
				lexicalizer.lexicalize(source.concept, universe, graph, profile)
			});
			lexicalizations.push(ResolvedConcept { source, realization: realization.clone() });
		}
		Self { utterance, lexicalizations }
	}

	pub fn realization_of(&self, concept: ConceptId) -> Option<&LexicalRealization> {
		self.lexicalizations
			.iter()
			.find(|item| item.source.concept == concept)
			.map(|item| &item.realization)
	}

	/// Inspectable procedural trace for one concept.
	pub fn debug_concept(
		&self,
		concept: ConceptId,
		universe: &impl ConceptUniverse,
		graph: &impl LexicalContextGraph,
	) -> Option<String> {
		let realization = self.realization_of(concept)?;
		let mut out = String::new();
		let label = universe
			.concept(concept)
			.map(|c| c.primary_gloss().to_owned())
			.unwrap_or_else(|| concept.to_string());
		let _ = writeln!(out, "{label}");
		let _ = writeln!(out, "  canonical neighborhood:");
		let neighbors: Vec<_> = graph.neighbors(concept).collect();
		if neighbors.is_empty() {
			let _ = writeln!(out, "    (none imported)");
		}
		for edge in &neighbors {
			let name = universe
				.concept(edge.target)
				.map(|c| c.primary_gloss().to_owned())
				.unwrap_or_else(|| edge.target.to_string());
			let _ = writeln!(out, "    {name}");
		}
		let _ = writeln!(out, "  existing lexical material:");
		let mut wrote_material = false;
		for edge in &neighbors {
			for lex in graph.lexicalizations(edge.target) {
				if let Some(term) = graph.term(lex.term) {
					let name = universe
						.concept(edge.target)
						.map(|c| c.primary_gloss().to_owned())
						.unwrap_or_else(|| edge.target.to_string());
					let _ = writeln!(out, "    {name} {}", term.ipa);
					wrote_material = true;
				}
			}
		}
		if !wrote_material {
			let _ = writeln!(out, "    (none)");
		}
		let _ = writeln!(out, "  realization:");
		let _ = writeln!(out, "    {}", realization.term.ipa);
		let _ = writeln!(out, "  provenance:");
		for part in &realization.provenance {
			let name = universe
				.concept(part.source_concept)
				.map(|c| c.primary_gloss().to_owned())
				.unwrap_or_else(|| part.source_concept.to_string());
			let _ = writeln!(out, "    {name} {:.2}", part.contribution);
		}
		Some(out)
	}

	pub fn debug_report(
		&self,
		title: &str,
		universe: &impl ConceptUniverse,
		graph: &impl LexicalContextGraph,
	) -> String {
		let mut seen = Vec::new();
		let mut out = format!("{title}\n\n");
		for item in &self.lexicalizations {
			if seen.contains(&item.source.concept) {
				continue;
			}
			seen.push(item.source.concept);
			if let Some(block) = self.debug_concept(item.source.concept, universe, graph) {
				out.push_str(&block);
				out.push('\n');
			}
		}
		out
	}
}
