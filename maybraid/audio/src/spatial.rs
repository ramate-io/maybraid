//! Spatial play API and ECS voice sync.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bevy::asset::LoadState;
use bevy::prelude::*;
use oddio::{Cycle, FramesSignal, Gain, GainControl, Spatial, SpatialOptions};

use crate::asset::AudioClip;
use crate::backend::Audio;
use crate::mixer::{AudioBus, Mixer};

/// Virtual ear spacing for the Bevy [`SpatialListener`] marker.
pub const FIRE_LISTENER_GAP: f32 = 0.4;

/// Authoritative world velocity for a followed emitter (m/s).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AudioVelocity(pub Vec3);

/// World-fixed event. Does not follow an entity after play.
#[derive(Clone, Copy, Debug)]
pub struct SpatialOneShot {
	pub position: Vec3,
	pub radius: f32,
	pub gain: f32,
	pub bus: AudioBus,
	/// Seconds of silence before the clip. Sample-accurate on the mixer clock.
	pub delay: f32,
	/// Playback rate (`1.0` = authored pitch). Values below 1 lengthen the clip.
	pub playback_rate: f32,
	/// When false, skip the [`AudioBus::PlayerWeapon`] transient duck.
	pub duck: bool,
}

impl SpatialOneShot {
	pub fn at(position: Vec3) -> Self {
		Self {
			position,
			radius: 1.0,
			gain: 1.0,
			bus: AudioBus::Weapons,
			delay: 0.0,
			playback_rate: 1.0,
			duck: true,
		}
	}

	pub fn radius(mut self, radius: f32) -> Self {
		self.radius = radius;
		self
	}

	pub fn gain(mut self, gain: f32) -> Self {
		self.gain = gain;
		self
	}

	pub fn bus(mut self, bus: AudioBus) -> Self {
		self.bus = bus;
		self
	}

	pub fn delay(mut self, delay: f32) -> Self {
		self.delay = delay.max(0.0);
		self
	}

	pub fn duck(mut self, duck: bool) -> Self {
		self.duck = duck;
		self
	}

	pub fn playback_rate(mut self, rate: f32) -> Self {
		self.playback_rate = rate.clamp(0.5, 2.0);
		self
	}
}

/// Loop (or long source) that follows `follow` with [`AudioVelocity`].
#[derive(Clone, Copy, Debug)]
pub struct SpatialEmitter {
	pub follow: Entity,
	pub position: Vec3,
	pub velocity: Vec3,
	pub radius: f32,
	pub gain: f32,
	pub bus: AudioBus,
}

impl SpatialEmitter {
	pub fn on(follow: Entity, position: Vec3) -> Self {
		Self {
			follow,
			position,
			velocity: Vec3::ZERO,
			radius: 1.0,
			gain: 1.0,
			bus: AudioBus::Weapons,
		}
	}

	pub fn velocity(mut self, velocity: Vec3) -> Self {
		self.velocity = velocity;
		self
	}

	pub fn radius(mut self, radius: f32) -> Self {
		self.radius = radius;
		self
	}

	pub fn gain(mut self, gain: f32) -> Self {
		self.gain = gain;
		self
	}

	pub fn bus(mut self, bus: AudioBus) -> Self {
		self.bus = bus;
		self
	}
}

/// Live oddio voice. Dropping the component stops the signal.
#[derive(Component)]
pub struct SpatialVoice {
	spatial: Spatial,
	stop: Arc<AtomicBool>,
	gain: GainControl,
	clip_gain: f32,
	bus: AudioBus,
	/// `None` = world-fixed one-shot at [`Self::world`].
	follow: Option<Entity>,
	world: Vec3,
	/// Skip the spawn frame so a stale child GT cannot overwrite `play()`.
	armed: bool,
	despawn_when_done: bool,
}

impl Drop for SpatialVoice {
	fn drop(&mut self) {
		self.stop.store(true, Ordering::Relaxed);
	}
}

impl SpatialVoice {
	pub fn follows(&self) -> bool {
		self.follow.is_some()
	}
}

