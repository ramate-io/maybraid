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
	#[error("UDPipe model is missing at {path}")]
	UdpipeMissing { path: String },
	#[error("dependency parse failed: {0}")]
	DependencyParse(String),
	#[error("semantic marshal failed: {0}")]
	SemanticMarshal(String),
}

/// Stage-tagged pipeline failure. Keeps Qwen, UDPipe, marshalling,
/// concept lookup, and realization distinguishable at the CLI.
#[derive(Debug, Error)]
pub enum LanguagePipelineError {
	#[error("response generation failed: {0}")]
	ResponseGeneration(String),
	#[error("dependency parse failed: {0}")]
	DependencyParse(String),
	#[error("semantic marshal failed: {0}")]
	SemanticMarshal(String),
	#[error("concept resolution failed: {0}")]
	ConceptResolution(String),
	#[error("language render failed: {0}")]
	LanguageRender(String),
}

impl From<LanguageError> for LanguagePipelineError {
	fn from(error: LanguageError) -> Self {
		match error {
			LanguageError::DependencyParse(detail) => Self::DependencyParse(detail),
			LanguageError::UdpipeMissing { path } => {
				Self::DependencyParse(format!("UDPipe model is missing at {path}"))
			}
			LanguageError::SemanticMarshal(detail) => Self::SemanticMarshal(detail),
			LanguageError::MissingConcept(name) => Self::ConceptResolution(name),
			LanguageError::MissingLemma { lemma, pos } => {
				Self::ConceptResolution(format!("missing lemma {lemma:?} ({pos})"))
			}
			LanguageError::MissingSense { lemma, pos, sense } => {
				Self::ConceptResolution(format!("missing sense {sense} for {lemma:?} ({pos})"))
			}
			LanguageError::WordNet { path, detail } => {
				Self::ConceptResolution(format!("failed to load WordNet from {path}: {detail}"))
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pipeline_error_keeps_stages_distinct() {
		assert!(matches!(
			LanguagePipelineError::from(LanguageError::DependencyParse("ud".into())),
			LanguagePipelineError::DependencyParse(_)
		));
		assert!(matches!(
			LanguagePipelineError::from(LanguageError::MissingConcept("walk".into())),
			LanguagePipelineError::ConceptResolution(_)
		));
		assert!(matches!(
			LanguagePipelineError::from(LanguageError::SemanticMarshal("frame".into())),
			LanguagePipelineError::SemanticMarshal(_)
		));
	}
}
