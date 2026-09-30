//! Base lexical forms and concept-term usage.

use slotmap::new_key_type;

new_key_type! {
	/// Stable handle for a stored [`Term`].
	pub struct TermId;
}

/// Base lexical form. Morphology is out of scope for this layer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Term {
	pub ipa: Ipa,
}

impl Term {
	pub fn new(ipa: impl Into<Ipa>) -> Self {
		Self { ipa: ipa.into() }
	}
}

/// IPA string. A later phonology issue may replace this with structured phones.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ipa(pub String);

impl Ipa {
	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl From<String> for Ipa {
	fn from(value: String) -> Self {
		Self(value)
	}
}

impl From<&str> for Ipa {
	fn from(value: &str) -> Self {
		Self(value.to_owned())
	}
}

impl std::fmt::Display for Ipa {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "/{}/", self.0)
	}
}

/// How a term is used for a particular concept.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Usage {
	pub literal: f32,
	pub metaphorical: f32,
	pub intellectual: f32,
	pub borrowing: f32,
	pub frequency: f32,
	pub familiarity: f32,
}

impl Usage {
	pub fn novel() -> Self {
		Self {
			literal: 1.0,
			metaphorical: 0.0,
			intellectual: 0.0,
			borrowing: 0.0,
			frequency: 0.0,
			familiarity: 0.0,
		}
	}

	pub fn is_established(self) -> bool {
		self.familiarity >= 0.15 || self.frequency >= 1.0
	}

	pub fn reinforced(self) -> Self {
		Self {
			frequency: self.frequency + 1.0,
			familiarity: (self.familiarity * 0.8 + 0.25).min(1.0),
			..self
		}
	}

	pub fn preference(self) -> f32 {
		self.familiarity * 0.6 + self.frequency.min(8.0) * 0.05 + self.literal * 0.2
	}
}

impl Default for Usage {
	fn default() -> Self {
		Self::novel()
	}
}
