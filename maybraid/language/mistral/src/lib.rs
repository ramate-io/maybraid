//! Local Qwen integration for [`maybraid_language_core`].
//!
//! This crate owns `mistral.rs` and English response generation. Semantic
//! parse lives in [`maybraid_language_core`] via UDPipe.
//!
//! See [#911](https://github.com/ramate-io/maybraid/issues/911).

pub mod config;
mod convert;
pub mod device;
pub mod error;
pub mod model;
pub mod prompt;
pub mod schema;

pub use config::{
	bundled_model_path, MistralLanguageConfig, ParseGenerationConfig, ResponseGenerationConfig,
};
pub use device::InferenceDevice;
pub use error::MistralLanguageError;
pub use model::{MistralLanguageModel, ResponseRequest};
pub use schema::{GeneratedArgument, GeneratedClause, GeneratedReferent, GeneratedUtterance};

#[cfg(test)]
mod tests;
