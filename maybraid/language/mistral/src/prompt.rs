//! Short, deterministic prompts. No ECS or world handles.

use serde_json::{json, Value};

/// English reply given a bounded JSON context blob.
pub struct RespondPrompt;

impl RespondPrompt {
	pub const SYSTEM: &str = "\
Respond naturally and concisely in one or two short English sentences.
Prefer clear grammatical English with explicit referents.
Avoid unnecessary ambiguity, sentence fragments, or obscure idioms.
Use only the supplied world/context JSON.
Do not mention the JSON, tools, or hidden state.";

	pub fn user(input: &str, context: &Value) -> String {
		format!(
			"Respond naturally to the supplied statement using the provided world/context JSON.\n\n\
Input:\n{input:?}\n\n\
Context:\n{}",
			context
		)
	}

	pub fn empty_context() -> Value {
		json!({
			"speaker": {},
			"listener": {},
			"location": {},
			"nearby_entities": [],
			"known_facts": []
		})
	}
}
