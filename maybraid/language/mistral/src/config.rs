//! Load and generation settings for the local Qwen model.

use std::env;
use std::path::PathBuf;

/// Default development asset. Not embedded; resolve it from disk.
pub const BUNDLED_MODEL_FILE: &str = "qwen3-1.7b-q4.gguf";

/// Path to the vendored Qwen GGUF used when callers omit `--model-path`.
pub fn bundled_model_path() -> PathBuf {
	if let Ok(path) = env::var("MAYBRAID_QWEN_GGUF") {
		return PathBuf::from(path);
	}
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../assets/models")
		.join(BUNDLED_MODEL_FILE)
}

/// How the model is located and how each task samples.
#[derive(Clone, Debug, PartialEq)]
pub struct MistralLanguageConfig {
	pub model_path: PathBuf,
	pub parse: ParseGenerationConfig,
	pub respond: ResponseGenerationConfig,
	pub force_cpu: bool,
}

impl MistralLanguageConfig {
	pub fn from_path(model_path: PathBuf) -> Self {
		Self {
			model_path,
			parse: ParseGenerationConfig::default(),
			respond: ResponseGenerationConfig::default(),
			force_cpu: force_cpu_from_env(),
		}
	}

	pub fn with_force_cpu(mut self, force_cpu: bool) -> Self {
		self.force_cpu = force_cpu;
		self
	}

	pub fn bundled() -> Self {
		Self::from_path(bundled_model_path())
	}
}

/// Default parse cap. The GGUF context is 40960; unconstrained decode
/// will fill it. A short utterance JSON is a few hundred tokens.
pub const PARSE_MAX_LEN: usize = 256;

/// One or two short English sentences.
pub const RESPOND_MAX_LEN: usize = 128;

/// Low-temperature structured parse. Prefer greedy decoding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParseGenerationConfig {
	pub temperature: f32,
	pub max_len: usize,
}

impl Default for ParseGenerationConfig {
	fn default() -> Self {
		Self { temperature: 0.0, max_len: PARSE_MAX_LEN }
	}
}

/// Conversational response sampling, independent of parse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResponseGenerationConfig {
	pub temperature: f32,
	pub top_p: f32,
	pub max_len: usize,
}

impl Default for ResponseGenerationConfig {
	fn default() -> Self {
		Self { temperature: 0.7, top_p: 0.9, max_len: RESPOND_MAX_LEN }
	}
}

fn force_cpu_from_env() -> bool {
	matches!(env::var("MAYBRAID_LANGUAGE_FORCE_CPU").as_deref(), Ok("1" | "true" | "TRUE"))
}
