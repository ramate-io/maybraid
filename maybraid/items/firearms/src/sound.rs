//! Four-layer firearm SFX: fire, hammer, fizz, and impact.
//!
//! Fire and hammer are muzzle one-shots on every pull. Bolts and bullets then
//! carry a looping fizz until the flight despawns; a hit plays a world-space
//! impact. A laser loops fire on the beam and plays hammer once when it starts.
//! Each layer has its own volume and spatial radius. Distance is oddio's
//! `radius / max(distance, radius)`; we do not apply a second falloff curve.
//!
//! Playback is an [`oddio::SpatialScene`]: ITD, ILD, 1/r, Doppler, and
//! propagation delay. Bevy [`SpatialListener`] is the ear pose only.

mod scene;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bevy::audio::AudioSource;
use bevy::prelude::*;
use oddio::{Cycle, FixedGain, Frames, FramesSignal, Sample};

use firearms_components::AssetPath;

use scene::{amplitude_to_db, clip_frames, decode_wav_mono, Stoppable};

pub use scene::OddioScene;
pub(crate) use scene::{
	despawn_finished_oddio_voices, preferred_sample_format, setup_oddio_scene, sync_oddio_listener,
	sync_oddio_voices, MOTION_TELEPORT_M,
};

/// Energy report at the muzzle.
pub const WEAPON_FIRE: AssetPath =
	AssetPath::new("sound-effects/weapons/firearms/fire__wet_lazer_001.wav");
/// Percussive layer fired with [`WEAPON_FIRE`].
pub const WEAPON_HAMMER: AssetPath =
	AssetPath::new("sound-effects/weapons/firearms/hammer__lazer_001.wav");
/// Seamless flight loop on a bolt or bullet.
pub const WEAPON_FIZZ: AssetPath =
	AssetPath::new("sound-effects/weapons/firearms/fizz__lazer_001.wav");
/// One-shot when a bolt or bullet first crosses a collider.
pub const WEAPON_IMPACT: AssetPath =
	AssetPath::new("sound-effects/weapons/firearms/impact__lazer_001.wav");

pub const FIRE_VOLUME: f32 = 0.7;
pub const HAMMER_VOLUME: f32 = 1.0;
pub const FIZZ_VOLUME: f32 = 1.3;
pub const IMPACT_VOLUME: f32 = 0.8;
/// Inverse of the oddio zero-attenuation radius (meters). Same numbers as the
/// old Bevy `SpatialScale`: smaller scale = the layer carries farther.
pub const FIRE_SPATIAL_SCALE: f32 = 0.05;
pub const HAMMER_SPATIAL_SCALE: f32 = 0.1;
pub const FIZZ_SPATIAL_SCALE: f32 = 0.2;
pub const IMPACT_SPATIAL_SCALE: f32 = 0.2;
pub const FIRE_SPATIAL_RADIUS: f32 = 1.0 / FIRE_SPATIAL_SCALE;
pub const HAMMER_SPATIAL_RADIUS: f32 = 1.0 / HAMMER_SPATIAL_SCALE;
pub const FIZZ_SPATIAL_RADIUS: f32 = 1.0 / FIZZ_SPATIAL_SCALE;
pub const IMPACT_SPATIAL_RADIUS: f32 = 1.0 / IMPACT_SPATIAL_SCALE;
/// Virtual ear spacing for the Bevy listener marker. Oddio uses its own head
/// radius; this only keeps [`SpatialListener`] constructible.
pub const FIRE_LISTENER_GAP: f32 = 0.4;
/// Laser mesh is a cylinder centered on the beam. Parent scale.y is length, so
/// local −0.5 Y sits on the muzzle.
pub const LASER_MUZZLE_LOCAL: Vec3 = Vec3::new(0.0, -0.5, 0.0);

/// Looping fizz emitter parented to a bolt or bullet.
#[derive(Component, Clone, Copy, Debug)]
pub struct FlightFizz;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FirearmClipKind {
	Fire,
	Hammer,
	Fizz,
	Impact,
}

/// Play this layer once [`AudioSource`] bytes are decoded.
#[derive(Component, Debug)]
pub(crate) struct PendingFirearmClip {
	kind: FirearmClipKind,
	parent: Option<Entity>,
	local: Vec3,
	world: Vec3,
	volume: f32,
	radius: f32,
	looped: bool,
	fizz: bool,
}

