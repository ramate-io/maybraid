//! Firearm catalog for inventory items. Kits stay in the `firearms` crate;
//! this is the bag identity and thumbnail path.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Named firearm bodies currently in `items/guns/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum FirearmMesh {
	Bullpup,
	Silopup,
	Reltor,
	Samsonist,
	Snailer,
}

impl FirearmMesh {
	pub const VALUES: &'static [Self] =
		&[Self::Bullpup, Self::Silopup, Self::Reltor, Self::Samsonist, Self::Snailer];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Bullpup => "bullpup",
			Self::Silopup => "silopup",
			Self::Reltor => "reltor",
			Self::Samsonist => "samsonist",
			Self::Snailer => "snailer",
		}
	}

	/// Unfitted catalog path relative to the `maybraid/assets` root.
	pub const fn path(self) -> &'static str {
		match self {
			Self::Bullpup => "items/guns/concepts/bullpup_full_concept.glb",
			Self::Silopup => "items/guns/concepts/silopup_full_concept.glb",
			Self::Reltor => "items/guns/bodies/reltor_body.glb",
			Self::Samsonist => "items/guns/bodies/samsonist_body.glb",
			Self::Snailer => "items/guns/bodies/snailer_body.glb",
		}
	}

	/// Nouns a rolled item of this mesh may be called.
	pub const fn nouns(self) -> &'static [&'static str] {
		match self {
			Self::Bullpup => &["Bullpup", "Carbine", "Short Rifle", "Compact", "Issue Rifle"],
			Self::Silopup => &["Silopup", "Suppressor", "Quiet Rifle", "Hush Gun", "Whisper"],
			Self::Reltor => &["Reltor", "Receiver", "Box Gun", "Service Rifle", "Latch Gun"],
			Self::Samsonist => &["Samsonist", "Long Rifle", "Pike", "Reach Gun", "Lance"],
			Self::Snailer => &["Snailer", "Coil Gun", "Helix", "Spiral", "Shell Gun"],
		}
	}
}

#[cfg(test)]
mod tests {
	use super::FirearmMesh;

	#[test]
	fn labels_are_kebab_case() {
		assert_eq!(FirearmMesh::Bullpup.label(), "bullpup");
		assert_eq!(FirearmMesh::Snailer.label(), "snailer");
	}
}
