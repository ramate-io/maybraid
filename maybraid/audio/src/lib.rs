//! Shared oddio + CPAL backend. Gameplay plays clips; this crate owns the device.

pub mod ambient;
pub mod asset;
pub mod backend;
pub mod flinch;
pub mod mixer;
pub mod movement;
pub mod spatial;

use bevy::prelude::*;

pub use ambient::{
	AmbientClip, AmbientPick, AmbientSounds, BIRDSONG_SPATIAL_RADIUS, BIRDSONG_SPATIAL_SCALE,
	BIRDSONG_VOLUME, HERD_GRUNT_SPATIAL_RADIUS, HERD_GRUNT_SPATIAL_SCALE, HERD_GRUNT_VOLUME,
	HERD_WAIL_SPATIAL_RADIUS, HERD_WAIL_SPATIAL_SCALE, HERD_WAIL_VOLUME,
};
pub use asset::{decode_wav_mono, AudioClip, AudioClipError};
pub use backend::{preferred_sample_format, Audio};
pub use flinch::{
	pick_variant, FlinchProfile, FlinchSounds, FlinchState, GruntStyle, GRUNT_SPATIAL_RADIUS,
	GRUNT_SPATIAL_SCALE, GRUNT_VOLUME,
};
pub use mixer::{AudioBus, Mixer};
pub use movement::{
	MovementClip, MovementSounds, MovementState, BREATH_SPATIAL_RADIUS, BREATH_SPATIAL_SCALE,
	BREATH_VOLUME, CHANGE_ITEM_SPATIAL_RADIUS, CHANGE_ITEM_SPATIAL_SCALE, CHANGE_ITEM_VOLUME,
	FOOTSTEP_SPATIAL_RADIUS, FOOTSTEP_SPATIAL_SCALE, FOOTSTEP_VOLUME, SPRINT_EXHALE_SECS,
	SPRINT_INHALE_SECS,
};
pub use spatial::{
	amplitude_to_db, AudioVelocity, SpatialEmitter, SpatialOneShot, SpatialVoice, FIRE_LISTENER_GAP,
};

/// Registers [`AudioClip`], starts CPAL, and syncs listener / voices.
pub struct AudioPlugin;

/// Gameplay that writes [`AudioVelocity`] should run before [`AudioSystems::Sync`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AudioSystems {
	Mix,
	Sync,
}

impl Plugin for AudioPlugin {
	fn build(&self, app: &mut App) {
		app.init_asset::<AudioClip>()
			.register_asset_loader(asset::AudioClipLoader)
			.init_resource::<Mixer>()
			.configure_sets(
				PostUpdate,
				(AudioSystems::Mix, AudioSystems::Sync)
					.chain()
					.after(TransformSystems::Propagate),
			)
			.add_systems(
				Startup,
				(
					setup_audio,
					flinch::setup_flinch_sounds,
					movement::setup_movement_sounds,
					ambient::setup_ambient_sounds,
				),
			)
			.add_systems(PostUpdate, mixer::tick_mixer.in_set(AudioSystems::Mix))
			.add_systems(
				PostUpdate,
				(
					spatial::sync_listener,
					spatial::sync_voices,
					spatial::despawn_finished_voices,
					spatial::flush_pending,
				)
					.chain()
					.in_set(AudioSystems::Sync),
			);
	}
}

fn setup_audio(mut commands: Commands) {
	if let Some(audio) = Audio::start() {
		commands.insert_resource(audio);
	}
}

/// Bevy listener marker. Oddio uses its own head radius; the gap only
/// constructs [`SpatialListener`].
pub fn listener() -> SpatialListener {
	SpatialListener::new(FIRE_LISTENER_GAP)
}
