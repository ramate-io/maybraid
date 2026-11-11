//! [`lod::VisualLodScene`] for packed orchard hosts.

use lod::lod_ref::LodRef;
use lod::{LodSceneLevel, VisualLodPrimitive, VisualLodScene, VisualSceneChunk};

use crate::host::ChicoGroveHost;
use crate::packed::instances::visual_chunks_for_orchard;
use crate::packed::mode::PackMode;

impl VisualLodScene for ChicoGroveHost {
	fn visual_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> VisualSceneChunk {
		let _ = lod_ref;
		if !PackMode::current().packs_orchard() {
			return VisualSceneChunk::primitive(VisualLodPrimitive::stub(level));
		}
		let Some(orchard) = self.tile.as_orchard() else {
			return VisualSceneChunk::primitive(VisualLodPrimitive::stub(level));
		};
		visual_chunks_for_orchard(orchard, level)
	}
}

impl ChicoGroveHost {
	pub fn is_orchard(&self) -> bool {
		self.tile.as_orchard().is_some()
	}

	pub fn packs_visuals(&self) -> bool {
		PackMode::current().packs_orchard() && self.is_orchard()
	}
}
