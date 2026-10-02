//! Parser-independent Universal Dependencies document.
//!
//! Runtime parsers (UDPipe, later replacements) convert English into this
//! shape. Core semantic types stay free of parser crates.

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

use crate::error::LanguageError;

/// Sentence-local token id. UD ids are 1-based; `0` is the virtual root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenId(pub u32);

/// One or more UD sentences.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DependencyDocument {
	pub sentences: Vec<DependencySentence>,
}

/// Tokens of a single sentence, in linear order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DependencySentence {
	pub tokens: Vec<DependencyToken>,
}

/// One UD token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyToken {
	pub id: TokenId,
	pub text: String,
	pub lemma: String,
	pub pos: UniversalPos,
	pub features: MorphFeatures,
	pub head: Option<TokenId>,
	pub relation: DependencyRelation,
}

/// Coarse UD `UPOS`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UniversalPos {
	Noun,
	ProperNoun,
	Verb,
	Adjective,
	Adverb,
	Pronoun,
	Determiner,
	Adposition,
	Auxiliary,
	CoordinatingConjunction,
	SubordinatingConjunction,
	Numeral,
	Particle,
	Interjection,
	Punctuation,
	Other(String),
}

impl UniversalPos {
	pub fn parse(tag: &str) -> Self {
		match tag {
			"NOUN" => Self::Noun,
			"PROPN" => Self::ProperNoun,
			"VERB" => Self::Verb,
			"ADJ" => Self::Adjective,
			"ADV" => Self::Adverb,
			"PRON" => Self::Pronoun,
			"DET" => Self::Determiner,
			"ADP" => Self::Adposition,
			"AUX" => Self::Auxiliary,
			"CCONJ" => Self::CoordinatingConjunction,
			"SCONJ" => Self::SubordinatingConjunction,
			"NUM" => Self::Numeral,
			"PART" => Self::Particle,
			"INTJ" => Self::Interjection,
			"PUNCT" => Self::Punctuation,
			other => Self::Other(other.to_owned()),
		}
	}

	pub fn as_str(&self) -> &str {
		match self {
			Self::Noun => "NOUN",
			Self::ProperNoun => "PROPN",
			Self::Verb => "VERB",
			Self::Adjective => "ADJ",
			Self::Adverb => "ADV",
			Self::Pronoun => "PRON",
			Self::Determiner => "DET",
			Self::Adposition => "ADP",
			Self::Auxiliary => "AUX",
			Self::CoordinatingConjunction => "CCONJ",
			Self::SubordinatingConjunction => "SCONJ",
			Self::Numeral => "NUM",
			Self::Particle => "PART",
			Self::Interjection => "INTJ",
			Self::Punctuation => "PUNCT",
			Self::Other(tag) => tag,
		}
	}
}

/// Morphological features (`Key=Value|Key=Value`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MorphFeatures {
	pairs: BTreeMap<String, String>,
}

impl MorphFeatures {
	pub fn parse(feats: &str) -> Self {
		if feats.is_empty() || feats == "_" {
			return Self::default();
		}
		let mut pairs = BTreeMap::new();
		for part in feats.split('|') {
			if let Some((key, value)) = part.split_once('=') {
				pairs.insert(key.to_owned(), value.to_owned());
			}
		}
		Self { pairs }
	}

	pub fn get(&self, key: &str) -> Option<&str> {
		self.pairs.get(key).map(String::as_str)
	}

	pub fn has(&self, key: &str, value: &str) -> bool {
		self.get(key) == Some(value)
	}
}

/// UD dependency relation, including optional subtype (`obl:tmod`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyRelation {
	raw: String,
}

impl DependencyRelation {
	pub fn parse(name: impl Into<String>) -> Self {
		Self { raw: name.into() }
	}

	pub fn as_str(&self) -> &str {
		&self.raw
	}

	pub fn base(&self) -> &str {
		self.raw.split(':').next().unwrap_or(&self.raw)
	}

	pub fn is(&self, name: &str) -> bool {
		self.raw == name || self.base() == name
	}
}

/// English surface string → [`DependencyDocument`].
pub trait EnglishDependencyParser {
	fn parse(&self, text: &str) -> Result<DependencyDocument, LanguageError>;
}

impl DependencyDocument {
	pub fn debug_report(&self) -> String {
		let mut out = String::new();
		for (index, sentence) in self.sentences.iter().enumerate() {
			if index > 0 {
				out.push('\n');
			}
			let _ = writeln!(out, "Sentence {}:", index + 1);
			for token in &sentence.tokens {
				let head = token
					.head
					.and_then(|id| sentence.token(id))
					.map(|head| head.text.as_str())
					.unwrap_or("ROOT");
				let case = sentence
					.children(token.id)
					.find(|child| child.relation.is("case"))
					.map(|child| format!(" [case={}]", child.lemma))
					.unwrap_or_default();
				let _ = writeln!(
					out,
					"  {:<12} {:<8} {:<6} {:<12} -> {head}{case}",
					token.text,
					token.lemma,
					token.pos.as_str(),
					token.relation.as_str()
				);
			}
		}
		out
	}
}

impl DependencySentence {
	pub fn token(&self, id: TokenId) -> Option<&DependencyToken> {
		self.tokens.iter().find(|token| token.id == id)
	}

	pub fn children(&self, id: TokenId) -> impl Iterator<Item = &DependencyToken> {
		self.tokens.iter().filter(move |token| token.head == Some(id))
	}
}

impl fmt::Display for UniversalPos {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}
