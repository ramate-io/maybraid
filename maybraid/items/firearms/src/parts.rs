//! Per-slot kit catalogs. Body is required; every other slot includes `None`.

use clap::ValueEnum;
use firearms_components::assets::guns;
use firearms_components::{AssetPath, PartNode};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BodyMesh {
	#[default]
	Bullpup,
	Silopup,
	Reltor,
	Samsonist,
	Snailer,
}

impl BodyMesh {
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

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Bullpup => guns::BULLPUP_BODY,
			Self::Silopup => guns::SILOPUP_BODY,
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
	Keelripe,
	Paddle,
	Reltor,
}

impl TriggerBoxMesh {
	pub const VALUES: &'static [Self] = &[Self::None, Self::Keelripe, Self::Paddle, Self::Reltor];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Keelripe => "keelripe",
			Self::Paddle => "paddle",
			Self::Reltor => "reltor",
		}
	}

	pub const fn path(self) -> Option<AssetPath> {
		match self {
			Self::None => None,
			Self::Keelripe => Some(guns::KEELRIPE_BOX),
			Self::Paddle => Some(guns::PADDLE_BOX),
			Self::Reltor => Some(guns::RELTOR_BOX),
		}
	}

	pub fn node(self) -> Option<PartNode> {
		self.path().map(|path| PartNode::trigger_box(self.label(), path.as_str()))
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum GripMesh {
	#[default]
	None,
	BumpHandle,
}

impl GripMesh {
	pub const VALUES: &'static [Self] = &[Self::None, Self::BumpHandle];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::BumpHandle => "bump-handle",
		}
	}

	pub const fn path(self) -> Option<AssetPath> {
		match self {
			Self::None => None,
			Self::BumpHandle => Some(guns::BUMP_HANDLE),
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

/// ADS vertical FOV with no optic (50°). Matches the follow-camera iron-sight default.
pub const IRON_SIGHT_FOV: f32 = 50.0_f32.to_radians();

/// Uniform rest scale for a 1 m authored sight cube on `sight_camera_socket`.
const HOLORAND_REST_SCALE: f32 = 0.10;
const LESKOP_REST_SCALE: f32 = 0.16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SightMesh {
	#[default]
	None,
	Holorand,
	Leskop,
}

impl SightMesh {
	pub const VALUES: &'static [Self] = &[Self::None, Self::Holorand, Self::Leskop];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Holorand => "holorand",
			Self::Leskop => "leskop",
		}
	}

	pub const fn path(self) -> Option<AssetPath> {
		match self {
			Self::None => None,
			Self::Holorand => Some(guns::HOLORAND_SIGHT),
			Self::Leskop => Some(guns::LESKOP_SIGHT),
		}
	}

	/// Magnification relative to [`IRON_SIGHT_FOV`]. Iron sights are 1×.
	pub const fn zoom_min(self) -> f32 {
		match self {
			Self::None => 1.0,
			Self::Holorand => 1.0,
			Self::Leskop => 3.0,
		}
	}

	pub const fn zoom_max(self) -> f32 {
		match self {
			Self::None => 1.0,
			Self::Holorand => 3.0,
			Self::Leskop => 5.0,
		}
	}

	pub fn rest_scale(self) -> f32 {
		match self {
			Self::None => 1.0,
			Self::Holorand => HOLORAND_REST_SCALE,
			Self::Leskop => LESKOP_REST_SCALE,
		}
	}

	/// Vertical FOV at `zoom`× relative to iron sights.
	pub fn fov_at_zoom(zoom: f32) -> f32 {
		fov_at_zoom(IRON_SIGHT_FOV, zoom)
	}

	pub fn ads_fov_range(self) -> (f32, f32) {
		(Self::fov_at_zoom(self.zoom_min()), Self::fov_at_zoom(self.zoom_max()))
	}

	pub fn node(self) -> Option<PartNode> {
		self.path()
			.map(|path| PartNode::sight(self.label(), path.as_str(), self.rest_scale()))
	}
}

pub fn fov_at_zoom(base_fov: f32, zoom: f32) -> f32 {
	2.0 * ((base_fov * 0.5).tan() / zoom.max(1e-4)).atan()
}

/// Kit socket bones that can be lengthened / thickened in the playground.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KitBone {
	#[default]
	Body,
	Barrel,
	TriggerBox,
	Grip,
	Stock,
}

impl KitBone {
	pub const VALUES: &'static [Self] =
		&[Self::Body, Self::Barrel, Self::TriggerBox, Self::Grip, Self::Stock];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Body => "body",
			Self::Barrel => "barrel",
			Self::TriggerBox => "trigger-box",
			Self::Grip => "grip",
			Self::Stock => "stock",
		}
	}

	pub const fn bone_name(self) -> &'static str {
		match self {
			Self::Body => "body",
			Self::Barrel => "barrel",
			Self::TriggerBox => "trigger_box",
			Self::Grip => "grip",
			Self::Stock => "stock",
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn iron_sights_keep_current_ads_fov() {
		let (min, max) = SightMesh::None.ads_fov_range();
		assert!((min - IRON_SIGHT_FOV).abs() < 1e-5);
		assert!((max - IRON_SIGHT_FOV).abs() < 1e-5);
	}

	#[test]
	fn holorand_spans_iron_to_three_times() {
		let (min, max) = SightMesh::Holorand.ads_fov_range();
		assert!((min - IRON_SIGHT_FOV).abs() < 1e-5);
		assert!((max - SightMesh::fov_at_zoom(3.0)).abs() < 1e-5);
		assert!(max < min);
	}

	#[test]
	fn leskop_spans_three_to_five_times() {
		let (min, max) = SightMesh::Leskop.ads_fov_range();
		assert!((min - SightMesh::fov_at_zoom(3.0)).abs() < 1e-5);
		assert!((max - SightMesh::fov_at_zoom(5.0)).abs() < 1e-5);
		assert!(max < min);
	}
}