struct PlaybackRate<S> {
	inner: S,
	rate: f32,
}

impl<S> PlaybackRate<S> {
	fn new(inner: S, rate: f32) -> Self {
		Self { inner, rate: rate.clamp(0.5, 2.0) }
	}
}

impl<S: oddio::Signal<Frame = oddio::Sample>> oddio::Signal for PlaybackRate<S> {
	type Frame = oddio::Sample;

	fn sample(&mut self, interval: f32, out: &mut [oddio::Sample]) {
		self.inner.sample(interval * self.rate, out);
	}

	fn is_finished(&self) -> bool {
		self.inner.is_finished()
	}
}

struct Stoppable<T> {
	inner: T,
	stop: Arc<AtomicBool>,
}

impl<T> Stoppable<T> {
	fn new(inner: T, stop: Arc<AtomicBool>) -> Self {
		Self { inner, stop }
	}
}

impl<T: oddio::Signal<Frame = oddio::Sample>> oddio::Signal for Stoppable<T> {
	type Frame = oddio::Sample;

	fn sample(&mut self, interval: f32, out: &mut [oddio::Sample]) {
		if self.stop.load(Ordering::Relaxed) {
			out.fill(0.0);
			return;
		}
		self.inner.sample(interval, out);
	}

	fn is_finished(&self) -> bool {
		self.stop.load(Ordering::Relaxed) || self.inner.is_finished()
	}
}

/// Clip is still loading. Failed loads despawn this entity.
#[derive(Component)]
pub struct PendingPlay {
	pub clip: Handle<AudioClip>,
	pub name: &'static str,
	pub oneshot: Option<SpatialOneShot>,
	pub emitter: Option<SpatialEmitter>,
	pub looping: bool,
}

pub fn amplitude_to_db(amplitude: f32) -> f32 {
	if amplitude <= 1e-6 {
		-80.0
	} else {
		20.0 * amplitude.log10()
	}
}

fn point3(v: Vec3) -> mint::Point3<f32> {
	mint::Point3 { x: v.x, y: v.y, z: v.z }
}

fn vec3(v: Vec3) -> mint::Vector3<f32> {
	mint::Vector3 { x: v.x, y: v.y, z: v.z }
}

fn options(
	world: Vec3,
	world_velocity: Vec3,
	listener: &GlobalTransform,
	listener_vel: Vec3,
	radius: f32,
) -> SpatialOptions {
	SpatialOptions {
		position: point3(world - listener.translation()),
		velocity: vec3(world_velocity - listener_vel),
		radius,
	}
}

fn delayed_playback(
	frames: Arc<oddio::Frames<oddio::Sample>>,
	delay: f32,
	playback_rate: f32,
) -> PlaybackRate<FramesSignal<oddio::Sample>> {
	let rate = playback_rate.clamp(0.5, 2.0);
	let (_, frames) = FramesSignal::new(frames, -f64::from(delay * rate));
	PlaybackRate::new(frames, rate)
}

impl Audio {
	pub fn play(
		&self,
		commands: &mut Commands,
		clip: &AudioClip,
		spec: SpatialOneShot,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		name: &'static str,
	) {
		if spec.duck && spec.bus == AudioBus::PlayerWeapon {
			mixer.duck_player_shot();
		}
		let stop = Arc::new(AtomicBool::new(false));
		let rate = spec.playback_rate.clamp(0.5, 2.0);
		let frames = delayed_playback(clip.frames.clone(), spec.delay, rate);
		let (mut gain, signal) = Gain::new(Stoppable::new(frames, stop.clone()));
		let peak = spec.gain * mixer.gain(spec.bus);
		gain.set_amplitude_ratio(peak);
		let Some(spatial) = self.play_buffered(
			signal,
			options(spec.position, Vec3::ZERO, listener, self.listener_velocity(), spec.radius),
		) else {
			return;
		};
		commands.spawn((
			Name::new(name),
			Transform::from_translation(spec.position),
			SpatialVoice {
				spatial,
				stop,
				gain,
				clip_gain: spec.gain,
				bus: spec.bus,
				follow: None,
				world: spec.position,
				armed: false,
				despawn_when_done: true,
			},
		));
	}

