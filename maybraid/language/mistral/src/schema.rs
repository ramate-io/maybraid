//! Intermediate serializable utterance. No SlotMap IDs.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Constrained-decode bounds. Unbounded arrays let Qwen fill the 40k context.
pub const MAX_REFERENTS: u32 = 16;
pub const MAX_CLAUSES: u32 = 16;
pub const MAX_ROOTS: u32 = 8;
pub const MAX_MODIFIERS: u32 = 8;
pub const MAX_RELATIVE_CLAUSES: u32 = 8;
pub const MAX_ARGUMENTS: u32 = 8;

/// Model-local semantic graph before conversion into [`maybraid_language_core::Utterance`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedUtterance {
	#[serde(default)]
	pub referents: Vec<GeneratedReferent>,
	#[serde(default)]
	pub clauses: Vec<GeneratedClause>,
	#[serde(default)]
	pub roots: Vec<usize>,
}

/// Discourse referent keyed by a model-local integer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedReferent {
	pub id: usize,
	pub concept: String,
	#[serde(default)]
	pub definiteness: Option<String>,
	#[serde(default)]
	pub number: Option<String>,
	#[serde(default)]
	pub modifiers: Vec<String>,
	#[serde(default)]
	pub relative_clauses: Vec<usize>,
}

/// Predication keyed by order in [`GeneratedUtterance::clauses`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedClause {
	pub predicate: String,
	#[serde(default)]
	pub arguments: Vec<GeneratedArgument>,
	#[serde(default)]
	pub tense: Option<String>,
	#[serde(default)]
	pub aspect: Option<String>,
	#[serde(default)]
	pub mood: Option<String>,
	#[serde(default)]
	pub polarity: Option<String>,
}

/// Semantic-role argument. Exactly one of `referent` or `clause` should be set.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedArgument {
	pub role: String,
	#[serde(default)]
	pub referent: Option<usize>,
	#[serde(default)]
	pub clause: Option<usize>,
}

impl GeneratedUtterance {
	pub fn from_json(value: &str) -> Result<Self, serde_json::Error> {
		serde_json::from_str(value)
	}

	/// JSON Schema used for constrained decoding.
	pub fn json_schema() -> Value {
		json!({
			"type": "object",
			"additionalProperties": false,
			"required": ["referents", "clauses", "roots"],
			"properties": {
				"referents": {
					"type": "array",
					"maxItems": MAX_REFERENTS,
					"items": {
						"type": "object",
						"additionalProperties": false,
						"required": ["id", "concept"],
						"properties": {
							"id": { "type": "integer", "minimum": 0 },
							"concept": { "type": "string" },
							"definiteness": {
								"type": "string",
								"enum": ["definite", "indefinite", "generic", "proper"]
							},
							"number": {
								"type": "string",
								"enum": ["singular", "plural", "many", "mass", "unspecified"]
							},
							"modifiers": {
								"type": "array",
								"maxItems": MAX_MODIFIERS,
								"items": { "type": "string" }
							},
							"relative_clauses": {
								"type": "array",
								"maxItems": MAX_RELATIVE_CLAUSES,
								"items": { "type": "integer", "minimum": 0 }
							}
						}
					}
				},
				"clauses": {
					"type": "array",
					"maxItems": MAX_CLAUSES,
					"items": {
						"type": "object",
						"additionalProperties": false,
						"required": ["predicate", "arguments"],
						"properties": {
							"predicate": { "type": "string" },
							"arguments": {
								"type": "array",
								"maxItems": MAX_ARGUMENTS,
								"items": argument_schema()
							},
							"tense": { "type": "string", "enum": ["past", "present", "future", "unspecified"] },
							"aspect": { "type": "string", "enum": ["simple", "progressive", "perfect", "unspecified"] },
							"mood": { "type": "string", "enum": ["indicative", "imperative", "interrogative", "unspecified"] },
							"polarity": { "type": "string", "enum": ["affirmative", "negative"] }
						}
					}
				},
				"roots": {
					"type": "array",
					"maxItems": MAX_ROOTS,
					"items": { "type": "integer", "minimum": 0 }
				}
			}
		})
	}
}

fn argument_schema() -> Value {
	let role = json!({
		"type": "string",
		"enum": [
			"Agent", "Patient", "Theme", "Experiencer",
			"Recipient", "Beneficiary", "Instrument",
			"Location", "Source", "Goal", "Possessor",
			"Cause", "Content", "Classification",
			"Manner", "Rate", "Time"
		]
	});
	// llguidance rejects JSON Schema `not`. Exclusive property sets
	// encode role+referent XOR role+clause.
	json!({
		"oneOf": [
			{
				"type": "object",
				"additionalProperties": false,
				"required": ["role", "referent"],
				"properties": {
					"role": role.clone(),
					"referent": { "type": "integer", "minimum": 0 }
				}
			},
			{
				"type": "object",
				"additionalProperties": false,
				"required": ["role", "clause"],
				"properties": {
					"role": role,
					"clause": { "type": "integer", "minimum": 0 }
				}
			}
		]
	})
}
