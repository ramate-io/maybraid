//! Maybraid Language Core
//!
//! Base lexicon layer for procedurally lexicalizing structured semantics.
//!
//! ```text
//! Utterance -> Concepts -> Lexical Context -> Candidates -> Base Terms -> LanguageOutput
//! ```
//!
//! This crate implements the [semantic utterance → procedural lexicon
//! POC](https://github.com/ramate-io/maybraid/issues/897). A tiny
//! [`grammar`] linearizer can print a formatted IPA string; agreement, case,
//! and historical evolution remain out of scope.
//!
//! WordNet 3.1 is the initial English concept universe. The bundled extract and
//! [`wordnet::WORDNET_LICENSE`] reproduce Princeton's license as required.
//! Princeton University does not endorse this project. See
//! [`wordnet::WORDNET_CITATION`].

pub mod concept;
pub mod error;
pub mod grammar;
pub mod graph;
pub mod lexicalizer;
pub mod marshall;
pub mod output;
pub mod poc;
pub mod profile;
pub mod term;
pub mod utterance;
pub mod wordnet;

pub use concept::{
	Concept, ConceptId, ConceptRelation, ConceptUniverse, EnglishSenseLookup, NeighborhoodRequest,
	Pos, RelationKind,
};
pub use error::LanguageError;
pub use grammar::{
	ClauseOrder, IpaUtterance, ModifierPlacement, RelativePlacement, RoleParticles, SurfaceGrammar,
};
pub use graph::{
	ConceptEdge, GraphDelta, InMemoryLexicalGraph, LexicalContextGraph, LexicalContextGraphMut,
	LexicalEdge, LexicalUpdate,
};
pub use lexicalizer::{
	CandidateContribution, CompositionalLexicalizer, LexicalCandidate, LexicalRealization,
	Lexicalizer, RootHeavyLexicalizer, GENERATOR_VERSION,
};
pub use marshall::{ConceptMarshaller, ConceptUse, DefaultMarshaller, SemanticNode};
pub use output::{LanguageOutput, ResolvedConcept};
pub use poc::{poc_universe, PocLexicon};
pub use profile::{Profile, Register};
pub use term::{Ipa, Term, TermId, Usage};
pub use utterance::{
	Argument, Aspect, Clause, ClauseId, Definiteness, Modifier, Mood, Number, Polarity, Referent,
	ReferentId, SemanticRole, SemanticValue, Tense, Utterance,
};
pub use wordnet::{WordNetConceptUniverse, WORDNET_CITATION, WORDNET_LICENSE};

#[cfg(test)]
mod tests;
