//! Breeze and gust events placed near the listener.

use std::f32::consts::TAU;

use bevy::prelude::*;
use maybraid_audio::{
	Audio, AudioBus, AudioClip, AudioVelocity, Mixer, SpatialEmitter, SpatialOneShot,
};

pub const BREEZE_CLIP: &str = "sound-effects/environment/wind/breeze_001.wav";
pub const GUST_CLIP: &str = "sound-effects/environment/wind/gust_001.wav";

/// Quiet bed; radius stays wide so the loop still reads off-camera.
pub const BREEZE_VOLUME: f32 = 1.2;
pub const GUST_VOLUME: f32 = 1.0;
pub const WIND_SPATIAL_SCALE: f32 = 0.02;
pub const WIND_SPATIAL_RADIUS: f32 = 1.0 / WIND_SPATIAL_SCALE;
pub const BREEZE_LIFE_MIN: f32 = 10.0;
pub const BREEZE_LIFE_MAX: f32 = 15.0;
/// Authored gust is ~3 s.
pub const GUST_LIFE: f32 = 3.2;
pub const WIND_NEAR_MIN: f32 = 10.0;
pub const WIND_NEAR_MAX: f32 = 22.0;
const BREEZE_GAP_MIN: f32 = 4.0;
const BREEZE_GAP_MAX: f32 = 8.0;
const GUST_GAP_MIN: f32 = 14.0;
const GUST_GAP_MAX: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeatherKind {
	Breeze,
	Gust,
}

/// Live weather token. Despawn ends a looping breeze.
#[derive(Component, Clone, Copy, Debug)]
pub struct WeatherEvent {
	pub kind: WeatherKind,
	pub age: f32,
	pub life: f32,
}

impl WeatherEvent {
	pub fn breeze(life: f32) -> Self {
		Self {
			kind: WeatherKind::Breeze,
			age: 0.0,
			life: life.clamp(BREEZE_LIFE_MIN, BREEZE_LIFE_MAX),
		}
	}

	pub fn gust() -> Self {
		Self { kind: WeatherKind::Gust, age: 0.0, life: GUST_LIFE }
	}

	pub fn finished(self) -> bool {
		self.age >= self.life
	}
}

/// When the next breeze / gust may appear.
#[derive(Resource, Debug)]
pub struct WeatherClock {
	pub next_breeze: f32,
	pub next_gust: f32,
	pub noise: u64,
}

impl Default for WeatherClock {
	fn default() -> Self {
		Self { next_breeze: 1.5, next_gust: 6.0, noise: 0xC2B2_AE3D_27D4_EB4F }
	}
}

#[derive(Resource)]
pub struct WeatherSounds {
	pub breeze: Handle<AudioClip>,
	pub gust: Handle<AudioClip>,
}

impl WeatherSounds {
	pub fn load(assets: &AssetServer) -> Self {
		Self { breeze: assets.load(BREEZE_CLIP), gust: assets.load(GUST_CLIP) }
	}

	fn play_breeze(
		&self,
		commands: &mut Commands,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &Mixer,
		listener: &GlobalTransform,
		event: Entity,
		world: Vec3,
	) {
		audio.play_loop_or_queue(
			commands,
			&self.breeze,
			clips,
			SpatialEmitter::on(event, world)
				.radius(WIND_SPATIAL_RADIUS)
				.gain(BREEZE_VOLUME)
				.bus(AudioBus::Ambience),
			mixer,
			listener,
			"weather-breeze",
		);
	}

	fn play_gust(
		&self,
		commands: &mut Commands,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		world: Vec3,
	) {
		audio.play_or_queue(
			commands,
			&self.gust,
			clips,
			SpatialOneShot::at(world)
				.radius(WIND_SPATIAL_RADIUS)
				.gain(GUST_VOLUME)
				.bus(AudioBus::Ambience),
			mixer,
			listener,
			"weather-gust",
		);
	}
}

pub(crate) fn setup_weather_sounds(mut commands: Commands, assets: Res<AssetServer>) {
	commands.insert_resource(WeatherSounds::load(&assets));
}

pub(crate) fn despawn_finished_weather(
	time: Res<Time>,
	mut commands: Commands,
	mut events: Query<(Entity, &mut WeatherEvent)>,
) {
	let dt = time.delta_secs();
	for (entity, mut event) in &mut events {
		event.age += dt;
		if event.finished() {
			commands.entity(entity).try_despawn();
		}
	}
}

