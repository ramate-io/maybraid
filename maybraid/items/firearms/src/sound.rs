//! Shared fire clip for every [`crate::Weapon`].
//!
//! Bolts and bullets play a one-shot. A laser keeps the same clip looping on the
//! beam entity so the sound dies with the beam.

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy::prelude::*;

use firearms_components::AssetPath;

/// Authored wet-laser bip under `maybraid/assets`.
pub const WET_LASER_FIRE: AssetPath =
	AssetPath::new("sound-effects/weapons__wet_laser__wet_laser_001.wav");

const FIRE_VOLUME: f32 = 0.8;

/// Loaded fire clip. Missing this resource is a silent no-op.
#[derive(Resource, Clone)]
pub struct FirearmFireSounds {
	pub wet_laser: Handle<AudioSource>,
}

impl FirearmFireSounds {
	pub fn load(asset_server: &AssetServer) -> Self {
		Self { wet_laser: asset_server.load(WET_LASER_FIRE.as_str()) }
	}

	pub fn shot_settings() -> PlaybackSettings {
		PlaybackSettings::DESPAWN.with_volume(Volume::Linear(FIRE_VOLUME))
	}

	pub fn laser_settings() -> PlaybackSettings {
		PlaybackSettings::LOOP.with_volume(Volume::Linear(FIRE_VOLUME))
	}

	pub fn play_shot(&self, commands: &mut Commands) {
		commands.spawn((
			Name::new("firearm-fire"),
			AudioPlayer::new(self.wet_laser.clone()),
			Self::shot_settings(),
		));
	}

	pub fn loop_on(&self, commands: &mut Commands, entity: Entity) {
		commands
			.entity(entity)
			.insert((AudioPlayer::new(self.wet_laser.clone()), Self::laser_settings()));
	}
}

impl FromWorld for FirearmFireSounds {
	fn from_world(world: &mut World) -> Self {
		let asset_server = world.resource::<AssetServer>();
		Self::load(asset_server)
	}
}

pub(crate) fn setup_fire_sounds(mut commands: Commands, assets: Res<AssetServer>) {
	commands.insert_resource(FirearmFireSounds::load(&assets));
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::audio::PlaybackMode;
	use std::path::Path;

	#[test]
	fn wet_laser_clip_is_in_assets() {
		let path = Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("../../assets")
			.join(WET_LASER_FIRE.as_str());
		assert!(path.is_file(), "{}", path.display());
	}

	#[test]
	fn ballistic_shot_despawns_and_laser_loops() {
		assert!(matches!(FirearmFireSounds::shot_settings().mode, PlaybackMode::Despawn));
		assert!(matches!(FirearmFireSounds::laser_settings().mode, PlaybackMode::Loop));
	}
}
