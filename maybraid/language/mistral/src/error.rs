//! Recoverable failures for model load, inference, and schema conversion.

use std::path::PathBuf;

use maybraid_language_core::LanguageError;
use thiserror::Error;

/// Errors from the Mistral/Qwen language service.
#[derive(Debug, Error)]
pub enum MistralLanguageError {
	#[error("Qwen GGUF is missing at {}", .path.display())]
	ModelMissing { path: PathBuf },
	#[error("failed to load Qwen from {}: {detail}", .path.display())]
	Load { path: PathBuf, detail: String },
	#[error("model inference failed: {0}")]
	Inference(String),
	#[error("structured model output was invalid: {0}")]
	InvalidOutput(String),
	#[error(transparent)]
	Language(#[from] LanguageError),
}

impl MistralLanguageError {
	pub fn load(path: impl Into<PathBuf>, detail: impl ToString) -> Self {
		Self::Load { path: path.into(), detail: detail.to_string() }
	}

	pub fn inference(detail: impl ToString) -> Self {
		Self::Inference(detail.to_string())
	}

	pub fn invalid_output(detail: impl ToString) -> Self {
		Self::InvalidOutput(detail.to_string())
	}
}
