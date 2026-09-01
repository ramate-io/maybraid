//! Per-slot kit catalogs. Body is required; every other slot includes `None`.

use clap::ValueEnum;
use firearms_components::assets::guns;
use firearms_components::{AssetPath, PartNode};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BodyMesh {
	#[default]
	Bullpup,
	Silopup,
	Keelripe,
	Reltor,
	Samsonist,
	Snailer,
}

impl BodyMesh {
	pub const VALUES: &'static [Self] = &[
		Self::Bullpup,
		Self::Silopup,
		Self::Keelripe,
		Self::Reltor,
		Self::Samsonist,
		Self::Snailer,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Bullpup => "bullpup",
			Self::Silopup => "silopup",
			Self::Keelripe => "keelripe",
			Self::Reltor => "reltor",
			Self::Samsonist => "samsonist",
			Self::Snailer => "snailer",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Bullpup => guns::BULLPUP_BODY,
			Self::Silopup => guns::SILOPUP_BODY,
			Self::Keelripe => guns::KEELRIPE_BODY,
			Self::Reltor => guns::RELTOR_BODY,
			Self::Samsonist => guns::SAMSONIST_BODY,
			Self::Snailer => guns::SNAILER_BODY,
		}
	}

	pub fn node(self) -> PartNode {
		PartNode::body(self.label(), self.path().as_str())
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BarrelMesh {
	#[default]
	None,
	Bullpup,
	Laznard,
}

impl BarrelMesh {
	pub const VALUES: &'static [Self] = &[Self::None, Self::Bullpup, Self::Laznard];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Bullpup => "bullpup",
			Self::Laznard => "laznard",
		}
	}

	pub const fn path(self) -> Option<AssetPath> {
		match self {
			Self::None => None,
			Self::Bullpup => Some(guns::BULLPUP_BARREL),
			Self::Laznard => Some(guns::LAZNARD_BARREL),
		}
	}

	pub fn node(self) -> Option<PartNode> {
		self.path().map(|path| PartNode::barrel(self.label(), path.as_str()))
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TriggerBoxMesh {
	#[default]
	None,
}

impl TriggerBoxMesh {
	pub const VALUES: &'static [Self] = &[Self::None];

	pub const fn label(self) -> &'static str {
		"none"
	}

	pub const fn path(self) -> Option<AssetPath> {
		None
	}

	pub fn node(self) -> Option<PartNode> {
		self.path().map(|path| PartNode::trigger_box(self.label(), path.as_str()))
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum GripMesh {
	#[default]
	None,
	Bullpup,
}

impl GripMesh {
	pub const VALUES: &'static [Self] = &[Self::None, Self::Bullpup];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Bullpup => "bullpup",
		}
	}

	pub const fn path(self) -> Option<AssetPath> {
		match self {
			Self::None => None,
			Self::Bullpup => Some(guns::BULLPUP_GRIP),
		}
	}

	pub fn node(self) -> Option<PartNode> {
		self.path().map(|path| PartNode::grip(self.label(), path.as_str()))
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum StockMesh {
	#[default]
	None,
}

impl StockMesh {
	pub const VALUES: &'static [Self] = &[Self::None];

	pub const fn label(self) -> &'static str {
		"none"
	}

	pub const fn path(self) -> Option<AssetPath> {
		None
	}

	pub fn node(self) -> Option<PartNode> {
		self.path().map(|path| PartNode::stock(self.label(), path.as_str()))
	}
}
