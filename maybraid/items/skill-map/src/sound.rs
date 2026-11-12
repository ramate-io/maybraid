//! Overlay activate SFX. Plays on the expand edge, not while RB is held.

use bevy::prelude::*;
use maybraid_audio::{
	Audio, AudioBus, AudioClip, AudioPlugin, AudioSystems, Mixer, SpatialOneShot,
	CHANGE_ITEM_SPATIAL_RADIUS,
};

use crate::overlay::SkillMapOverlayCue;
use crate::user::SkillMapUser;

/// Reuses the authored change-item foley (inventory HUD / item attention).
pub const OVERLAY_ACTIVATE_SOUND: &str = "sound-effects/character/movement/change_item_001.wav";
pub const OVERLAY_ACTIVATE_SOUND_NAME: &str = "skill-map-overlay-activate";
pub const OVERLAY_ACTIVATE_VOLUME: f32 = 0.7;
pub const OVERLAY_ACTIVATE_SPATIAL_RADIUS: f32 = CHANGE_ITEM_SPATIAL_RADIUS;

/// Loaded overlay clip. Missing this resource is a silent no-op.
#[derive(Resource, Debug)]
pub struct SkillMapOverlaySounds {
	clip: Handle<AudioClip>,
	activations: u32,
}

impl SkillMapOverlaySounds {
	pub fn load(assets: &AssetServer) -> Self {
		Self { clip: assets.load(OVERLAY_ACTIVATE_SOUND), activations: 0 }
	}

	pub fn from_clip(clip: Handle<AudioClip>) -> Self {
		Self { clip, activations: 0 }
	}

	pub fn activations(&self) -> u32 {
		self.activations
	}

	pub fn play(
		&mut self,
		commands: &mut Commands,
		clips: Option<&Assets<AudioClip>>,
		audio: Option<&Audio>,
		mixer: Option<&mut Mixer>,
		listener: Option<&GlobalTransform>,
		point: Vec3,
	) {
		self.activations += 1;
		let (Some(audio), Some(clips), Some(mixer), Some(listener)) =
			(audio, clips, mixer, listener)
		else {
			return;
		};
		audio.play_or_queue(
			commands,
			&self.clip,
			clips,
			SpatialOneShot::at(point)
				.radius(OVERLAY_ACTIVATE_SPATIAL_RADIUS)
				.gain(OVERLAY_ACTIVATE_VOLUME)
				.bus(AudioBus::Ui)
				.duck(false),
			mixer,
			listener,
			OVERLAY_ACTIVATE_SOUND_NAME,
		);
	}
}

pub(crate) fn setup_overlay_sounds(mut commands: Commands, assets: Res<AssetServer>) {
	commands.insert_resource(SkillMapOverlaySounds::load(&assets));
}

pub fn play_overlay_activate_sounds(
	mut commands: Commands,
	mut cues: MessageReader<SkillMapOverlayCue>,
	sounds: Option<ResMut<SkillMapOverlaySounds>>,
	clips: Option<Res<Assets<AudioClip>>>,
	audio: Option<Res<Audio>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
	users: Query<&GlobalTransform, With<SkillMapUser>>,
) {
	let Some(mut sounds) = sounds else {
		return;
	};
	let listener = listeners.iter().next();
	let audio = audio.as_deref();
	let clips = clips.as_deref();
	let mut mixer = mixer;
	for cue in cues.read() {
		let point = users
			.get(cue.user)
			.ok()
			.map(GlobalTransform::translation)
			.or_else(|| listener.map(GlobalTransform::translation))
			.unwrap_or(Vec3::ZERO);
		sounds.play(&mut commands, clips, audio, mixer.as_deref_mut(), listener, point);
	}
}

pub(crate) fn configure_overlay_audio(app: &mut App) {
	if !app.is_plugin_added::<AudioPlugin>() {
		app.add_plugins(AudioPlugin);
	}
	app.add_systems(Startup, setup_overlay_sounds)
		.add_systems(PostUpdate, play_overlay_activate_sounds.before(AudioSystems::Sync));
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_audio::MovementClip;
	use std::path::Path;

	#[test]
	fn activate_clip_is_the_authored_change_item() {
		assert_eq!(OVERLAY_ACTIVATE_SOUND, MovementClip::ChangeItem.path(0));
		let file = Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("../../assets")
			.join(OVERLAY_ACTIVATE_SOUND);
		assert!(file.is_file(), "{}", file.display());
		let bytes = std::fs::read(&file).unwrap_or_else(|_| panic!("{}", file.display()));
		assert!(
			maybraid_audio::decode_wav_mono(&bytes).is_ok(),
			"{OVERLAY_ACTIVATE_SOUND} must be authored mono"
		);
	}
}
