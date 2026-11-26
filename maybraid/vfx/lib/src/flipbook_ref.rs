//! Deferred packaged flipbook identity, following the path convention of `SceneRef`.

use bevy::prelude::*;

use crate::parts::packaged::PackagedFlipbook;

/// An asset path, independent of any runtime asset server or preloaded handle.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FlipbookRef {
	pub path: String,
}

impl FlipbookRef {
	pub fn path(path: impl Into<String>) -> Self {
		Self { path: path.into() }
	}

	/// Bevy deduplicates typed loads of the same path and tracks image dependencies.
	pub fn load(&self, server: &AssetServer) -> Handle<PackagedFlipbook> {
		server.load(self.path.clone())
	}
}

impl From<&str> for FlipbookRef {
	fn from(path: &str) -> Self {
		Self::path(path)
	}
}

impl From<String> for FlipbookRef {
	fn from(path: String) -> Self {
		Self::path(path)
	}
}
