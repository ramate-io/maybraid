//! Firearm shot, beam, and in-flight clips.
//!
//! Bolts and bullets play a muzzle one-shot, then a looping fizz child follows
//! the flight until it despawns. A hit plays a world-space impact one-shot.
//! A laser loops the fire clip on the beam. Bevy spatial audio supplies stereo
//! pan. Flight and impact loudness use a listener-distance curve, not rodio's
//! inverse-square.

use bevy::audio::{
	AudioPlayer, AudioSinkPlayback, AudioSource, PlaybackSettings, SpatialAudioSink, SpatialScale,
	Volume,
};
use bevy::prelude::*;

use firearms_components::AssetPath;

/// Authored wet-laser bip under `maybraid/assets`.
pub const WET_LASER_FIRE: AssetPath =
	AssetPath::new("sound-effects/weapons__wet_laser__wet_laser_001.wav");

/// Short seamless flight loop under `maybraid/assets`.
pub const LAZER_FIZZ: AssetPath = AssetPath::new("sound-effects/lazer_fizz_001.wav");

/// One-shot when a bolt or bullet first crosses a collider.
pub const LAZER_IMPACT_FIZZ: AssetPath = AssetPath::new("sound-effects/lazer_impact_fizz_001.wav");

const FIRE_VOLUME: f32 = 1.4;
const FIZZ_VOLUME: f32 = 0.55;
const IMPACT_VOLUME: f32 = 0.7;
/// Compresses world meters so a 3.6 m follow boom stays at full volume and a
/// 20 m flanker is about a quarter. Rodio clamps `1 / dist²` at 1 scaled unit.
pub const FIRE_SPATIAL_SCALE: SpatialScale = SpatialScale::new(0.1);
/// Keeps rodio's `1 / dist²` clamped through [`FlightAttenuation::silent`] so
/// the authored meter curve owns loudness. Pan is scale-invariant.
pub const FIZZ_SPATIAL_SCALE: SpatialScale = SpatialScale::new(0.5);
/// Same remap for the impact one-shot. Independent of the flight loop so a
/// hit can fall off faster or carry farther without retuning the fizz.
pub const IMPACT_SPATIAL_SCALE: SpatialScale = SpatialScale::new(0.25);
/// Virtual ear spacing. Slightly wider than a human head so left/right still
/// reads at follow-camera range. Panning uses this gap, not the spatial scale.
pub const FIRE_LISTENER_GAP: f32 = 0.4;
/// Laser mesh is a cylinder centered on the beam. Parent scale.y is length, so
/// local −0.5 Y sits on the muzzle.
pub const LASER_MUZZLE_LOCAL: Vec3 = Vec3::new(0.0, -0.5, 0.0);

/// Looping fizz emitter parented to a bolt or bullet.
#[derive(Component, Clone, Copy, Debug)]
pub struct FlightFizz;

/// Listener-distance gain for an in-flight emitter. Not a function of clip time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlightAttenuation {
	pub near: f32,
	pub mid: f32,
	pub far: f32,
	pub silent: f32,
	pub mid_gain: f32,
	pub far_gain: f32,
}

impl FlightAttenuation {
	/// Full to 5 m, quieter at 20 m, very quiet at 50 m, inaudible at 100 m.
	pub const FIREARM: Self =
		Self { near: 5.0, mid: 20.0, far: 50.0, silent: 100.0, mid_gain: 0.35, far_gain: 0.08 };

	pub fn gain(self, distance: f32) -> f32 {
		let distance = distance.max(0.0);
		if distance <= self.near {
			1.0
		} else if distance <= self.mid {
			Self::lerp(1.0, self.mid_gain, Self::unit(self.near, self.mid, distance))
		} else if distance <= self.far {
			Self::lerp(self.mid_gain, self.far_gain, Self::unit(self.mid, self.far, distance))
		} else if distance <= self.silent {
			Self::lerp(self.far_gain, 0.0, Self::unit(self.far, self.silent, distance))
		} else {
			0.0
		}
	}

	pub fn volume(self, distance: f32) -> Volume {
		self.volume_at(FIZZ_VOLUME, distance)
	}

	pub fn volume_at(self, peak: f32, distance: f32) -> Volume {
		Volume::Linear(peak * self.gain(distance))
	}

	fn unit(start: f32, end: f32, value: f32) -> f32 {
		let span = end - start;
		if span.abs() < 1e-6 {
			0.0
		} else {
			((value - start) / span).clamp(0.0, 1.0)
		}
	}

	fn lerp(start: f32, end: f32, t: f32) -> f32 {
		start + (end - start) * t
	}
}

/// Loaded fire and flight clips. Missing this resource is a silent no-op.
#[derive(Resource, Clone)]
pub struct FirearmFireSounds {
	pub wet_laser: Handle<AudioSource>,
	pub lazer_fizz: Handle<AudioSource>,
	pub lazer_impact: Handle<AudioSource>,
}

