//! Recoverable failures for universe loading and utterance construction.

use thiserror::Error;

/// Errors produced while loading WordNet data or assembling a POC utterance.
#[derive(Debug, Error)]
pub enum LanguageError {
	#[error("failed to load WordNet from {path}: {detail}")]
	WordNet { path: String, detail: String },
	#[error("WordNet is missing lemma {lemma:?} ({pos})")]
	MissingLemma { lemma: String, pos: String },
	#[error("WordNet has no sense {sense} for {lemma:?} ({pos})")]
	MissingSense { lemma: String, pos: String, sense: usize },
	#[error("required concept {0} is not in the concept universe")]
	MissingConcept(String),
}