	pub fn play_loop(
		&self,
		commands: &mut Commands,
		clip: &AudioClip,
		spec: SpatialEmitter,
		mixer: &Mixer,
		listener: &GlobalTransform,
		name: &'static str,
	) {
		let stop = Arc::new(AtomicBool::new(false));
		let (mut gain, signal) =
			Gain::new(Stoppable::new(Cycle::new(clip.frames.clone()), stop.clone()));
		gain.set_amplitude_ratio(spec.gain * mixer.gain(spec.bus));
		let Some(spatial) = self.play_buffered(
			signal,
			options(spec.position, spec.velocity, listener, self.listener_velocity(), spec.radius),
		) else {
			return;
		};
		commands.spawn((
			Name::new(name),
			ChildOf(spec.follow),
			Transform::IDENTITY,
			AudioVelocity(spec.velocity),
			SpatialVoice {
				spatial,
				stop,
				gain,
				clip_gain: spec.gain,
				bus: spec.bus,
				follow: Some(spec.follow),
				world: spec.position,
				armed: false,
				despawn_when_done: false,
			},
		));
	}

	pub fn play_or_queue(
		&self,
		commands: &mut Commands,
		handle: &Handle<AudioClip>,
		clips: &Assets<AudioClip>,
		spec: SpatialOneShot,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		name: &'static str,
	) {
		if let Some(clip) = clips.get(handle) {
			self.play(commands, clip, spec, mixer, listener, name);
			return;
		}
		commands.spawn((
			Name::new("pending-audio"),
			PendingPlay {
				clip: handle.clone(),
				name,
				oneshot: Some(spec),
				emitter: None,
				looping: false,
			},
		));
	}

	pub fn play_loop_or_queue(
		&self,
		commands: &mut Commands,
		handle: &Handle<AudioClip>,
		clips: &Assets<AudioClip>,
		spec: SpatialEmitter,
		mixer: &Mixer,
		listener: &GlobalTransform,
		name: &'static str,
	) {
		if let Some(clip) = clips.get(handle) {
			self.play_loop(commands, clip, spec, mixer, listener, name);
			return;
		}
		commands.spawn((
			Name::new("pending-audio"),
			PendingPlay {
				clip: handle.clone(),
				name,
				oneshot: None,
				emitter: Some(spec),
				looping: true,
			},
		));
	}
}

pub(crate) fn sync_listener(
	time: Res<Time>,
	audio: Option<Res<Audio>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
) {
	let Some(audio) = audio else {
		return;
	};
	let Some(listener) = listeners.iter().next() else {
		return;
	};
	audio.update_listener(listener.translation(), time.delta_secs());
	audio.set_listener_rotation(listener.rotation());
}

pub(crate) fn sync_voices(
	audio: Option<Res<Audio>>,
	mixer: Res<Mixer>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
	follows: Query<(&GlobalTransform, Option<&AudioVelocity>)>,
	mut voices: Query<&mut SpatialVoice>,
) {
	let Some(audio) = audio else {
		return;
	};
	let Some(listener) = listeners.iter().next() else {
		return;
	};
	let ear = listener.translation();
	let listener_vel = audio.listener_velocity();
	for mut voice in &mut voices {
		if !voice.armed {
			voice.armed = true;
			continue;
		}
		let (world, velocity) = if let Some(follow) = voice.follow {
			match follows.get(follow) {
				Ok((transform, motion)) => {
					(transform.translation(), motion.map(|motion| motion.0).unwrap_or(Vec3::ZERO))
				}
				Err(_) => {
					voice.stop.store(true, Ordering::Relaxed);
					continue;
				}
			}
		} else {
			(voice.world, Vec3::ZERO)
		};
		voice
			.spatial
			.set_motion(point3(world - ear), vec3(velocity - listener_vel), false);
		let peak = voice.clip_gain * mixer.gain(voice.bus);
		voice.gain.set_amplitude_ratio(peak);
		voice.world = world;
	}
}

