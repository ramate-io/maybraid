//! Portable metadata and shared KTX2 dependencies for authored VFX.
//!
//! Paths resolve relative to the manifest, including within embedded asset sources.
//! Images remain ordinary AssetServer assets: instances clone handles, never pixels.

mod loaders;
mod manifest;

pub use manifest::{
	AlphaConvention, Bounds3, EndBehavior, FlipbookManifest, FlipbookView, VolumeFormat,
	VolumeManifest,
};

use bevy::prelude::*;
use thiserror::Error;

/// One atlas layer per captured direction; frames occupy the same grid in every layer.
#[derive(Asset, TypePath, Clone, Debug)]
pub struct PackagedFlipbook {
	pub manifest: FlipbookManifest,
	#[dependency]
	pub image: Handle<Image>,
}

/// An x-fastest R16Float 3D density texture, plus its authored coordinate transform.
#[derive(Asset, TypePath, Clone, Debug)]
pub struct StaticVolume {
	pub manifest: VolumeManifest,
	#[dependency]
	pub image: Handle<Image>,
}

#[derive(Debug, Error)]
pub enum PackagedAssetError {
	#[error("cannot read VFX manifest: {0}")]
	Io(#[from] std::io::Error),
	#[error("invalid VFX manifest JSON: {0}")]
	Json(#[from] serde_json::Error),
	#[error("invalid VFX asset: {0}")]
	Invalid(String),
	#[error("cannot read VFX image dependency: {0}")]
	Dependency(#[from] bevy::asset::ReadAssetBytesError),
}

impl PackagedAssetError {
	pub(crate) fn invalid(message: impl Into<String>) -> Self {
		Self::Invalid(message.into())
	}
}

/// Register alongside AssetPlugin/ImagePlugin (included by DefaultPlugins).
pub struct PackagedVfxPlugin;

impl Plugin for PackagedVfxPlugin {
	fn build(&self, app: &mut App) {
		app.init_asset::<PackagedFlipbook>()
			.init_asset::<StaticVolume>()
			.init_asset_loader::<loaders::FlipbookLoader>()
			.init_asset_loader::<loaders::VolumeLoader>();
	}
}

#[cfg(test)]
mod tests;