impl FirearmFireSounds {
	pub fn load(asset_server: &AssetServer) -> Self {
		Self {
			wet_laser: asset_server.load(WET_LASER_FIRE.as_str()),
			lazer_fizz: asset_server.load(LAZER_FIZZ.as_str()),
			lazer_impact: asset_server.load(LAZER_IMPACT_FIZZ.as_str()),
		}
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

	pub fn fizz_settings() -> PlaybackSettings {
		PlaybackSettings::LOOP
			.with_volume(Volume::Linear(FIZZ_VOLUME))
			.with_spatial(true)
			.with_spatial_scale(FIZZ_SPATIAL_SCALE)
	}

	pub fn impact_settings(volume: Volume) -> PlaybackSettings {
		PlaybackSettings::DESPAWN
			.with_volume(volume)
			.with_spatial(true)
			.with_spatial_scale(IMPACT_SPATIAL_SCALE)
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

	pub fn loop_fizz(&self, commands: &mut Commands, projectile: Entity) {
		commands.spawn((
			Name::new("projectile-fizz"),
			ChildOf(projectile),
			Transform::IDENTITY,
			FlightFizz,
			AudioPlayer::new(self.lazer_fizz.clone()),
			Self::fizz_settings(),
		));
	}

	pub fn play_impact(&self, commands: &mut Commands, point: Vec3, distance: f32) {
		commands.spawn((
			Name::new("projectile-impact"),
			Transform::from_translation(point),
			AudioPlayer::new(self.lazer_impact.clone()),
			Self::impact_settings(FlightAttenuation::FIREARM.volume_at(IMPACT_VOLUME, distance)),
		));
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

/// Loudness from listener distance. A pass-by gets louder, then quieter.
pub(crate) fn attenuate_flight_fizz(
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
	mut emitters: Query<(&GlobalTransform, &mut SpatialAudioSink), With<FlightFizz>>,
) {
	let Some(listener) = listeners.iter().next() else {
		return;
	};
	let ear = listener.translation();
	let curve = FlightAttenuation::FIREARM;
	for (transform, mut sink) in &mut emitters {
		sink.set_volume(curve.volume(transform.translation().distance(ear)));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::audio::PlaybackMode;
	use std::path::Path;

	fn asset_exists(path: AssetPath) {
		let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets").join(path.as_str());
		assert!(file.is_file(), "{}", file.display());
	}

	#[test]
	fn authored_clips_are_in_assets() {
		asset_exists(WET_LASER_FIRE);
		asset_exists(LAZER_FIZZ);
		asset_exists(LAZER_IMPACT_FIZZ);
	}

	#[test]
	fn ballistic_shot_despawns_and_loops_loop() {
		assert!(matches!(FirearmFireSounds::shot_settings().mode, PlaybackMode::Despawn));
		assert!(matches!(FirearmFireSounds::laser_settings().mode, PlaybackMode::Loop));
		assert!(matches!(FirearmFireSounds::fizz_settings().mode, PlaybackMode::Loop));
		assert!(matches!(
			FirearmFireSounds::impact_settings(Volume::Linear(1.0)).mode,
			PlaybackMode::Despawn
		));
	}

	#[test]
	fn fire_clips_are_spatial() {
		let shot = FirearmFireSounds::shot_settings();
		let laser = FirearmFireSounds::laser_settings();
		let fizz = FirearmFireSounds::fizz_settings();
		assert!(shot.spatial && laser.spatial && fizz.spatial);
		assert_eq!(shot.spatial_scale.map(|scale| scale.0), Some(FIRE_SPATIAL_SCALE.0));
		assert_eq!(fizz.spatial_scale.map(|scale| scale.0), Some(FIZZ_SPATIAL_SCALE.0));
		let impact = FirearmFireSounds::impact_settings(Volume::Linear(1.0));
		assert!(impact.spatial);
		assert_eq!(impact.spatial_scale.map(|scale| scale.0), Some(IMPACT_SPATIAL_SCALE.0));
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
	fn flight_gain_follows_listener_distance_not_time() {
		let curve = FlightAttenuation::FIREARM;
		assert!((curve.gain(0.0) - 1.0).abs() < 1e-5);
		assert!((curve.gain(5.0) - 1.0).abs() < 1e-5);
		assert!((curve.gain(20.0) - curve.mid_gain).abs() < 1e-5);
		assert!((curve.gain(50.0) - curve.far_gain).abs() < 1e-5);
		assert!(curve.gain(100.0) < 1e-5);
		assert!(curve.gain(12.5) > curve.gain(20.0));
		assert!(curve.gain(20.0) > curve.gain(50.0));
		let near = curve.volume_at(IMPACT_VOLUME, 0.0);
		let far = curve.volume_at(IMPACT_VOLUME, 50.0);
		assert!(near.to_linear() > far.to_linear());
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
