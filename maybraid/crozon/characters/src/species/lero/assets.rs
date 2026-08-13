//! Lero asset catalog.

use clap::ValueEnum;

use crate::assets::AssetPath;

const HEAD_ORTHO_TEE: AssetPath = AssetPath::new("characters/heads/ortho_tee_head.glb");
const SNOUT_LERODON: AssetPath = AssetPath::new("characters/snouts/lerodon_snout.glb");
const SNOUT_ROBREK: AssetPath = AssetPath::new("characters/snouts/robrek_snout.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum LeroHeadMesh {
	#[default]
	OrthoTee,
}

impl LeroHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::OrthoTee];

	pub const fn label(self) -> &'static str {
		"ortho-tee"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_ORTHO_TEE
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum LeroMouthMesh {
	#[default]
	Lerodon,
	Robrek,
}

impl LeroMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Lerodon, Self::Robrek];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Lerodon => "lerodon",
			Self::Robrek => "robrek",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Lerodon => SNOUT_LERODON,
			Self::Robrek => SNOUT_ROBREK,
		}
	}
}
