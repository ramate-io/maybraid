//! Discover skill-map items in a character bag.
//!
//! Kind plus seed is the whole identity: the same kind with a different seed
//! is a different map. The live session presents one equipped spec at a time.

use serde::{Deserialize, Serialize};

/// Authored Discover map a character can carry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillMapKind {
	#[default]
	Fireball,
	Dumbwave,
}

impl SkillMapKind {
	pub const VALUES: &'static [Self] = &[Self::Fireball, Self::Dumbwave];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Fireball => "fireball",
			Self::Dumbwave => "dumbwave",
		}
	}

	pub const fn display_name(self) -> &'static str {
		match self {
			Self::Fireball => "Fireball",
			Self::Dumbwave => "Dumbwave",
		}
	}

	pub const fn adjectives(self) -> &'static [&'static str] {
		match self {
			Self::Fireball => &["Searing", "Ember", "Cinder", "Solar", "Ashen"],
			Self::Dumbwave => &["Dull", "Hollow", "Mute", "Blank", "Haze"],
		}
	}
}

/// One owned skill map: which effect, and which noise field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillMapSpec {
	pub kind: SkillMapKind,
	pub seed: u32,
}

impl SkillMapSpec {
	pub const fn new(kind: SkillMapKind, seed: u32) -> Self {
		Self { kind, seed }
	}
}
