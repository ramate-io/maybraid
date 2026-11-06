use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::packaged::PackagedAssetError;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds3 {
	pub min: [f32; 3],
	pub max: [f32; 3],
}

impl Bounds3 {
	pub fn validate(&self) -> Result<(), PackagedAssetError> {
		if (0..3).any(|i| {
			!self.min[i].is_finite()
				|| !self.max[i].is_finite()
				|| self.min[i] >= self.max[i]
				|| !(self.max[i] - self.min[i]).is_finite()
		}) {
			return Err(PackagedAssetError::invalid(
				"bounds must be finite and have positive extent",
			));
		}
		Ok(())
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlphaConvention {
	Straight,
	Premultiplied,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum EndBehavior {
	Hold,
	/// Fade during the final `seconds` of the authored playback, then remain transparent.
	Fade {
		seconds: f32,
	},
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlipbookView {
	pub layer: u32,
	/// Zero is +Z; positive azimuth rotates toward +X in local space.
	pub azimuth_degrees: f32,
	pub elevation_degrees: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlipbookManifest {
	pub schema_version: u32,
	pub image: String,
	pub atlas_size: [u32; 2],
	pub grid: [u32; 2],
	pub frame_size: [u32; 2],
	pub gutter_px: u32,
	pub frame_count: u32,
	pub fps: f32,
	pub bounds: Bounds3,
	pub pivot: [f32; 3],
	pub quad_center: [f32; 3],
	pub capture_size: [f32; 2],
	pub alpha_mode: AlphaConvention,
	pub end_behavior: EndBehavior,
	pub views: Vec<FlipbookView>,
}

impl FlipbookManifest {
	pub fn from_json(bytes: &[u8]) -> Result<Self, PackagedAssetError> {
		let manifest: Self = serde_json::from_slice(bytes)?;
		manifest.validate()?;
		Ok(manifest)
	}

	pub fn validate(&self) -> Result<(), PackagedAssetError> {
		ManifestImage::validate(self.schema_version, &self.image)?;
		self.bounds.validate()?;
		for axis in 0..2 {
			let stride =
				self.gutter_px.checked_mul(2).and_then(|g| self.frame_size[axis].checked_add(g));
			let extent = stride.and_then(|s| s.checked_mul(self.grid[axis]));
			if self.frame_size[axis] == 0
				|| self.grid[axis] == 0
				|| extent != Some(self.atlas_size[axis])
			{
				return Err(PackagedAssetError::invalid(
					"atlas size must equal grid * (frame size + 2 * gutter)",
				));
			}
		}
		if self.frame_count == 0
			|| self.grid[0].checked_mul(self.grid[1]).is_none_or(|n| self.frame_count > n)
		{
			return Err(PackagedAssetError::invalid(
				"frame count must fit the nonempty atlas grid",
			));
		}
		if !self.fps.is_finite() || self.fps <= 0.0 || !self.duration_seconds().is_finite() {
			return Err(PackagedAssetError::invalid(
				"fps must be positive; fps and playback duration must be finite",
			));
		}
		if let EndBehavior::Fade { seconds } = self.end_behavior {
			if !seconds.is_finite() || seconds <= 0.0 || seconds > self.duration_seconds() {
				return Err(PackagedAssetError::invalid(
					"fade seconds must be positive and no longer than playback",
				));
			}
		}
		if self.pivot.iter().chain(&self.quad_center).any(|v| !v.is_finite())
			|| self.capture_size.iter().any(|v| !v.is_finite() || *v <= 0.0)
		{
			return Err(PackagedAssetError::invalid(
				"capture transform must be finite with positive capture size",
			));
		}
		if self.views.is_empty() {
			return Err(PackagedAssetError::invalid("at least one capture view is required"));
		}
		for (i, view) in self.views.iter().enumerate() {
			if view.layer as usize != i
				|| !view.azimuth_degrees.is_finite()
				|| !(0.0..360.0).contains(&view.azimuth_degrees)
				|| !view.elevation_degrees.is_finite()
				|| !(-90.0..=90.0).contains(&view.elevation_degrees)
				|| self.views[..i].iter().any(|v| {
					v.azimuth_degrees == view.azimuth_degrees
						&& v.elevation_degrees == view.elevation_degrees
				}) {
				return Err(PackagedAssetError::invalid(
					"views need consecutive layers and distinct finite directions (azimuth [0,360), elevation [-90,90])",
				));
			}
		}
		Ok(())
	}

	/// Evenly spaced capture planes at one elevation, in consecutive azimuth order.
	pub fn azimuth_ring(&self) -> Option<AzimuthRing> {
		let first = self.views.first()?;
		let step = 360.0 / self.views.len() as f32;
		self.views.iter().enumerate().all(|(i, view)| {
			view.layer == i as u32
				&& (view.elevation_degrees - first.elevation_degrees).abs() < 1e-3
				&& ((view.azimuth_degrees - first.azimuth_degrees).rem_euclid(360.0)
					- i as f32 * step)
					.abs() < 1e-3
		})
		.then_some(AzimuthRing {
			elevation_radians: first.elevation_degrees.to_radians(),
			first_azimuth_radians: first.azimuth_degrees.to_radians(),
			view_count: self.views.len() as u32,
		})
	}

	/// Frame pair, interpolation, and end-behavior opacity for a playback age.
	pub fn clock_at(&self, age: f32) -> FlipbookClock {
		let frame = self.frame_at(age);
		FlipbookClock {
			frame_0: frame.floor(),
			frame_1: (frame.floor() + 1.0).min(self.frame_count.saturating_sub(1) as f32),
			fract: frame.fract(),
			opacity: self.opacity_at(age),
		}
	}

	/// Samples start at zero; the clip ends at the final captured timestamp.
	pub fn duration_seconds(&self) -> f32 {
		self.frame_count.saturating_sub(1) as f32 / self.fps
	}

	/// Continuous frame coordinate for interpolation, clamped to the last valid frame.
	pub fn frame_at(&self, seconds: f32) -> f32 {
		(seconds.max(0.0) * self.fps).clamp(0.0, self.frame_count.saturating_sub(1) as f32)
	}

	pub fn opacity_at(&self, seconds: f32) -> f32 {
		match self.end_behavior {
			EndBehavior::Hold => 1.0,
			EndBehavior::Fade { seconds: fade } => {
				((self.duration_seconds() - seconds.max(0.0)) / fade).clamp(0.0, 1.0)
			}
		}
	}

	/// UV offset.xy and scale.zw, sampling only the authored interior (not gutters).
	pub fn frame_uv(&self, frame: u32) -> Vec4 {
		let frame = frame.min(self.frame_count.saturating_sub(1));
		let tile = UVec2::new(frame % self.grid[0], frame / self.grid[0]);
		let stride = UVec2::from_array(self.frame_size) + UVec2::splat(2 * self.gutter_px);
		let size = Vec2::from_array(self.atlas_size.map(|v| v as f32));
		let offset = (tile * stride + UVec2::splat(self.gutter_px)).as_vec2() / size;
		let scale = Vec2::from_array(self.frame_size.map(|v| v as f32)) / size;
		Vec4::new(offset.x, offset.y, scale.x, scale.y)
	}

	pub fn nearest_view(&self, local_direction: Vec3) -> u32 {
		self.views
			.iter()
			.max_by(|a, b| {
				a.direction()
					.dot(local_direction)
					.total_cmp(&b.direction().dot(local_direction))
			})
			.map_or(0, |view| view.layer)
	}
}

/// Even azimuth ring used by the capture-plane flipbook renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AzimuthRing {
	pub elevation_radians: f32,
	pub first_azimuth_radians: f32,
	pub view_count: u32,
}

/// Material clock for one packaged flipbook age sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlipbookClock {
	pub frame_0: f32,
	pub frame_1: f32,
	pub fract: f32,
	pub opacity: f32,
}

impl FlipbookClock {
	pub fn time(self) -> Vec4 {
		Vec4::new(self.frame_0, self.frame_1, self.fract, self.opacity)
	}
}

impl FlipbookView {
	pub fn direction(&self) -> Vec3 {
		let (az_sin, az_cos) = self.azimuth_degrees.to_radians().sin_cos();
		let (el_sin, el_cos) = self.elevation_degrees.to_radians().sin_cos();
		Vec3::new(az_sin * el_cos, el_sin, az_cos * el_cos)
	}
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VolumeFormat {
	R16Float,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeManifest {
	pub schema_version: u32,
	pub image: String,
	pub dimensions: [u32; 3],
	pub bounds: Bounds3,
	pub format: VolumeFormat,
	pub density_scale: f32,
	/// Column-major affine matrix mapping integer indices to local voxel centers.
	pub index_to_local: [f32; 16],
}

impl VolumeManifest {
	pub fn from_json(bytes: &[u8]) -> Result<Self, PackagedAssetError> {
		let manifest: Self = serde_json::from_slice(bytes)?;
		manifest.validate()?;
		Ok(manifest)
	}

	pub fn validate(&self) -> Result<(), PackagedAssetError> {
		ManifestImage::validate(self.schema_version, &self.image)?;
		self.bounds.validate()?;
		if self.dimensions.contains(&0) || self.byte_len().is_none() {
			return Err(PackagedAssetError::invalid(
				"volume dimensions must be positive without byte-size overflow",
			));
		}
		if !self.density_scale.is_finite() || self.density_scale <= 0.0 {
			return Err(PackagedAssetError::invalid("density scale must be finite and positive"));
		}
		let matrix = self.index_to_local_matrix();
		if !matrix.is_finite()
			|| !matrix.determinant().is_finite()
			|| matrix.determinant().abs() < f32::MIN_POSITIVE
			|| matrix.row(3) != Vec4::W
		{
			return Err(PackagedAssetError::invalid(
				"index_to_local must be finite, affine and invertible",
			));
		}
		let mut grid_min = Vec3::splat(f32::INFINITY);
		let mut grid_max = Vec3::splat(f32::NEG_INFINITY);
		for corner in 0..8 {
			let index = Vec3::from_array(std::array::from_fn(|axis| {
				if corner & (1 << axis) == 0 {
					-0.5
				} else {
					self.dimensions[axis] as f32 - 0.5
				}
			}));
			let point = matrix.transform_point3(index);
			grid_min = grid_min.min(point);
			grid_max = grid_max.max(point);
		}
		let min = Vec3::from_array(self.bounds.min);
		let max = Vec3::from_array(self.bounds.max);
		let tolerance = (max - min).max_element() * 1e-4 + 1e-6;
		if !grid_min.is_finite()
			|| !grid_max.is_finite()
			|| (grid_min - min).abs().max_element() > tolerance
			|| (grid_max - max).abs().max_element() > tolerance
		{
			return Err(PackagedAssetError::invalid(
				"bounds must enclose the transformed voxel edges (indices -0.5 to dimensions -0.5)",
			));
		}
		Ok(())
	}

	pub fn index_to_local_matrix(&self) -> Mat4 {
		Mat4::from_cols_array(&self.index_to_local)
	}

	pub fn byte_len(&self) -> Option<usize> {
		self.dimensions
			.iter()
			.try_fold(2usize, |bytes, n| bytes.checked_mul(*n as usize))
	}
}

pub(crate) struct ManifestImage;

impl ManifestImage {
	fn validate(version: u32, image: &str) -> Result<(), PackagedAssetError> {
		if version != 1 {
			return Err(PackagedAssetError::invalid(format!(
				"unsupported schema version {version}"
			)));
		}
		if !image.ends_with(".ktx2")
			|| image.contains(':')
			|| image.contains('#')
			|| image.contains('\\')
			|| std::path::Path::new(image).is_absolute()
			|| std::path::Path::new(image).components().any(|c| {
				!matches!(c, std::path::Component::Normal(_) | std::path::Component::CurDir)
			}) {
			return Err(PackagedAssetError::invalid(
				"image must be a manifest-relative KTX2 path without traversal",
			));
		}
		Ok(())
	}
}
