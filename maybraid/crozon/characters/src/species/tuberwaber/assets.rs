//! Tuberwaber asset catalog: tuberwaber body + head on the humanoid biped stack.

use clap::ValueEnum;

use crate::assets::AssetPath;

pub use crate::species::common::{EyeMesh, HairMesh, MouthMesh, NoseMesh};

const BODY_TUBERWABER: AssetPath = AssetPath::new("characters/bodies/biped/tuberwaber_body.glb");
const HEAD_TUBERWABER: AssetPath = AssetPath::new("characters/heads/tuberwaber_head.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TuberwaberBodyMesh {
	#[default]
	Tuberwaber,
}

impl TuberwaberBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Tuberwaber];

	pub const fn label(self) -> &'static str {
		"tuberwaber"
	}

	pub const fn path(self) -> AssetPath {
		BODY_TUBERWABER
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TuberwaberHeadMesh {
	#[default]
	Tuberwaber,
}

impl TuberwaberHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Tuberwaber];

	pub const fn label(self) -> &'static str {
		"tuberwaber"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_TUBERWABER
	}
}
