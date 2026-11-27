//! Seed-13 non-contact Cartoon Landing, authored offline.
use crate::{EffectLayer, EffectLayerGroup, EffectRecipe};
use bevy::prelude::*;
#[derive(Component, Clone, Debug, EffectRecipe)]
#[effect(name="cartoon-dust",layers=Self::layers)]
pub struct CartoonDust;
impl CartoonDust {
	pub const CLIP: &'static str = "vfx/dust/cartoon_landing.mesh_clip.json";
	fn layers(&self) -> EffectLayerGroup {
		EffectLayer::mesh_clip_path(Self::CLIP)
			.with_name("opaque landing lobes")
			.into_group()
	}
}
