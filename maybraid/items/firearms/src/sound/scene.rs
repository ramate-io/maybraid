//! cpal output thread plus an [`oddio::SpatialScene`].
//!
//! Positions are world-space, translated so the listener sits at the origin
//! and not rotated. Oddio then applies ITD, ILD, 1/r, Doppler, and propagation
//! delay. [`SpatialListener`] is only a pose marker; Bevy's rodio mixer is not
//! used for these clips.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use bevy::audio::AudioSource;
use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use oddio::{Frames, GainControl, Sample, Seek, Signal, Spatial, SpatialOptions, SpatialScene};

use crate::sound::FlightFizz;

/// Shared oddio mixer. Missing this resource is a silent no-op.
#[derive(Resource)]
pub struct OddioScene {
	control: Mutex<oddio::SpatialSceneControl>,
	pub sample_rate: u32,
	listener: Mutex<ListenerMotion>,
}

struct ListenerMotion {
	last: Option<Vec3>,
	velocity: Vec3,
}

/// Playing spatial voice. Dropping the component stops the oddio signal.
#[derive(Component)]
pub(crate) struct OddioVoice {
	spatial: Spatial,
	stop: Arc<AtomicBool>,
	gain: Option<GainControl>,
	last_world: Option<Vec3>,
	/// False on the spawn frame so we do not `set_motion` from a stale GT.
	follow_transform: bool,
	despawn_when_done: bool,
}

impl Drop for OddioVoice {
	fn drop(&mut self) {
		self.stop.store(true, Ordering::Relaxed);
	}
}

/// Mono signal that finishes when the owning entity is despawned.
pub(crate) struct Stoppable<T> {
	inner: T,
	stop: Arc<AtomicBool>,
}

impl<T> Stoppable<T> {
	pub(crate) fn new(inner: T, stop: Arc<AtomicBool>) -> Self {
		Self { inner, stop }
	}
}

impl<T: Signal<Frame = Sample>> Signal for Stoppable<T> {
	type Frame = Sample;

	fn sample(&mut self, interval: f32, out: &mut [Sample]) {
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

impl<T: Seek<Frame = Sample>> Seek for Stoppable<T> {
	fn seek(&mut self, seconds: f32) {
		self.inner.seek(seconds);
	}
}

/// Farther than a bolt travels in a couple of ticks: treat as a teleport.
pub(crate) const MOTION_TELEPORT_M: f32 = 8.0;

impl OddioScene {
	pub fn start() -> Option<Self> {
		let host = cpal::default_host();
		let device = host.default_output_device()?;
		let (config, format) = output_config(&device)?;
		let rate = config.sample_rate.0;
		let (control, scene) = SpatialScene::new();
		thread::Builder::new()
			.name("oddio-output".into())
			.spawn(move || {
				let Some(stream) = bind_output_stream(device, config, format, scene, rate) else {
					return;
				};
				if stream.play().is_err() {
					return;
				}
				loop {
					thread::park();
				}
			})
			.ok()?;
		Some(Self {
			control: Mutex::new(control),
			sample_rate: rate,
			listener: Mutex::new(ListenerMotion { last: None, velocity: Vec3::ZERO }),
		})
	}

	pub fn update_listener(&self, listener: Vec3, dt: f32) -> Vec3 {
		let Ok(mut motion) = self.listener.lock() else {
			return Vec3::ZERO;
		};
		motion.velocity = motion
			.last
			.map(|prev| {
				if dt > 1e-5 {
					(listener - prev) / dt
				} else {
					Vec3::ZERO
				}
			})
			.unwrap_or(Vec3::ZERO);
		motion.last = Some(listener);
		motion.velocity
	}

	pub fn listener_velocity(&self) -> Vec3 {
		self.listener.lock().map(|motion| motion.velocity).unwrap_or(Vec3::ZERO)
	}

	pub fn set_listener_rotation(&self, rotation: Quat) {
		let Ok(mut control) = self.control.lock() else {
			return;
		};
		control.set_listener_rotation(mint::Quaternion {
			s: rotation.w,
			v: mint::Vector3 { x: rotation.x, y: rotation.y, z: rotation.z },
		});
	}

	pub fn play_seek<S: Seek<Frame = Sample> + Send + 'static>(
		&self,
		signal: S,
		options: SpatialOptions,
	) -> Option<Spatial> {
		self.control.lock().ok().map(|mut control| control.play(signal, options))
	}

	pub fn play_buffered<S: Signal<Frame = Sample> + Send + 'static>(
		&self,
		signal: S,
		options: SpatialOptions,
	) -> Option<Spatial> {
		let rate = self.sample_rate;
		self.control.lock().ok().map(|mut control| {
			control.play_buffered(signal, options, 120.0, rate, 0.1)
		})
	}