/// Loaded fire, hammer, fizz, and impact clips. Missing this resource is a silent no-op.
#[derive(Resource)]
pub struct FirearmFireSounds {
	pub fire: Handle<AudioSource>,
	pub hammer: Handle<AudioSource>,
	pub fizz: Handle<AudioSource>,
	pub impact: Handle<AudioSource>,
	decoded_fire: Option<Arc<Frames<Sample>>>,
	decoded_hammer: Option<Arc<Frames<Sample>>>,
	decoded_fizz: Option<Arc<Frames<Sample>>>,
	decoded_impact: Option<Arc<Frames<Sample>>>,
}

impl FirearmFireSounds {
	pub fn load(asset_server: &AssetServer) -> Self {
		Self {
			fire: asset_server.load(WEAPON_FIRE.as_str()),
			hammer: asset_server.load(WEAPON_HAMMER.as_str()),
			fizz: asset_server.load(WEAPON_FIZZ.as_str()),
			impact: asset_server.load(WEAPON_IMPACT.as_str()),
			decoded_fire: None,
			decoded_hammer: None,
			decoded_fizz: None,
			decoded_impact: None,
		}
	}

	pub fn listener() -> SpatialListener {
		SpatialListener::new(FIRE_LISTENER_GAP)
	}

	fn fire_frames(&mut self, sources: &Assets<AudioSource>) -> Option<Arc<Frames<Sample>>> {
		clip_frames(&self.fire, sources, &mut self.decoded_fire)
	}

	fn hammer_frames(&mut self, sources: &Assets<AudioSource>) -> Option<Arc<Frames<Sample>>> {
		clip_frames(&self.hammer, sources, &mut self.decoded_hammer)
	}

	fn fizz_frames(&mut self, sources: &Assets<AudioSource>) -> Option<Arc<Frames<Sample>>> {
		clip_frames(&self.fizz, sources, &mut self.decoded_fizz)
	}

	fn impact_frames(&mut self, sources: &Assets<AudioSource>) -> Option<Arc<Frames<Sample>>> {
		clip_frames(&self.impact, sources, &mut self.decoded_impact)
	}

	fn frames_for(
		&mut self,
		kind: FirearmClipKind,
		sources: &Assets<AudioSource>,
	) -> Option<Arc<Frames<Sample>>> {
		match kind {
			FirearmClipKind::Fire => self.fire_frames(sources),
			FirearmClipKind::Hammer => self.hammer_frames(sources),
			FirearmClipKind::Fizz => self.fizz_frames(sources),
			FirearmClipKind::Impact => self.impact_frames(sources),
		}
	}

	fn queue_clip(
		commands: &mut Commands,
		kind: FirearmClipKind,
		parent: Option<Entity>,
		local: Vec3,
		world: Vec3,
		volume: f32,
		radius: f32,
		looped: bool,
		fizz: bool,
	) {
		commands.spawn((
			Name::new("pending-firearm-clip"),
			PendingFirearmClip { kind, parent, local, world, volume, radius, looped, fizz },
		));
	}

	fn play_or_queue_oneshot(
		&mut self,
		commands: &mut Commands,
		sources: &Assets<AudioSource>,
		scene: &OddioScene,
		listener: &GlobalTransform,
		kind: FirearmClipKind,
		name: &'static str,
		parent: Option<Entity>,
		local: Vec3,
		world: Vec3,
		volume: f32,
		radius: f32,
	) {
		match self.frames_for(kind, sources) {
			Some(frames) => self.spawn_oneshot(
				commands, scene, listener, name, frames, parent, local, world, volume, radius,
			),
			None => {
				Self::queue_clip(commands, kind, parent, local, world, volume, radius, false, false)
			}
		}
	}

	fn play_or_queue_loop(
		&mut self,
		commands: &mut Commands,
		sources: &Assets<AudioSource>,
		scene: &OddioScene,
		listener: &GlobalTransform,
		kind: FirearmClipKind,
		name: &'static str,
		parent: Entity,
		local: Vec3,
		world: Vec3,
		volume: f32,
		radius: f32,
		fizz: bool,
	) {
		match self.frames_for(kind, sources) {
			Some(frames) => self.spawn_loop(
				commands, scene, listener, name, frames, parent, local, world, volume, radius, fizz,
			),
			None => Self::queue_clip(
				commands,
				kind,
				Some(parent),
				local,
				world,
				volume,
				radius,
				true,
				fizz,
			),
		}
	}

	fn spawn_oneshot(
		&self,
		commands: &mut Commands,
		scene: &OddioScene,
		listener: &GlobalTransform,
		name: &'static str,
		frames: Arc<Frames<Sample>>,
		parent: Option<Entity>,
		local: Vec3,
		world: Vec3,
		volume: f32,
		radius: f32,
	) {
		let stop = Arc::new(AtomicBool::new(false));
		let signal = Stoppable::new(
			FixedGain::new(FramesSignal::from(frames), amplitude_to_db(volume)),
			stop.clone(),
		);
		let Some(spatial) = scene.play_seek(signal, OddioScene::options(world, listener, radius))
		else {
			return;
		};
		let mut entity = commands.spawn((
			Name::new(name),
			Transform::from_translation(local),
			OddioScene::voice(spatial, stop, world, true),
		));
		if let Some(parent) = parent {
			entity.insert(ChildOf(parent));
		}
	}

