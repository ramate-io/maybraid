//! Translate a subset of English kind terms in a tile language.

use maybraid_language_core::lexicalizer::mix;
use maybraid_language_core::{
	CompositionalLexicalizer, ConceptId, ConceptUniverse, InMemoryLexicalGraph, Lexicalizer,
	ModifierPlacement, Profile, RootHeavyLexicalizer,
};

use crate::bundle::{LanguageBundle, LexiconFamily};
use crate::catalog::KindConceptUniverse;

/// Assigned place name. Overlay concept is the proper-name handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceName {
	pub surface: String,
	pub english: Vec<String>,
	pub overlay: ConceptId,
	pub language_seed: u64,
}

impl PlaceName {
	pub fn translate(bundle: &LanguageBundle, english: &[String], pick_seed: u64) -> Self {
		let chosen = pick_terms(english, pick_seed);
		let mut catalog = KindConceptUniverse::new();
		catalog.intern_all(chosen.iter());
		let mut graph = InMemoryLexicalGraph::default();
		let profile = Profile::neutral();
		let mut forms = Vec::new();
		for word in &chosen {
			let Some(concept) = catalog.resolve_english(word).into_iter().next() else {
				continue;
			};
			let term = lexicalize(bundle, concept, &catalog, &mut graph, &profile);
			forms.push(term);
		}
		if matches!(bundle.grammar.modifiers, ModifierPlacement::AfterNoun) && forms.len() > 1 {
			if let Some(head) = forms.pop() {
				forms.insert(0, head);
			}
		}
		let surface = forms.join(" ");
		let overlay = ConceptId::overlay(mix(bundle.seed ^ stable_surface(&surface)) as u32);
		Self { surface, english: chosen, overlay, language_seed: bundle.seed }
	}
}

fn lexicalize(
	bundle: &LanguageBundle,
	concept: ConceptId,
	universe: &impl ConceptUniverse,
	graph: &mut InMemoryLexicalGraph,
	profile: &Profile,
) -> String {
	let realization = match bundle.lexicon {
		LexiconFamily::Compositional => {
			CompositionalLexicalizer::new(bundle.seed).lexicalize(concept, universe, graph, profile)
		}
		LexiconFamily::RootHeavy => {
			RootHeavyLexicalizer::new(bundle.seed).lexicalize(concept, universe, graph, profile)
		}
	};
	realization.term.ipa.0
}

fn pick_terms(english: &[String], pick_seed: u64) -> Vec<String> {
	if english.is_empty() {
		return Vec::new();
	}
	let mut unique = Vec::new();
	for word in english {
		if !unique.iter().any(|seen: &String| seen == word) {
			unique.push(word.clone());
		}
	}
	if unique.is_empty() {
		return unique;
	}
	let take = (1 + (mix(pick_seed) as usize % 3.min(unique.len()))).min(unique.len());
	let mut rng = mix(pick_seed ^ 0xC0FF);
	let mut order = unique;
	for i in 0..order.len() {
		rng = mix(rng);
		let j = (rng as usize) % order.len();
		order.swap(i, j);
	}
	order.truncate(take);
	order
}

fn stable_surface(surface: &str) -> u64 {
	let mut h = 0x811c_9dc5_u64;
	for byte in surface.as_bytes() {
		h = mix(h ^ u64::from(*byte));
	}
	h
}