	pub fn options(world: Vec3, listener: &GlobalTransform, radius: f32) -> SpatialOptions {
		let rel = world - listener.translation();
		SpatialOptions { position: point3(rel), velocity: [0.0, 0.0, 0.0].into(), radius }
	}

	pub(crate) fn voice(
		spatial: Spatial,
		stop: Arc<AtomicBool>,
		gain: Option<GainControl>,
		world: Vec3,
		despawn_when_done: bool,
	) -> OddioVoice {
		OddioVoice {
			spatial,
			stop,
			gain,
			last_world: Some(world),
			follow_transform: false,
			despawn_when_done,
		}
	}
}

impl OddioVoice {
	/// First call after spawn: keep the `play()` pose. Later calls follow GT.
	pub(crate) fn follow_after_spawn(&mut self) -> bool {
		if self.follow_transform {
			return true;
		}
		self.follow_transform = true;
		false
	}

	pub(crate) fn motion_discontinuity(previous: Vec3, world: Vec3) -> bool {
		world.distance(previous) > MOTION_TELEPORT_M
	}
}

pub(crate) fn setup_oddio_scene(mut commands: Commands) {
	if let Some(scene) = OddioScene::start() {
		commands.insert_resource(scene);
	}
}

pub(crate) fn sync_oddio_listener(
	time: Res<Time>,
	scene: Option<Res<OddioScene>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
) {
	let Some(scene) = scene else {
		return;
	};
	let Some(listener) = listeners.iter().next() else {
		return;
	};
	scene.update_listener(listener.translation(), time.delta_secs());
	scene.set_listener_rotation(listener.rotation());
}

pub(crate) fn sync_oddio_voices(
	time: Res<Time>,
	scene: Option<Res<OddioScene>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
	mut voices: Query<(&GlobalTransform, &mut OddioVoice)>,
) {
	let Some(scene) = scene else {
		return;
	};
	let Some(listener) = listeners.iter().next() else {
		return;
	};
	let dt = time.delta_secs();
	let ear = listener.translation();
	let listener_vel = scene.listener_velocity();
	for (transform, mut voice) in &mut voices {
		if !voice.follow_after_spawn() {
			continue;
		}
		let world = transform.translation();
		let previous = voice.last_world.unwrap_or(world);
		let discontinuity = OddioVoice::motion_discontinuity(previous, world);
		let velocity = if discontinuity || dt <= 1e-5 {
			Vec3::ZERO
		} else {
			(world - previous) / dt - listener_vel
		};
		voice.spatial.set_motion(point3(world - ear), vec3(velocity), discontinuity);
		voice.last_world = Some(world);
	}
}

pub(crate) fn despawn_finished_oddio_voices(
	mut commands: Commands,
	voices: Query<(Entity, &OddioVoice)>,
) {
	for (entity, voice) in &voices {
		if voice.despawn_when_done && voice.spatial.is_finished() {
			commands.entity(entity).try_despawn();
		}
	}
}

pub(crate) fn attenuate_flight_fizz(
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
	mut emitters: Query<(&GlobalTransform, &mut OddioVoice), With<FlightFizz>>,
) {
	let Some(listener) = listeners.iter().next() else {
		return;
	};
	let ear = listener.translation();
	let curve = crate::sound::FlightAttenuation::FIREARM;
	for (transform, mut voice) in &mut emitters {
		let Some(gain) = voice.gain.as_mut() else {
			continue;
		};
		gain.set_amplitude_ratio(curve.gain(transform.translation().distance(ear)) * crate::sound::FIZZ_VOLUME);
	}
}

pub(crate) fn decode_wav_mono(bytes: &[u8]) -> Option<Arc<Frames<Sample>>> {
	let mut reader = hound::WavReader::new(std::io::Cursor::new(bytes)).ok()?;
	let spec = reader.spec();
	let rate = spec.sample_rate;
	if rate == 0 {
		return None;
	}
	let channels = usize::from(spec.channels.max(1));
	let pcm = read_pcm(&mut reader, spec)?;
	if pcm.is_empty() {
		return None;
	}
	let mono = if channels == 1 {
		pcm
	} else {
		pcm.chunks(channels)
			.map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
			.collect()
	};
	Some(Frames::from_slice(rate, &mono))
}

fn read_pcm(reader: &mut hound::WavReader<std::io::Cursor<&[u8]>>, spec: hound::WavSpec) -> Option<Vec<f32>> {
	match spec.sample_format {
		hound::SampleFormat::Float => {
			let mut out = Vec::with_capacity(reader.duration() as usize);
			for sample in reader.samples::<f32>() {
				out.push(sample.ok()?);
			}
			Some(out)
		}
		hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
			let max = f32::from(i16::MAX);
			let mut out = Vec::with_capacity(reader.duration() as usize);
			for sample in reader.samples::<i16>() {
				out.push(f32::from(sample.ok()?) / max);
			}
			Some(out)
		}
		hound::SampleFormat::Int => {
			let shift = 32u32.saturating_sub(u32::from(spec.bits_per_sample));
			let max = (i32::MAX as u32 >> shift) as f32;
			if max <= 0.0 {
				return None;
			}
			let mut out = Vec::with_capacity(reader.duration() as usize);
			for sample in reader.samples::<i32>() {
				out.push(sample.ok()? as f32 / max);
			}
			Some(out)
		}
	}
}

