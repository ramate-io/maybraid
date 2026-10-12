//! `--respond` path: optional Qwen / mistral.rs English generation.

use std::path::PathBuf;

use anyhow::Result;
use maybraid_language_core::{
	DependencyDocument, EnglishDependencyParser, EnglishSemanticMarshaller, SemanticMarshaller,
	UdpipeEnglishParser, Utterance, WordNetConceptUniverse,
};

#[cfg(feature = "llm")]
use maybraid_language_core::LanguagePipelineError;

pub struct RespondOutput {
	pub english: String,
	pub parsed: DependencyDocument,
	pub utterance: Utterance,
	pub universe: WordNetConceptUniverse,
}

#[cfg(feature = "llm")]
pub async fn run_respond(
	input: &str,
	parser: &UdpipeEnglishParser,
	marshaller: &EnglishSemanticMarshaller,
	model_path: Option<PathBuf>,
	force_cpu: bool,
) -> Result<RespondOutput> {
	use maybraid_language_mistral::{
		bundled_model_path, MistralLanguageConfig, MistralLanguageModel, ResponseRequest,
	};

	let model_path = model_path.unwrap_or_else(bundled_model_path);
	let mut config = MistralLanguageConfig::from_path(model_path);
	if force_cpu {
		config = config.with_force_cpu(true);
	}
	let model = MistralLanguageModel::load(config)
		.await
		.map_err(|error| LanguagePipelineError::ResponseGeneration(error.to_string()))?;
	let english = model
		.respond(ResponseRequest::new(input))
		.await
		.map_err(|error| LanguagePipelineError::ResponseGeneration(error.to_string()))?;
	let parsed = parser.parse(&english).map_err(LanguagePipelineError::from)?;
	let universe = super::poc_universe_for(&parsed)?;
	let utterance = marshaller.marshal(&parsed, &universe).map_err(LanguagePipelineError::from)?;
	Ok(RespondOutput {
		english,
		parsed,
		utterance,
		universe,
	})
}

#[cfg(not(feature = "llm"))]
pub async fn run_respond(
	_input: &str,
	_parser: &UdpipeEnglishParser,
	_marshaller: &EnglishSemanticMarshaller,
	_model_path: Option<PathBuf>,
	_force_cpu: bool,
) -> Result<RespondOutput> {
	anyhow::bail!(
		"`--respond` requires the `llm` feature when building `maybraid-language` \
		 (for example: `cargo build -p maybraid-language --features llm`; on Apple Silicon add `metal`)"
	);
}
