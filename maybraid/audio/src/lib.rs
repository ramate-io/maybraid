//! Shared oddio + CPAL backend. Gameplay plays clips; this crate owns the device.

pub mod asset;
pub mod backend;
pub mod flinch;
pub mod mixer;
pub mod spatial;

use bevy::prelude::*;

pub use asset::{AudioClip, AudioClipError, decode_wav_mono};
pub use backend::{Audio, preferred_sample_format};
pub use flinch::{
	FlinchProfile, FlinchSounds, FlinchState, GRUNT_SPATIAL_RADIUS, GRUNT_SPATIAL_SCALE,
	GRUNT_VOLUME, GruntStyle, pick_variant,
};
pub use mixer::{AudioBus, Mixer};
pub use spatial::{
	AudioVelocity, FIRE_LISTENER_GAP, SpatialEmitter, SpatialOneShot, SpatialVoice, amplitude_to_db,
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
			.add_systems(Startup, (setup_audio, flinch::setup_flinch_sounds))
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
