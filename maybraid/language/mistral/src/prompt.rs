//! Short, deterministic prompts. No ECS or world handles.

use serde_json::{json, Value};

/// English → structured semantic graph.
pub struct ParsePrompt;

impl ParsePrompt {
	pub const SYSTEM: &str = "\
Convert English into the provided semantic JSON schema.
Represent meaning rather than English syntax.
Reuse referent IDs for coreference.
Do not emit subject/object roles; use semantic roles.
Do not emit SlotMap or other runtime handles.
Think of predicates as events or states, not English verbs that must be copied.
Concept labels must be WordNet lemmas (`walk`, `book`), not surface noun phrases (`a walk`, `the book`).
Each argument must set exactly one of `referent` or `clause` to an id.
Leave thinking mode off; emit only JSON.";

	pub fn user(sentence: &str) -> String {
		format!(
			"Convert the following English sentence into the provided semantic schema.\n\n\
Represent meaning rather than English syntax.\n\
Reuse referent IDs for coreference.\n\
Do not emit subject/object roles; use semantic roles.\n\
Use WordNet lemmas for concepts, without articles.\n\
Each argument must set exactly one of referent or clause.\n\n\
Sentence:\n{sentence:?}"
		)
	}
}

/// English reply given a bounded JSON context blob.
pub struct RespondPrompt;

impl RespondPrompt {
	pub const SYSTEM: &str = "\
Respond naturally in one or two short English sentences.
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