	fn spawn_loop(
		&self,
		commands: &mut Commands,
		scene: &OddioScene,
		listener: &GlobalTransform,
		name: &'static str,
		frames: Arc<Frames<Sample>>,
		parent: Entity,
		local: Vec3,
		world: Vec3,
		volume: f32,
		radius: f32,
		fizz: bool,
	) {
		let stop = Arc::new(AtomicBool::new(false));
		let signal = Stoppable::new(
			FixedGain::new(Cycle::new(frames), amplitude_to_db(volume)),
			stop.clone(),
		);
		let Some(spatial) = scene.play_seek(signal, OddioScene::options(world, listener, radius))
		else {
			return;
		};
		let mut entity = commands.spawn((
			Name::new(name),
			ChildOf(parent),
			Transform::from_translation(local),
			OddioScene::voice(spatial, stop, world, false),
		));
		if fizz {
			entity.insert(FlightFizz);
		}
	}

	pub fn play_shot(
		&mut self,
		commands: &mut Commands,
		sources: &Assets<AudioSource>,
		scene: &OddioScene,
		listener: &GlobalTransform,
		barrel: Entity,
		muzzle_local: Vec3,
		world: Vec3,
	) {
		self.play_or_queue_oneshot(
			commands,
			sources,
			scene,
			listener,
			FirearmClipKind::Fire,
			"firearm-fire",
			Some(barrel),
			muzzle_local,
			world,
			FIRE_VOLUME,
			FIRE_SPATIAL_RADIUS,
		);
		self.play_or_queue_oneshot(
			commands,
			sources,
			scene,
			listener,
			FirearmClipKind::Hammer,
			"firearm-hammer",
			Some(barrel),
			muzzle_local,
			world,
			HAMMER_VOLUME,
			HAMMER_SPATIAL_RADIUS,
		);
	}

	pub fn loop_on(
		&mut self,
		commands: &mut Commands,
		sources: &Assets<AudioSource>,
		scene: &OddioScene,
		listener: &GlobalTransform,
		laser: Entity,
		world: Vec3,
	) {
		self.play_or_queue_loop(
			commands,
			sources,
			scene,
			listener,
			FirearmClipKind::Fire,
			"firearm-fire",
			laser,
			LASER_MUZZLE_LOCAL,
			world,
			FIRE_VOLUME,
			FIRE_SPATIAL_RADIUS,
			false,
		);
		self.play_or_queue_oneshot(
			commands,
			sources,
			scene,
			listener,
			FirearmClipKind::Hammer,
			"firearm-hammer",
			Some(laser),
			LASER_MUZZLE_LOCAL,
			world,
			HAMMER_VOLUME,
			HAMMER_SPATIAL_RADIUS,
		);
	}

	pub fn loop_fizz(
		&mut self,
		commands: &mut Commands,
		sources: &Assets<AudioSource>,
		scene: &OddioScene,
		listener: &GlobalTransform,
		projectile: Entity,
		world: Vec3,
	) {
		self.play_or_queue_loop(
			commands,
			sources,
			scene,
			listener,
			FirearmClipKind::Fizz,
			"projectile-fizz",
			projectile,
			Vec3::ZERO,
			world,
			FIZZ_VOLUME,
			FIZZ_SPATIAL_RADIUS,
			true,
		);
	}

	pub fn play_impact(
		&mut self,
		commands: &mut Commands,
		sources: &Assets<AudioSource>,
		scene: &OddioScene,
		listener: &GlobalTransform,
		point: Vec3,
	) {
		self.play_or_queue_oneshot(
			commands,
			sources,
			scene,
			listener,
			FirearmClipKind::Impact,
			"projectile-impact",
			None,
			point,
			point,
			IMPACT_VOLUME,
			IMPACT_SPATIAL_RADIUS,
		);
	}

