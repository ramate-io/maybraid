//! CPAL stream lifecycle and the oddio [`SpatialScene`] control handle.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use oddio::{Sample, Seek, Signal, Spatial, SpatialOptions, SpatialScene};

/// Game-thread handle to the mixer. Dropping it stops the output thread.
#[derive(Resource)]
pub struct Audio {
	control: Mutex<oddio::SpatialSceneControl>,
	pub sample_rate: u32,
	listener: Mutex<ListenerMotion>,
	shutdown: Arc<AtomicBool>,
	last_error: Arc<Mutex<Option<String>>>,
	_thread: JoinHandle<()>,
}

pub(crate) struct ListenerMotion {
	pub last: Option<Vec3>,
	pub velocity: Vec3,
}

impl Drop for Audio {
	fn drop(&mut self) {
		self.shutdown.store(true, Ordering::Relaxed);
		self._thread.thread().unpark();
	}
}

impl Audio {
	pub fn start() -> Option<Self> {
		let host = cpal::default_host();
		let device = host.default_output_device()?;
		let (config, format) = output_config(&device)?;
		let rate = config.sample_rate.0;
		let (control, scene) = SpatialScene::new();
		let shutdown = Arc::new(AtomicBool::new(false));
		let last_error = Arc::new(Mutex::new(None));
		let thread_shutdown = shutdown.clone();
		let thread_error = last_error.clone();
		let thread = thread::Builder::new()
			.name("maybraid-audio".into())
			.spawn(move || {
				let Some(stream) =
					bind_output_stream(device, config, format, scene, rate, thread_error)
				else {
					return;
				};
				if stream.play().is_err() {
					return;
				}
				while !thread_shutdown.load(Ordering::Relaxed) {
					thread::park_timeout(Duration::from_millis(50));
				}
				drop(stream);
			})
			.ok()?;
		Some(Self {
			control: Mutex::new(control),
			sample_rate: rate,
			listener: Mutex::new(ListenerMotion { last: None, velocity: Vec3::ZERO }),
			shutdown,
			last_error,
			_thread: thread,
		})
	}

	pub fn last_error(&self) -> Option<String> {
		self.last_error.lock().ok().and_then(|slot| slot.clone())
	}

	pub fn update_listener(&self, listener: Vec3, dt: f32) -> Vec3 {
		let Ok(mut motion) = self.listener.lock() else {
			return Vec3::ZERO;
		};
		motion.velocity = motion
			.last
			.map(|prev| if dt > 1e-5 { (listener - prev) / dt } else { Vec3::ZERO })
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

	pub fn play_buffered<S: Signal<Frame = Sample> + Send + 'static>(
		&self,
		signal: S,
		options: SpatialOptions,
	) -> Option<Spatial> {
		let rate = self.sample_rate;
		self.control
			.lock()
			.ok()
			.map(|mut control| control.play_buffered(signal, options, 120.0, rate, 0.1))
	}

	pub fn play_seek<S: Seek<Frame = Sample> + Send + 'static>(
		&self,
		signal: S,
		options: SpatialOptions,
	) -> Option<Spatial> {
		self.control.lock().ok().map(|mut control| control.play(signal, options))
	}
}

/// Prefer F32 stereo, then I16 stereo. Only exact 2-channel configs.
pub fn preferred_sample_format(
	default: Option<(SampleFormat, u16)>,
	supported: &[(SampleFormat, u16)],
) -> Option<SampleFormat> {
	let has = |format: SampleFormat| {
		supported
			.iter()
			.any(|(candidate, channels)| *candidate == format && *channels == 2)
	};
	if matches!(default, Some((SampleFormat::F32, 2))) {
		return Some(SampleFormat::F32);
	}
	if has(SampleFormat::F32) {
		return Some(SampleFormat::F32);
	}
	if matches!(default, Some((SampleFormat::I16, 2))) {
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
	if let Some(config) =
		default.filter(|config| config.sample_format() == format && config.channels() == 2)
	{
		return Some((config.config(), format));
	}
	first_stereo(device, format)
}

fn first_stereo(
	device: &cpal::Device,
	format: SampleFormat,
) -> Option<(cpal::StreamConfig, SampleFormat)> {
	device.supported_output_configs().ok()?.find_map(|range| {
		if range.channels() != 2 || range.sample_format() != format {
			return None;
		}
		Some((range.with_max_sample_rate().config(), format))
	})
}

fn bind_output_stream(
	device: cpal::Device,
	config: cpal::StreamConfig,
	format: SampleFormat,
	scene: SpatialScene,
	rate: u32,
	last_error: Arc<Mutex<Option<String>>>,
) -> Option<cpal::Stream> {
	let on_err = {
		let last_error = last_error.clone();
		move |err: cpal::StreamError| {
			if let Ok(mut slot) = last_error.lock() {
				*slot = Some(err.to_string());
			}
			tracing::warn!(error = %err, "cpal output");
		}
	};
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
					on_err,
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
					on_err,
					None,
				)
				.ok()
		}
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rejects_six_channel_as_stereo() {
		assert!(preferred_sample_format(Some((SampleFormat::F32, 6)), &[(SampleFormat::F32, 6)])
			.is_none());
		assert!(matches!(
			preferred_sample_format(Some((SampleFormat::F32, 6)), &[(SampleFormat::F32, 2)]),
			Some(SampleFormat::F32)
		));
	}

	#[test]
	fn prefers_f32_stereo_then_i16() {
		assert!(matches!(
			preferred_sample_format(Some((SampleFormat::I16, 2)), &[(SampleFormat::F32, 2)]),
			Some(SampleFormat::F32)
		));
		assert!(matches!(
			preferred_sample_format(Some((SampleFormat::I16, 2)), &[]),
			Some(SampleFormat::I16)
		));
	}
}
