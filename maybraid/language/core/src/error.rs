//! Recoverable failures for universe loading and utterance construction.

use thiserror::Error;

/// Errors produced while loading WordNet data or assembling a POC utterance.
#[derive(Debug, Error)]
pub enum LanguageError {
	#[error("failed to read WordNet file {path}: {source}")]
	Io {
		path: String,
		#[source]
		source: std::io::Error,
	},
	#[error("malformed WordNet record in {path}: {detail}")]
	Parse { path: String, detail: String },
	#[error("WordNet extract is missing lemma {lemma:?} ({pos})")]
	MissingLemma { lemma: String, pos: String },
	#[error("WordNet extract has no sense {sense} for {lemma:?} ({pos})")]
	MissingSense { lemma: String, pos: String, sense: usize },
	#[error("required concept {0} is not in the concept universe")]
	MissingConcept(String),
}