pub(crate) fn despawn_finished_voices(
	mut commands: Commands,
	voices: Query<(Entity, &SpatialVoice)>,
) {
	for (entity, voice) in &voices {
		if voice.despawn_when_done && voice.spatial.is_finished() {
			commands.entity(entity).try_despawn();
		}
	}
}

pub(crate) fn flush_pending(
	mut commands: Commands,
	audio: Option<Res<Audio>>,
	mut mixer: ResMut<Mixer>,
	clips: Res<Assets<AudioClip>>,
	assets: Res<AssetServer>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
	follows: Query<Entity>,
	pending: Query<(Entity, &PendingPlay)>,
) {
	let (Some(audio), Some(listener)) = (audio.as_deref(), listeners.iter().next()) else {
		return;
	};
	for (entity, pending) in &pending {
		match assets.load_state(&pending.clip) {
			LoadState::Failed(_) => {
				commands.entity(entity).try_despawn();
				continue;
			}
			LoadState::Loaded => {}
			_ => continue,
		}
		let Some(clip) = clips.get(&pending.clip) else {
			continue;
		};
		if pending.looping {
			let Some(spec) = pending.emitter else {
				commands.entity(entity).try_despawn();
				continue;
			};
			if follows.get(spec.follow).is_err() {
				commands.entity(entity).try_despawn();
				continue;
			}
			audio.play_loop(&mut commands, clip, spec, &mixer, listener, pending.name);
		} else if let Some(spec) = pending.oneshot {
			audio.play(&mut commands, clip, spec, &mut mixer, listener, pending.name);
		}
		commands.entity(entity).try_despawn();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn oneshot_builder_is_world_fixed() {
		let spec = SpatialOneShot::at(Vec3::X)
			.radius(20.0)
			.gain(0.7)
			.bus(AudioBus::PlayerWeapon)
			.delay(0.016)
			.duck(false);
		assert!((spec.position - Vec3::X).length() < 1e-5);
		assert!((spec.radius - 20.0).abs() < 1e-5);
		assert!(matches!(spec.bus, AudioBus::PlayerWeapon));
		assert!((spec.delay - 0.016).abs() < 1e-5);
		assert!(!spec.duck);
	}

	#[test]
	fn playback_rate_preserves_mixer_clock_delay() {
		use oddio::Signal;

		const SAMPLE_RATE: u32 = 48_000;
		let source = oddio::Frames::from_slice(SAMPLE_RATE, &[1.0_f32; 144_000]);
		for rate in [0.5_f32, 1.0, 2.0, 0.1, 3.0] {
			let effective_rate = rate.clamp(0.5, 2.0);
			let mut signal = delayed_playback(source.clone(), 1.0, rate);
			let mut frame = [0.0];
			let first_audible = (0..96_000).find(|_| {
				signal.sample(1.0 / SAMPLE_RATE as f32, &mut frame);
				frame[0].abs() > 0.1
			});
			assert!(first_audible.is_some(), "rate {rate} signal never started");
			let Some(first_audible) = first_audible else { continue };
			let onset = first_audible as f32 / SAMPLE_RATE as f32;
			// FramesSignal interpolates the first sample across the negative-to-zero
			// boundary. At the slowest rate that spans two mixer samples; allow two
			// more samples for the accumulated f32 phase.
			let boundary_tolerance = 4.0 / SAMPLE_RATE as f32;
			assert!(
				(onset - 1.0).abs() <= boundary_tolerance,
				"rate {rate} (effective {effective_rate}) began at {onset:.8}s"
			);
		}
	}

	#[test]
	fn silent_amplitude_is_very_quiet_db() {
		assert!(amplitude_to_db(1.0).abs() < 1e-5);
		assert!(amplitude_to_db(0.0) <= -80.0);
		assert!(amplitude_to_db(0.5) < 0.0);
	}
}
