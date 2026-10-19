//! Read-only, deliberately versioned Houdini contract and bounded animation storage.

use std::fs;
use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::render::render_resource::{
	Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

use crate::dust::bounds::DustBounds;
use crate::dust::playback::DustCaptureBasis;

#[derive(Debug, thiserror::Error)]
pub enum DustAssetError {
	#[error(transparent)]
	Io(#[from] std::io::Error),
	#[error(transparent)]
	Json(#[from] serde_json::Error),
	#[error("{0}")]
	Contract(String),
	#[error("{0}")]
	Image(String),
}

impl DustAssetError {
	fn contract(message: impl Into<String>) -> Self {
		Self::Contract(message.into())
	}
}

/// On-disk v1 density/atlas bundle (`wind_dust_demos`).
pub struct DustBundle {
	pub root: PathBuf,
	pub manifest: serde_json::Value,
}

impl DustBundle {
	pub fn showcase_data_root() -> PathBuf {
		PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../showcase/data")
	}

	pub fn read(root: PathBuf) -> Result<Self, DustAssetError> {
		let manifest: serde_json::Value =
			serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
		let bundle = Self { root, manifest };
		bundle.validate()?;
		Ok(bundle)
	}

	pub fn multiview_root(&self) -> PathBuf {
		self.root.with_file_name("wind_dust_multiview")
	}

	pub fn validate(&self) -> Result<(), DustAssetError> {
		let m = &self.manifest;
		for (actual, expected) in [
			(&m["C"]["resolution_xyz"], serde_json::json!([64, 32, 64])),
			(&m["space"]["bounds_min_m"], serde_json::json!([-3.2, 0, -3.2])),
			(&m["space"]["bounds_max_m"], serde_json::json!([3.2, 3.2, 3.2])),
			(&m["B"]["atlas_size_px"], serde_json::json!([1848, 1848])),
			(&m["B"]["tiles_xy"], serde_json::json!([7, 7])),
			(&m["B"]["padding_each_edge_px"], serde_json::json!(4)),
			(&m["C"]["consistent_scale"], serde_json::json!(1.0)),
		] {
			if actual != &expected {
				return Err(DustAssetError::contract(format!(
					"Unsupported v1 contract: {actual} != {expected}"
				)));
			}
		}
		let timestamps = m["shared_source"]["timestamps_s"]
			.as_array()
			.ok_or_else(|| DustAssetError::contract("missing timestamps"))?;
		if timestamps.len() != DustBounds::FRAME_COUNT as usize
			|| m["B"]["timestamps_s"] != m["C"]["timestamps_s"]
			|| m["B"]["timestamps_s"].as_array() != Some(timestamps)
		{
			return Err(DustAssetError::contract("B/C/source timestamps differ"));
		}
		for (i, t) in timestamps.iter().enumerate() {
			let value = t.as_f64().ok_or_else(|| DustAssetError::contract("invalid timestamp"))?;
			if (value - i as f64 / DustBounds::FRAME_RATE as f64).abs() > 1e-8 {
				return Err(DustAssetError::contract("unsupported nonuniform timestamps"));
			}
		}
		Ok(())
	}

	pub fn capture_basis(&self) -> Result<DustCaptureBasis, DustAssetError> {
		Ok(DustCaptureBasis {
			right: self.basis("image_right")?,
			up: self.basis("image_up")?,
			centre: self.basis("quad_centre_m")?,
		})
	}

	pub fn atlas(&self) -> Result<Image, DustAssetError> {
		let bytes = fs::read(
			self.root.join(
				self.manifest["B"]["file"]
					.as_str()
					.ok_or_else(|| DustAssetError::contract("atlas path"))?,
			),
		)?;
		let image = Image::from_buffer(
			&bytes,
			ImageType::Extension("png"),
			CompressedImageFormats::NONE,
			true,
			ImageSampler::linear(),
			RenderAssetUsages::RENDER_WORLD,
		)
		.map_err(|error| DustAssetError::Image(error.to_string()))?;
		if image.texture_descriptor.size.width != 1848
			|| image.texture_descriptor.size.height != 1848
			|| image.texture_descriptor.format != TextureFormat::Rgba8UnormSrgb
		{
			return Err(DustAssetError::contract("unexpected atlas dimensions/format"));
		}
		Ok(image)
	}

	/// Pack the *bounded* 49-frame sequence into 7x7 spatial cells of one 3D R16F texture.
	/// Each cell remains a true 64x32x64 volume. The shader clamps to cell voxel centres.
	/// This trades 12.25 MiB GPU residency for zero playback uploads/decodes/allocations.
	pub fn density(&self, markers: bool) -> Result<Image, DustAssetError> {
		let mut packed = vec![0u8; 448 * 224 * 64 * 2];
		for i in 0..DustBounds::FRAME_COUNT as usize {
			let path = if markers {
				self.root.join("diagnostics/axis_markers.f16")
			} else {
				self.root.join(format!("C_density/frames/density_{i:03}.f16"))
			};
			let bytes = Self::read_frame(&path)?;
			for z in 0..64usize {
				for y in 0..32usize {
					let from = (z * 32 + y) * 64 * 2;
					let to = ((z * 224 + (i / 7) * 32 + y) * 448 + (i % 7) * 64) * 2;
					packed[to..to + 128].copy_from_slice(&bytes[from..from + 128]);
				}
			}
		}
		let mut image = Image::new(
			Extent3d { width: 448, height: 224, depth_or_array_layers: 64 },
			TextureDimension::D3,
			packed,
			TextureFormat::R16Float,
			RenderAssetUsages::RENDER_WORLD,
		);
		image.sampler = ImageSampler::linear();
		Ok(image)
	}

	pub fn unused_atlas() -> Image {
		Image::default()
	}

	pub fn unused_density() -> Image {
		Image::new(
			Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
			TextureDimension::D3,
			vec![0, 0],
			TextureFormat::R16Float,
			RenderAssetUsages::RENDER_WORLD,
		)
	}

	pub fn read_frame(path: &Path) -> Result<Vec<u8>, DustAssetError> {
		let data = fs::read(path)?;
		if data.len() != 262144 {
			return Err(DustAssetError::contract(format!("wrong frame size: {}", path.display())));
		}
		for pair in data.chunks_exact(2) {
			let bits = u16::from_le_bytes([pair[0], pair[1]]);
			if bits & 0x8000 != 0 || bits & 0x7c00 == 0x7c00 {
				return Err(DustAssetError::contract("negative/nonfinite density"));
			}
		}
		Ok(data)
	}

	pub fn basis(&self, key: &str) -> Result<Vec3, DustAssetError> {
		let a = &self.manifest["B"]["capture"][key];
		Ok(Vec3::new(
			a[0].as_f64().ok_or_else(|| DustAssetError::contract("basis x"))? as f32,
			a[1].as_f64().ok_or_else(|| DustAssetError::contract("basis y"))? as f32,
			a[2].as_f64().ok_or_else(|| DustAssetError::contract("basis z"))? as f32,
		))
	}
}

/// The bounded eight-view v1 experiment: one 952px atlas per layer, never rewritten during playback.
pub struct DustMultiView;

impl DustMultiView {
	pub const GPU_BYTES: usize = 952 * 952 * 8 * 4;
	pub const VIEW_COUNT: u32 = 8;
	pub const ELEVATION_DEGREES: f32 = 20.0;

	pub fn read(root: &Path, source: &DustBundle) -> Result<Image, DustAssetError> {
		let manifest: serde_json::Value =
			serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
		if manifest["contract"] != "Maybraid B+ eight-view baked dust v1"
			|| manifest["shared_source"]["timestamps_s"]
				!= source.manifest["shared_source"]["timestamps_s"]
			|| manifest["B_plus"]["timestamps_s"]
				!= source.manifest["shared_source"]["timestamps_s"]
			|| manifest["space"] != source.manifest["space"]
		{
			return Err(DustAssetError::contract(
				"B+ source time/space contract differs from matched C",
			));
		}
		for (key, expected) in [
			("view_count", serde_json::json!(8)),
			("atlas_size_px", serde_json::json!([952, 952])),
			("tiles_xy", serde_json::json!([7, 7])),
			("frame_tile_interior_px", serde_json::json!([128, 128])),
			("padding_each_edge_px", serde_json::json!(4)),
			("tile_pitch_px", serde_json::json!([136, 136])),
			("elevation_degrees", serde_json::json!(20)),
			("capture_width_height_m", serde_json::json!([7.8, 7.8])),
			("quad_centre_m", serde_json::json!([0., 0.55, 0.])),
			("pivot_m", serde_json::json!([0, 0, 0])),
		] {
			if manifest["B_plus"][key] != expected {
				return Err(DustAssetError::contract(format!("Unsupported B+ v1 {key}")));
			}
		}
		let views = manifest["B_plus"]["views"]
			.as_array()
			.ok_or_else(|| DustAssetError::contract("Missing B+ camera views"))?;
		if views.len() != Self::VIEW_COUNT as usize {
			return Err(DustAssetError::contract("Wrong view count"));
		}
		for (i, view) in views.iter().enumerate() {
			if view["index"] != i
				|| view["azimuth_degrees"] != i * 45
				|| view["elevation_degrees"] != 20
			{
				return Err(DustAssetError::contract("Unsupported B+ azimuth ring"));
			}
			let theta = i as f32 * std::f32::consts::FRAC_PI_4;
			let e = Self::ELEVATION_DEGREES.to_radians();
			for (key, expected) in [
				("image_right", Vec3::new(theta.cos(), 0., -theta.sin())),
				("image_up", Vec3::new(-e.sin() * theta.sin(), e.cos(), -e.sin() * theta.cos())),
				("camera_target_m", DustBounds::QUAD_CENTRE),
			] {
				for axis in 0..3 {
					if (view[key][axis]
						.as_f64()
						.ok_or_else(|| DustAssetError::contract("Invalid B+ camera basis"))?
						as f32 - expected[axis])
						.abs() > 0.000001
					{
						return Err(DustAssetError::contract(
							"B+ camera basis does not match shader",
						));
					}
				}
			}
		}
		let mut packed = Vec::with_capacity(Self::GPU_BYTES);
		for view in 0..Self::VIEW_COUNT {
			let bytes = fs::read(root.join(format!("atlases/view_{view:02}.png")))?;
			let image = Image::from_buffer(
				&bytes,
				ImageType::Extension("png"),
				CompressedImageFormats::NONE,
				true,
				ImageSampler::linear(),
				RenderAssetUsages::RENDER_WORLD,
			)
			.map_err(|error| DustAssetError::Image(error.to_string()))?;
			if image.texture_descriptor.size.width != 952
				|| image.texture_descriptor.size.height != 952
				|| image.texture_descriptor.format != TextureFormat::Rgba8UnormSrgb
			{
				return Err(DustAssetError::contract("Unsupported B+ v1 atlas format/dimensions"));
			}
			packed.extend(
				image
					.data
					.ok_or_else(|| DustAssetError::contract("missing multiview image data"))?,
			);
		}
		if packed.len() != Self::GPU_BYTES {
			return Err(DustAssetError::contract("B+ image byte count mismatch"));
		}
		Ok(Self::array_image(952, 952, Self::VIEW_COUNT, packed))
	}

	pub fn fallback() -> Image {
		Self::array_image(1, 1, 1, vec![0, 0, 0, 0])
	}

	fn array_image(width: u32, height: u32, layers: u32, data: Vec<u8>) -> Image {
		let mut image = Image::new(
			Extent3d { width, height, depth_or_array_layers: layers },
			TextureDimension::D2,
			data,
			TextureFormat::Rgba8UnormSrgb,
			RenderAssetUsages::RENDER_WORLD,
		);
		image.texture_view_descriptor = Some(TextureViewDescriptor {
			dimension: Some(TextureViewDimension::D2Array),
			..Default::default()
		});
		image.sampler = ImageSampler::linear();
		image
	}

	/// Azimuth is atan2(camera.x,camera.z), with a cyclic seam at +Z.
	pub fn views(azimuth: f32, nearest: bool) -> (u32, u32, f32) {
		let x = azimuth.rem_euclid(std::f32::consts::TAU) / std::f32::consts::FRAC_PI_4;
		if nearest {
			let i = (x.round() as u32) % Self::VIEW_COUNT;
			(i, i, 0.)
		} else {
			let i = x.floor() as u32 % Self::VIEW_COUNT;
			(i, (i + 1) % Self::VIEW_COUNT, x - x.floor())
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn matched_contract_and_axis_layout() -> Result<(), DustAssetError> {
		let bundle = DustBundle::read(DustBundle::showcase_data_root().join("wind_dust_demos"))?;
		let data = DustBundle::read_frame(&bundle.root.join("diagnostics/axis_markers.f16"))?;
		for ((x, y, z), expected) in
			[((50, 6, 9), 0x3400u16), ((10, 24, 20), 0x3800), ((25, 12, 50), 0x3a00)]
		{
			let n = 2 * ((z * 32 + y) * 64 + x);
			assert_eq!(u16::from_le_bytes([data[n], data[n + 1]]), expected);
		}
		let image = bundle.density(false)?;
		assert_eq!(
			image
				.data
				.as_ref()
				.ok_or_else(|| DustAssetError::contract("missing data"))?
				.len(),
			12_845_056
		);
		Ok(())
	}
}

#[cfg(test)]
mod multiview_tests {
	use super::*;

	#[test]
	fn actual_eight_atlases_match_source_and_pack_layers() -> Result<(), DustAssetError> {
		let root = DustBundle::showcase_data_root();
		let source = DustBundle::read(root.join("wind_dust_demos"))?;
		let image = DustMultiView::read(&root.join("wind_dust_multiview"), &source)?;
		assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 8);
		assert_eq!(
			image
				.data
				.as_ref()
				.ok_or_else(|| DustAssetError::contract("missing data"))?
				.len(),
			DustMultiView::GPU_BYTES
		);
		assert_eq!(
			image
				.texture_view_descriptor
				.as_ref()
				.ok_or_else(|| DustAssetError::contract("missing view"))?
				.dimension,
			Some(TextureViewDimension::D2Array)
		);
		Ok(())
	}

	#[test]
	fn cyclic_selection_preserves_seam_and_nearest() {
		let (a, b, w) = DustMultiView::views(-std::f32::consts::FRAC_PI_4 * 0.25, false);
		assert_eq!((a, b), (7, 0));
		assert!((w - 0.75).abs() < 0.00001);
		assert_eq!(DustMultiView::views(std::f32::consts::TAU, false), (0, 1, 0.));
		assert_eq!(DustMultiView::views(std::f32::consts::FRAC_PI_4 * 0.6, true), (1, 1, 0.));
	}

	#[test]
	fn capture_bases_have_fixed_pivot_scale_and_orthonormal_ring() {
		let elevation = DustMultiView::ELEVATION_DEGREES.to_radians();
		for i in 0..DustMultiView::VIEW_COUNT {
			let theta = i as f32 * std::f32::consts::FRAC_PI_4;
			let right = Vec3::new(theta.cos(), 0., -theta.sin());
			let up = Vec3::new(
				-elevation.sin() * theta.sin(),
				elevation.cos(),
				-elevation.sin() * theta.cos(),
			);
			assert!(right.dot(up).abs() < 0.000001);
			assert!((right.cross(up).length() - 1.).abs() < 0.000001);
		}
		assert_eq!(DustMultiView::GPU_BYTES, 29_001_728);
	}
}
