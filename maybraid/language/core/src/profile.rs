//! Speaker profile. Resolution may vary without changing the semantic graph.

/// Lexical resolution context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
	pub register: Register,
}

impl Profile {
	pub const NEUTRAL: Self = Self { register: Register::Neutral };

	pub fn neutral() -> Self {
		Self::NEUTRAL
	}

	pub fn formal() -> Self {
		Self { register: Register::Formal }
	}

	pub fn familiar() -> Self {
		Self { register: Register::Familiar }
	}
}

impl Default for Profile {
	fn default() -> Self {
		Self::neutral()
	}
}

/// Register can bias compounding versus reuse. Dialect and idiolect come later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Register {
	#[default]
	Neutral,
	Formal,
	Familiar,
}
