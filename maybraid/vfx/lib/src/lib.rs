//! Real-time VFX: particles, flipbooks, mesh lobes, and short-lived composite hosts.
//!
//! Domain shaders and world materials stay in their crates. This lib is the
//! effect graph — concepts land here before they are wired into firearms or world.
//!
//! Burst delivery ([#945](https://github.com/ramate-io/maybraid/issues/945)):
//! reusable flash / fireball / smoke / sparks layers and the composed
//! `fiery_explosion`. Driven delivery ([#1122](https://github.com/ramate-io/maybraid/issues/1122),
//! [#1123](https://github.com/ramate-io/maybraid/issues/1123)): `god_beam` and
//! `sonic_wave` through [`VfxDrive`]. Sonic rings render [`SonicWaveState`];
//! attack binding lives with [#1119](https://github.com/ramate-io/maybraid/issues/1119).

pub mod assets;
pub mod atlas;
pub mod composition;
pub mod drive;
pub mod effects;
pub mod field_material;
pub mod fields;
pub mod flipbook;
pub mod layers;
pub mod library;
pub mod lifecycle;
pub mod lobe_material;
pub mod lobes;
pub mod membership;
pub mod names;
pub mod palette;
pub mod particles;
pub mod seed;
pub mod spawn;

pub use assets::{FlipbookAsset, VfxFlipbooks};
pub use composition::{
	DrivenFieldVisual, EffectDefinition, EffectKind, EffectLayer, EffectPart, GodBeamVisual,
	LightPulse, LobeKind, LobeSpec, MeshPart, ParticlePart, ParticleShade, ScaleBounds,
	SonicWaveVisual, DEFAULT_SCALE_MAX, DEFAULT_SCALE_MIN,
};
pub use drive::{
	FieldEnvelope, SonicWaveState, VfxDrive, VfxWaveStates, GOD_BEAM_ACTIVE, GOD_BEAM_HEIGHT,
	GOD_BEAM_WIDTH, SONIC_WAVE_INTERVAL, SONIC_WAVE_RADIUS, SONIC_WAVE_RANGE, SONIC_WAVE_SPEED,
	SONIC_WAVE_STAGGER, SONIC_WAVE_THICKNESS,
};
pub use field_material::{FieldKind, FieldMaterial, FieldMaterialPlugin};
pub use library::{canonicalize_effect_name, VfxLibrary};
pub use lobe_material::{LobeMaterial, LobeMaterialPlugin};
pub use names::{FIERY_EXPLOSION, FIREBALL, FLASH, GOD_BEAM, SMOKE, SONIC_WAVE, SPARKS};
pub use palette::{ExplosionPalette, FieldPalette};
pub use spawn::{
	apply_cancel, cancel_vfx, despawn_vfx, spawn_vfx, SpawnVfxExt, VfxCancel, VfxInstance,
	VfxSpawn, MAX_PLAYBACK, MIN_PLAYBACK,
};

use bevy::prelude::*;
use bevy_hanabi::HanabiPlugin;

use crate::library::setup_vfx_library;
use crate::lifecycle::vfx_lifecycle_plugin;

/// Registers Hanabi, the lobe material, compiled definitions, and instance cleanup.
pub struct VfxPlugin;

impl Plugin for VfxPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<HanabiPlugin>() {
			app.add_plugins(HanabiPlugin);
		}
		if !app.is_plugin_added::<LobeMaterialPlugin>() {
			app.add_plugins(LobeMaterialPlugin);
		}
		if !app.is_plugin_added::<FieldMaterialPlugin>() {
			app.add_plugins(FieldMaterialPlugin);
		}
		app.add_systems(Startup, setup_vfx_library);
		vfx_lifecycle_plugin(app);
	}
}
