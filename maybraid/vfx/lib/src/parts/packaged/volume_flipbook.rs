//! Animated scalar density on a fixed voxel grid. Frames are ordinary 3D KTX2 assets.
use crate::parts::packaged::{
	Bounds3, EndBehavior, FlipbookClock, PackagedAssetError, VolumeFormat, VolumeManifest,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const MAX_VOLUME_FRAMES: usize = 64;
pub const MAX_VOLUME_AXIS: u32 = 128;
pub const MAX_VOLUME_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeFlipbookManifest {
	pub schema_version: u32,
	/// Manifest-relative R16Float KTX2 paths, in time order. X is the fastest voxel axis.
	pub frames: Vec<String>,
	pub dimensions: [u32; 3],
	pub bounds: Bounds3,
	pub format: VolumeFormat,
	/// Extinction per authored local-space unit, multiplied by sampled density.
	pub density_scale: f32,
	pub index_to_local: [f32; 16],
	pub fps: f32,
	pub end_behavior: EndBehavior,
}

impl VolumeFlipbookManifest {
	pub fn from_json(bytes: &[u8]) -> Result<Self, PackagedAssetError> {
		let manifest: Self = serde_json::from_slice(bytes)?;
		manifest.validate()?;
		Ok(manifest)
	}

	pub fn validate(&self) -> Result<(), PackagedAssetError> {
		// The pinned ImageLoader selects a D3 texture only when depth > 1.
		if !(2..=MAX_VOLUME_FRAMES).contains(&self.frames.len())
			|| self.dimensions.iter().any(|n| *n < 2 || *n > MAX_VOLUME_AXIS)
			|| self.byte_len().is_none_or(|bytes| bytes > MAX_VOLUME_BYTES)
		{
			return Err(PackagedAssetError::invalid("volume flipbooks require 2..64 frames, axes 2..128, and at most 128 MiB decoded density"));
		}
		for frame in &self.frames {
			self.frame_manifest(frame).validate()?;
		}
		if !self.fps.is_finite() || self.fps <= 0. || !self.duration_seconds().is_finite() {
			return Err(PackagedAssetError::invalid(
				"volume fps and duration must be finite and positive",
			));
		}
		if let EndBehavior::Fade { seconds } = self.end_behavior {
			if !seconds.is_finite() || seconds <= 0. || seconds > self.duration_seconds() {
				return Err(PackagedAssetError::invalid(
					"volume fade must be positive and no longer than playback",
				));
			}
		}
		if !self.local_to_texture().is_finite() {
			return Err(PackagedAssetError::invalid(
				"volume local-to-texture matrix must be finite",
			));
		}
		Ok(())
	}

	pub(crate) fn frame_manifest(&self, image: &str) -> VolumeManifest {
		VolumeManifest {
			schema_version: self.schema_version,
			image: image.into(),
			dimensions: self.dimensions,
			bounds: self.bounds,
			format: self.format,
			density_scale: self.density_scale,
			index_to_local: self.index_to_local,
		}
	}

	pub fn byte_len(&self) -> Option<usize> {
		self.dimensions
			.iter()
			.try_fold(self.frames.len().checked_mul(2)?, |n, axis| n.checked_mul(*axis as usize))
	}

	pub fn duration_seconds(&self) -> f32 {
		self.frames.len().saturating_sub(1) as f32 / self.fps
	}

	pub fn clock_at(&self, age: f32) -> FlipbookClock {
		let last = self.frames.len().saturating_sub(1) as f32;
		let frame = (age.max(0.) * self.fps).clamp(0., last);
		let opacity = match self.end_behavior {
			EndBehavior::Hold => 1.,
			EndBehavior::Fade { seconds } => {
				((self.duration_seconds() - age.max(0.)) / seconds).clamp(0., 1.)
			}
		};
		FlipbookClock {
			frame_0: frame.floor(),
			frame_1: (frame.floor() + 1.).min(last),
			fract: frame.fract(),
			opacity,
		}
	}

	/// Integer voxel centers map to (index + 0.5) / dimensions; supports rotated grids.
	pub fn local_to_texture(&self) -> Mat4 {
		Mat4::from_scale(Vec3::from_array(self.dimensions.map(|n| 1. / n as f32)))
			* Mat4::from_translation(Vec3::splat(0.5))
			* Mat4::from_cols_array(&self.index_to_local).inverse()
	}
}

#[derive(Asset, TypePath, Clone, Debug)]
pub struct PackagedVolumeFlipbook {
	pub manifest: VolumeFlipbookManifest,
	#[dependency]
	pub frames: Vec<Handle<Image>>,
}