pub(crate) fn spawn_weather_near_listener(
	time: Res<Time>,
	mut commands: Commands,
	mut clock: ResMut<WeatherClock>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	sounds: Option<Res<WeatherSounds>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
	events: Query<&WeatherEvent>,
) {
	let (Some(audio), Some(sounds), Some(mut mixer), Some(listener)) =
		(audio.as_deref(), sounds.as_deref(), mixer, listeners.iter().next())
	else {
		return;
	};
	let dt = time.delta_secs();
	clock.next_breeze = (clock.next_breeze - dt).max(0.0);
	clock.next_gust = (clock.next_gust - dt).max(0.0);
	let origin = listener.translation();
	let mut live_breeze = false;
	let mut live_gust = false;
	for event in &events {
		match event.kind {
			WeatherKind::Breeze => live_breeze = true,
			WeatherKind::Gust => live_gust = true,
		}
	}
	if !live_breeze && clock.next_breeze <= 0.0 {
		let life = lerp(BREEZE_LIFE_MIN, BREEZE_LIFE_MAX, unit(&mut clock.noise));
		let world = point_near_listener(origin, &mut clock.noise);
		let event = commands
			.spawn((
				Name::new("weather-breeze"),
				Transform::from_translation(world),
				AudioVelocity(Vec3::ZERO),
				WeatherEvent::breeze(life),
			))
			.id();
		sounds.play_breeze(&mut commands, &clips, audio, &mixer, listener, event, world);
		clock.next_breeze = lerp(BREEZE_GAP_MIN, BREEZE_GAP_MAX, unit(&mut clock.noise));
	}
	if !live_gust && clock.next_gust <= 0.0 {
		let world = point_near_listener(origin, &mut clock.noise);
		commands.spawn((
			Name::new("weather-gust"),
			Transform::from_translation(world),
			WeatherEvent::gust(),
		));
		sounds.play_gust(&mut commands, &clips, audio, &mut mixer, listener, world);
		clock.next_gust = lerp(GUST_GAP_MIN, GUST_GAP_MAX, unit(&mut clock.noise));
	}
}

/// Planar offset around the listener, slightly above ear height.
pub fn point_near_listener(origin: Vec3, noise: &mut u64) -> Vec3 {
	let angle = unit(noise) * TAU;
	let dist = lerp(WIND_NEAR_MIN, WIND_NEAR_MAX, unit(noise));
	origin + Vec3::new(angle.cos() * dist, 0.6, angle.sin() * dist)
}

fn lerp(min: f32, max: f32, t: f32) -> f32 {
	min + (max - min) * t
}

fn unit(noise: &mut u64) -> f32 {
	(next_u64(noise) >> 11) as f32 / ((1u64 << 53) as f32)
}

fn next_u64(noise: &mut u64) -> u64 {
	let mut x = if *noise == 0 { 0x9E37_79B9_7F4A_7C15 } else { *noise };
	x ^= x >> 12;
	x ^= x << 25;
	x ^= x >> 27;
	*noise = x;
	x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	fn asset_bytes(path: &str) -> Vec<u8> {
		let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets").join(path);
		match std::fs::read(&file) {
			Ok(bytes) => bytes,
			Err(_) => panic!("{}", file.display()),
		}
	}

	#[test]
	fn authored_wind_clips_are_mono() {
		for path in [BREEZE_CLIP, GUST_CLIP] {
			assert!(
				maybraid_audio::decode_wav_mono(&asset_bytes(path)).is_ok(),
				"{path} must be authored mono"
			);
		}
	}

	#[test]
	fn breeze_lives_between_ten_and_fifteen_seconds() {
		let short = WeatherEvent::breeze(BREEZE_LIFE_MIN);
		let long = WeatherEvent::breeze(BREEZE_LIFE_MAX);
		assert!((short.life - BREEZE_LIFE_MIN).abs() < 1e-5);
		assert!((long.life - BREEZE_LIFE_MAX).abs() < 1e-5);
		assert!(WeatherEvent::breeze(1.0).life >= BREEZE_LIFE_MIN);
		assert!(WeatherEvent::breeze(40.0).life <= BREEZE_LIFE_MAX);
	}

	#[test]
	fn gust_is_a_short_oneshot() {
		let gust = WeatherEvent::gust();
		assert_eq!(gust.kind, WeatherKind::Gust);
		assert!(gust.life < BREEZE_LIFE_MIN);
		assert!(gust.life > 2.0);
	}

	#[test]
	fn wind_is_quiet_and_far() {
		assert!(BREEZE_VOLUME < 0.3);
		assert!(GUST_VOLUME < 0.35);
		assert!(WIND_SPATIAL_RADIUS > 40.0);
		assert!(WIND_SPATIAL_RADIUS > maybraid_audio::GRUNT_SPATIAL_RADIUS);
	}

	#[test]
	fn spawn_points_stay_near_the_listener() {
		let origin = Vec3::new(3.0, 1.0, -2.0);
		let mut noise = 7;
		for _ in 0..24 {
			let point = point_near_listener(origin, &mut noise);
			let planar = Vec2::new(point.x - origin.x, point.z - origin.z).length();
			assert!(planar >= WIND_NEAR_MIN - 1e-3);
			assert!(planar <= WIND_NEAR_MAX + 1e-3);
			assert!((point.y - origin.y - 0.6).abs() < 1e-4);
		}
	}

	#[test]
	fn finished_event_despawns() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, despawn_finished_weather);
		let live = app
			.world_mut()
			.spawn(WeatherEvent { kind: WeatherKind::Gust, age: 0.0, life: 1.0 })
			.id();
		let done = app
			.world_mut()
			.spawn(WeatherEvent { kind: WeatherKind::Breeze, age: 12.0, life: 10.0 })
			.id();
		app.update();
		assert!(app.world().get::<WeatherEvent>(live).is_some());
		assert!(app.world().get::<WeatherEvent>(done).is_none());
	}
}
