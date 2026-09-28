//! Shared fire clip for every [`crate::Weapon`].
//!
//! Bolts and bullets play a one-shot at the muzzle. A laser keeps the same clip
//! looping on a child of the beam so the sound dies with the beam. Both use
//! Bevy spatial audio: stereo pan plus inverse-square falloff from the follow
//! camera's [`SpatialListener`].

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, SpatialScale, Volume};
use bevy::prelude::*;

use firearms_components::AssetPath;

/// Authored wet-laser bip under `maybraid/assets`.
pub const WET_LASER_FIRE: AssetPath =
	AssetPath::new("sound-effects/weapons__wet_laser__wet_laser_002.wav");

const FIRE_VOLUME: f32 = 0.8;
/// Compresses world meters so a 3.6 m follow boom stays at full volume and a
/// 20 m flanker is about a quarter. Rodio clamps `1 / dist²` at 1 scaled unit.
pub const FIRE_SPATIAL_SCALE: SpatialScale = SpatialScale::new(0.1);
/// Virtual ear spacing. Slightly wider than a human head so left/right still
/// reads at follow-camera range. Panning uses this gap, not the spatial scale.
pub const FIRE_LISTENER_GAP: f32 = 0.4;
/// Laser mesh is a cylinder centered on the beam. Parent scale.y is length, so
/// local −0.5 Y sits on the muzzle.
pub const LASER_MUZZLE_LOCAL: Vec3 = Vec3::new(0.0, -0.5, 0.0);

/// Loaded fire clip. Missing this resource is a silent no-op.
#[derive(Resource, Clone)]
pub struct FirearmFireSounds {
	pub wet_laser: Handle<AudioSource>,
}

impl FirearmFireSounds {
	pub fn load(asset_server: &AssetServer) -> Self {
		Self { wet_laser: asset_server.load(WET_LASER_FIRE.as_str()) }
	}

	pub fn listener() -> SpatialListener {
		SpatialListener::new(FIRE_LISTENER_GAP)
	}

	pub fn shot_settings() -> PlaybackSettings {
		PlaybackSettings::DESPAWN
			.with_volume(Volume::Linear(FIRE_VOLUME))
			.with_spatial(true)
			.with_spatial_scale(FIRE_SPATIAL_SCALE)
	}

	pub fn laser_settings() -> PlaybackSettings {
		PlaybackSettings::LOOP
			.with_volume(Volume::Linear(FIRE_VOLUME))
			.with_spatial(true)
			.with_spatial_scale(FIRE_SPATIAL_SCALE)
	}

	pub fn play_at(
		&self,
		commands: &mut Commands,
		parent: Entity,
		local: Vec3,
		settings: PlaybackSettings,
	) {
		commands.spawn((
			Name::new("firearm-fire"),
			ChildOf(parent),
			Transform::from_translation(local),
			AudioPlayer::new(self.wet_laser.clone()),
			settings,
		));
	}

	pub fn play_shot(&self, commands: &mut Commands, barrel: Entity, muzzle_local: Vec3) {
		self.play_at(commands, barrel, muzzle_local, Self::shot_settings());
	}

	pub fn loop_on(&self, commands: &mut Commands, laser: Entity) {
		self.play_at(commands, laser, LASER_MUZZLE_LOCAL, Self::laser_settings());
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

/// Playgrounds that do not spawn a follow-camera listener still get one on the
/// lone `Camera3d`. Skips when a listener already exists or when more than one
/// 3D camera is around.
pub(crate) fn ensure_camera_spatial_listener(
	mut commands: Commands,
	existing: Query<Entity, With<SpatialListener>>,
	cameras: Query<Entity, (With<Camera3d>, Without<SpatialListener>)>,
) {
	if !existing.is_empty() {
		return;
	}
	let mut cameras = cameras.iter();
	let Some(camera) = cameras.next() else {
		return;
	};
	if cameras.next().is_some() {
		return;
	}
	commands.entity(camera).insert(FirearmFireSounds::listener());
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

	#[test]
	fn fire_clips_are_spatial() {
		let shot = FirearmFireSounds::shot_settings();
		let laser = FirearmFireSounds::laser_settings();
		assert!(shot.spatial);
		assert!(laser.spatial);
		assert_eq!(shot.spatial_scale.map(|scale| scale.0), Some(FIRE_SPATIAL_SCALE.0));
		assert_eq!(laser.spatial_scale.map(|scale| scale.0), Some(FIRE_SPATIAL_SCALE.0));
	}

	#[test]
	fn laser_child_stays_on_the_muzzle() {
		for len in [0.02_f32, 4.0, 22.0] {
			let parent_y = 1.0 + len * 0.5;
			let child_world_y = parent_y + LASER_MUZZLE_LOCAL.y * len;
			assert!((child_world_y - 1.0).abs() < 1e-5, "len {len} -> {child_world_y}");
		}
	}

	#[test]
	fn listener_gap_is_a_wide_head() {
		let listener = FirearmFireSounds::listener();
		assert!(
			(listener.right_ear_offset.x - listener.left_ear_offset.x - FIRE_LISTENER_GAP).abs()
				< 1e-5
		);
	}

	#[test]
	fn ensure_listener_lands_on_a_lone_camera() {
		let mut app = App::new();
		app.add_systems(Update, ensure_camera_spatial_listener);
		let camera = app.world_mut().spawn(Camera3d::default()).id();
		app.update();
		assert!(app.world().get::<SpatialListener>(camera).is_some());
	}

	#[test]
	fn ensure_listener_does_not_add_a_second_pair_of_ears() {
		let mut app = App::new();
		app.add_systems(Update, ensure_camera_spatial_listener);
		let camera = app.world_mut().spawn(Camera3d::default()).id();
		let existing = app.world_mut().spawn(FirearmFireSounds::listener()).id();
		app.update();
		assert!(app.world().get::<SpatialListener>(camera).is_none());
		assert!(app.world().get::<SpatialListener>(existing).is_some());
	}
}
