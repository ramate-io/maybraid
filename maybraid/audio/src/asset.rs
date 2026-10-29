//! [`AudioClip`] asset: mono WAV decoded once by the asset server.

use std::io::Cursor;
use std::sync::Arc;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use oddio::{Frames, Sample};
use thiserror::Error;

/// Decoded mono frames. Spatial sources must be authored mono.
#[derive(Asset, TypePath, Clone)]
pub struct AudioClip {
	pub frames: Arc<Frames<Sample>>,
}

#[derive(Debug, Error)]
pub enum AudioClipError {
	#[error("read failed: {0}")]
	Io(#[from] std::io::Error),
	#[error("not a WAV: {0}")]
	Wav(#[from] hound::Error),
	#[error("spatial clips must be mono; got {0} channels")]
	NotMono(u16),
	#[error("empty clip")]
	Empty,
	#[error("sample rate is 0")]
	BadRate,
}

#[derive(Default, TypePath)]
pub(crate) struct AudioClipLoader;

impl AssetLoader for AudioClipLoader {
	type Asset = AudioClip;
	type Settings = ();
	type Error = AudioClipError;

	async fn load(
		&self,
		reader: &mut dyn Reader,
		_settings: &Self::Settings,
		_load_context: &mut LoadContext<'_>,
	) -> Result<Self::Asset, Self::Error> {
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		Ok(AudioClip { frames: decode_wav_mono(&bytes)? })
	}

	fn extensions(&self) -> &[&str] {
		&["wav"]
	}
}

/// Decode a WAV to mono frames. Stereo is an error, not a downmix.
pub fn decode_wav_mono(bytes: &[u8]) -> Result<Arc<Frames<Sample>>, AudioClipError> {
	let mut reader = hound::WavReader::new(Cursor::new(bytes))?;
	let spec = reader.spec();
	if spec.sample_rate == 0 {
		return Err(AudioClipError::BadRate);
	}
	if spec.channels != 1 {
		return Err(AudioClipError::NotMono(spec.channels));
	}
	let pcm = read_pcm(&mut reader, spec)?;
	if pcm.is_empty() {
		return Err(AudioClipError::Empty);
	}
	Ok(Frames::from_slice(spec.sample_rate, &pcm))
}

fn read_pcm(
	reader: &mut hound::WavReader<Cursor<&[u8]>>,
	spec: hound::WavSpec,
) -> Result<Vec<f32>, AudioClipError> {
	match spec.sample_format {
		hound::SampleFormat::Float => {
			let mut out = Vec::with_capacity(reader.duration() as usize);
			for sample in reader.samples::<f32>() {
				out.push(sample?);
			}
			Ok(out)
		}
		hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
			let max = f32::from(i16::MAX);
			let mut out = Vec::with_capacity(reader.duration() as usize);
			for sample in reader.samples::<i16>() {
				out.push(f32::from(sample?) / max);
			}
			Ok(out)
		}
		hound::SampleFormat::Int => {
			let shift = 32u32.saturating_sub(u32::from(spec.bits_per_sample));
			let max = (i32::MAX as u32 >> shift) as f32;
			if max <= 0.0 {
				return Err(AudioClipError::Empty);
			}
			let mut out = Vec::with_capacity(reader.duration() as usize);
			for sample in reader.samples::<i32>() {
				out.push(sample? as f32 / max);
			}
			Ok(out)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use hound::{SampleFormat, WavSpec, WavWriter};
	use std::io::Cursor;

	fn wav_bytes(channels: u16, samples: &[i16]) -> Vec<u8> {
		let spec = WavSpec {
			channels,
			sample_rate: 8_000,
			bits_per_sample: 16,
			sample_format: SampleFormat::Int,
		};
		let mut cursor = Cursor::new(Vec::new());
		let mut writer = match WavWriter::new(&mut cursor, spec) {
			Ok(writer) => writer,
			Err(_) => return Vec::new(),
		};
		for sample in samples {
			if writer.write_sample(*sample).is_err() {
				return Vec::new();
			}
		}
		if writer.finalize().is_err() {
			return Vec::new();
		}
		cursor.into_inner()
	}

	#[test]
	fn mono_wav_loads() {
		let bytes = wav_bytes(1, &[0, 1_000, -1_000]);
		assert!(decode_wav_mono(&bytes).is_ok());
	}

	#[test]
	fn stereo_wav_is_rejected() {
		let bytes = wav_bytes(2, &[0, 0, 1_000, -1_000]);
		assert!(matches!(decode_wav_mono(&bytes), Err(AudioClipError::NotMono(2))));
	}
}
