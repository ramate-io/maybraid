//! Deferred animated volume identity, following the path convention of `SceneRef`.

use bevy::prelude::*;

use crate::parts::packaged::PackagedVolumeFlipbook;

/// An asset path, independent of any runtime asset server or preloaded handle.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VolumeFlipbookRef {
	pub path: String,
}

impl VolumeFlipbookRef {
	pub fn path(path: impl Into<String>) -> Self {
		Self { path: path.into() }
	}

	/// Bevy deduplicates typed loads of the same path and tracks image dependencies.
	pub fn load(&self, server: &AssetServer) -> Handle<PackagedVolumeFlipbook> {
		server.load(self.path.clone())
	}
}

impl From<&str> for VolumeFlipbookRef {
	fn from(path: &str) -> Self {
		Self::path(path)
	}
}

impl From<String> for VolumeFlipbookRef {
	fn from(path: String) -> Self {
		Self::path(path)
	}
}
