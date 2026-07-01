//! Option traits and thumbnail collection for typed menu primitives.
//!
//! Preferred interaction path: `RenderMenu` implementations handle widget events
//! and emit `CharacterMenuEvent::MenuUpdate` from the renderer crate.
//!
//! Future direction — primitives generic over callbacks into menu context:
//! `Select<OnSelect: FnMut(&mut MenuContext, T), T: ListValues + …>`
//!
//! Escape hatch only — add `MenuInteractive` if `RenderMenu`-only interaction
//! proves insufficient during implementation.

use bevy_math::Vec3;

use crate::{IdentifiedAsset, ThumbnailCamera};

/// Types with a fixed list of selectable variants.
pub trait ListValues: Copy + PartialEq + 'static {
	fn values() -> &'static [Self];
}

/// Stable string id for persistence and renderer keys.
pub trait StringIdentified {
	fn id(&self) -> &'static str;
}

/// Human-readable option label.
pub trait LabelOption {
	fn label(&self) -> &'static str;
}

/// Color swatch option contract.
pub trait SwatchOption: LabelOption {
	fn color_hex(&self) -> &'static str;
}

/// Asset-backed option contract.
pub trait AssetOption: LabelOption {
	fn asset(&self) -> IdentifiedAsset;
}

/// Controls whether asset pickers show thumbnail previews.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AssetThumbnailDisplay {
	#[default]
	None,
	Inline,
	HoverPreview,
}

/// Request to render a cached asset thumbnail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbnailRequest {
	pub path: &'static str,
	pub color: [u8; 4],
	pub camera: ThumbnailCamera,
}

impl ThumbnailRequest {
	pub const fn new(path: &'static str, color: [u8; 4], camera: ThumbnailCamera) -> Self {
		Self { path, color, camera }
	}
}

/// Collect thumbnail prewarm targets from a menu tree.
pub trait ThumbnailSources {
	fn collect_thumbnail_requests(&self, out: &mut Vec<ThumbnailRequest>);
}

pub fn color_key(color: [u8; 4]) -> [u8; 4] {
	color
}

pub fn vec3_key(v: Vec3) -> [u8; 4] {
	[
		(v.x * 255.0) as u8,
		(v.y * 255.0) as u8,
		(v.z * 255.0) as u8,
		0,
	]
}
