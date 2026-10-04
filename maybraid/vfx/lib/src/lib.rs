//! Real-time VFX: particles, flipbooks, and short-lived composite hosts.
//!
//! Domain shaders and world materials stay in their crates. This lib is the
//! effect graph — concepts land here before they are wired into firearms or world.
//!
//! First delivery ([#945](https://github.com/ramate-io/maybraid/issues/945)):
//! reusable flash / fireball / smoke / sparks layers and the composed
//! `firey_explosion`.

pub mod assets;
pub mod composition;
pub mod effects;
pub mod flipbook;
pub mod layers;
pub mod library;
pub mod spawn;

pub use composition::{
	EffectDefinition, EffectLayer, EffectPart, LightPulse, ParticlePart, VfxFlipbooks, FIREBALL,
	FIREY_EXPLOSION, FLASH, SMOKE, SPARKS,
};
pub use library::{canonicalize_effect_name, VfxLibrary};
pub use spawn::{spawn_vfx, SpawnVfxExt, VfxInstance, VfxSpawn};

use bevy::prelude::*;
use bevy_hanabi::HanabiPlugin;

use crate::library::setup_vfx_library;
use crate::spawn::vfx_lifecycle_plugin;

/// Registers Hanabi, compiles shared definitions once, and ticks instance cleanup.
pub struct VfxPlugin;

impl Plugin for VfxPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<HanabiPlugin>() {
			app.add_plugins(HanabiPlugin);
		}
		app.add_systems(Startup, setup_vfx_library);
		vfx_lifecycle_plugin(app);
	}
}