pub(crate) fn clip_frames(
	handle: &Handle<AudioSource>,
	sources: &Assets<AudioSource>,
	cache: &mut Option<Arc<Frames<Sample>>>,
) -> Option<Arc<Frames<Sample>>> {
	if cache.is_none() {
		*cache = sources.get(handle).and_then(|source| decode_wav_mono(source.bytes.as_ref()));
	}
	cache.clone()
}

pub(crate) fn amplitude_to_db(amplitude: f32) -> f32 {
	if amplitude <= 1e-6 {
		-80.0
	} else {
		20.0 * amplitude.log10()
	}
}

pub(crate) fn point3(v: Vec3) -> mint::Point3<f32> {
	mint::Point3 { x: v.x, y: v.y, z: v.z }
}

pub(crate) fn vec3(v: Vec3) -> mint::Vector3<f32> {
	mint::Vector3 { x: v.x, y: v.y, z: v.z }
}

/// Prefer F32 stereo, then I16 stereo. Mono-only devices are unsupported.
pub(crate) fn preferred_sample_format(
	default: Option<(SampleFormat, u16)>,
	supported: &[(SampleFormat, u16)],
) -> Option<SampleFormat> {
	let has = |format: SampleFormat| {
		supported.iter().any(|(candidate, channels)| *candidate == format && *channels >= 2)
	};
	if matches!(default, Some((SampleFormat::F32, channels)) if channels >= 2) {
		return Some(SampleFormat::F32);
	}
	if has(SampleFormat::F32) {
		return Some(SampleFormat::F32);
	}
	if matches!(default, Some((SampleFormat::I16, channels)) if channels >= 2) {
		return Some(SampleFormat::I16);
	}
	if has(SampleFormat::I16) {
		return Some(SampleFormat::I16);
	}
	None
}

fn output_config(device: &cpal::Device) -> Option<(cpal::StreamConfig, SampleFormat)> {
	let default = device.default_output_config().ok();
	let supported: Vec<(SampleFormat, u16)> = device
		.supported_output_configs()
		.ok()
		.map(|configs| configs.map(|config| (config.sample_format(), config.channels())).collect())
		.unwrap_or_default();
	let default_pair = default.as_ref().map(|config| (config.sample_format(), config.channels()));
	let format = preferred_sample_format(default_pair, &supported)?;
	if let Some(config) = default.filter(|config| {
		config.sample_format() == format && config.channels() >= 2
	}) {
		return Some(stereo_stream_config(config));
	}
	first_supported(device, format)
}

fn stereo_stream_config(supported: cpal::SupportedStreamConfig) -> (cpal::StreamConfig, SampleFormat) {
	let format = supported.sample_format();
	let mut config = supported.config();
	config.channels = 2;
	(config, format)
}

fn first_supported(device: &cpal::Device, format: SampleFormat) -> Option<(cpal::StreamConfig, SampleFormat)> {
	device.supported_output_configs().ok()?.find_map(|range| {
		if range.channels() < 2 || range.sample_format() != format {
			return None;
		}
		Some(stereo_stream_config(range.with_max_sample_rate()))
	})
}

fn bind_output_stream(
	device: cpal::Device,
	config: cpal::StreamConfig,
	format: SampleFormat,
	scene: SpatialScene,
	rate: u32,
) -> Option<cpal::Stream> {
	match format {
		SampleFormat::F32 => {
			let mut mix = oddio::Reinhard::new(scene);
			device
				.build_output_stream(
					&config,
					move |data: &mut [f32], _| {
						let frames = oddio::frame_stereo(data);
						oddio::run(&mut mix, rate, frames);
					},
					 |_| {},
					None,
				)
				.ok()
		}
		SampleFormat::I16 => {
			let mut mix = oddio::Reinhard::new(scene);
			let mut scratch = Vec::<f32>::new();
			device
				.build_output_stream(
					&config,
					move |data: &mut [i16], _| {
						if scratch.len() < data.len() {
							scratch.resize(data.len(), 0.0);
						}
						let mixed = &mut scratch[..data.len()];
						oddio::run(&mut mix, rate, oddio::frame_stereo(mixed));
						for (dst, src) in data.iter_mut().zip(mixed.iter()) {
							*dst = (src * 32767.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
						}
					},
					 |_| {},
					None,
				)
				.ok()
		}
		_ => None,
	}
}