	fn clip_name(kind: FirearmClipKind) -> &'static str {
		match kind {
			FirearmClipKind::Fire => "firearm-fire",
			FirearmClipKind::Hammer => "firearm-hammer",
			FirearmClipKind::Fizz => "projectile-fizz",
			FirearmClipKind::Impact => "projectile-impact",
		}
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

pub(crate) fn flush_pending_firearm_clips(
	mut commands: Commands,
	mut sounds: Option<ResMut<FirearmFireSounds>>,
	sources: Res<Assets<AudioSource>>,
	scene: Option<Res<OddioScene>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
	globals: Query<&GlobalTransform>,
	pending: Query<(Entity, &PendingFirearmClip)>,
) {
	let (Some(sounds), Some(scene), Some(listener)) =
		(sounds.as_deref_mut(), scene.as_deref(), listeners.iter().next())
	else {
		return;
	};
	for (entity, pending) in &pending {
		if let Some(parent) = pending.parent {
			if globals.get(parent).is_err() {
				commands.entity(entity).try_despawn();
				continue;
			}
		}
		let Some(frames) = sounds.frames_for(pending.kind, &sources) else {
			continue;
		};
		let world = pending
			.parent
			.and_then(|parent| globals.get(parent).ok())
			.map(|global| global.transform_point(pending.local))
			.unwrap_or(pending.world);
		if pending.looped {
			let Some(parent) = pending.parent else {
				commands.entity(entity).try_despawn();
				continue;
			};
			sounds.spawn_loop(
				&mut commands,
				scene,
				listener,
				FirearmFireSounds::clip_name(pending.kind),
				frames,
				parent,
				pending.local,
				world,
				pending.volume,
				pending.radius,
				pending.fizz,
			);
		} else {
			sounds.spawn_oneshot(
				&mut commands,
				scene,
				listener,
				FirearmFireSounds::clip_name(pending.kind),
				frames,
				pending.parent,
				pending.local,
				world,
				pending.volume,
				pending.radius,
			);
		}
		commands.entity(entity).try_despawn();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	fn asset_exists(path: AssetPath) {
		let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets").join(path.as_str());
		assert!(file.is_file(), "{}", file.display());
	}

	#[test]
	fn authored_clips_are_in_assets() {
		asset_exists(WEAPON_FIRE);
		asset_exists(WEAPON_HAMMER);
		asset_exists(WEAPON_FIZZ);
		asset_exists(WEAPON_IMPACT);
	}

	#[test]
	fn each_layer_has_its_own_spatial_radius() {
		assert!((FIRE_SPATIAL_RADIUS - 1.0 / FIRE_SPATIAL_SCALE).abs() < 1e-5);
		assert!((HAMMER_SPATIAL_RADIUS - 1.0 / HAMMER_SPATIAL_SCALE).abs() < 1e-5);
		assert!((FIZZ_SPATIAL_RADIUS - 1.0 / FIZZ_SPATIAL_SCALE).abs() < 1e-5);
		assert!((IMPACT_SPATIAL_RADIUS - 1.0 / IMPACT_SPATIAL_SCALE).abs() < 1e-5);
		assert!(FIRE_SPATIAL_RADIUS > HAMMER_SPATIAL_RADIUS);
		assert!(HAMMER_SPATIAL_RADIUS > IMPACT_SPATIAL_RADIUS);
		assert!(IMPACT_SPATIAL_RADIUS > FIZZ_SPATIAL_RADIUS);
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
	fn prefers_f32_stereo_then_i16() {
		use cpal::SampleFormat;
		assert!(matches!(
			preferred_sample_format(Some((SampleFormat::I16, 2)), &[(SampleFormat::F32, 2)]),
			Some(SampleFormat::F32)
		));
		assert!(matches!(
			preferred_sample_format(Some((SampleFormat::I16, 2)), &[]),
			Some(SampleFormat::I16)
		));
		assert!(preferred_sample_format(Some((SampleFormat::F32, 1)), &[(SampleFormat::I16, 1)])
			.is_none());
	}

	#[test]
	fn far_jump_is_a_teleport() {
		assert!(!super::scene::OddioVoice::motion_discontinuity(Vec3::ZERO, Vec3::X * 3.0));
		assert!(super::scene::OddioVoice::motion_discontinuity(
			Vec3::ZERO,
			Vec3::X * (MOTION_TELEPORT_M + 1.0)
		));
	}

	#[test]
	fn silent_amplitude_is_very_quiet_db() {
		assert!(amplitude_to_db(1.0).abs() < 1e-5);
		assert!(amplitude_to_db(0.0) <= -80.0);
		assert!(amplitude_to_db(0.5) < 0.0);
	}

	#[test]
	fn ieee_float_wav_decodes_to_mono_frames() {
		let path = Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("../../assets")
			.join(WEAPON_FIRE.as_str());
		let bytes = match std::fs::read(&path) {
			Ok(bytes) => bytes,
			Err(_) => {
				assert!(path.is_file(), "{}", path.display());
				return;
			}
		};
		assert!(decode_wav_mono(&bytes).is_some(), "decode {}", path.display());
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
