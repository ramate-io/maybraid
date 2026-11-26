//! Synthetic sequence for checking depth and lifecycle; not production dust artwork.
use bevy::prelude::*;
use maybraid_vfx::{EffectLayer, EffectLayerGroup, EffectRecipe};

#[derive(Component, Clone, EffectRecipe)]
#[effect(name = "volume-demo", layers = Self::layers)]
pub(crate) struct VolumeDemo;
impl VolumeDemo {
	pub fn layers(&self) -> EffectLayerGroup {
		EffectLayer::volume_flipbook_path("vfx/fixtures/volume_puffs/puffs.volume_flipbook.json")
			.with_name("animated density")
			.into_group()
	}
}
