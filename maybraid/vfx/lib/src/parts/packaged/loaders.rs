use bevy::asset::{io::Reader, AssetLoader, LoadContext};
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;

use crate::parts::packaged::{
	AlphaConvention, FlipbookManifest, PackagedAssetError, PackagedFlipbook, StaticVolume,
	VolumeManifest,
};

#[derive(Default, TypePath)]
pub(crate) struct FlipbookLoader;

impl AssetLoader for FlipbookLoader {
	type Asset = PackagedFlipbook;
	type Settings = ();
	type Error = PackagedAssetError;

	async fn load(
		&self,
		reader: &mut dyn Reader,
		_settings: &(),
		context: &mut LoadContext<'_>,
	) -> Result<Self::Asset, Self::Error> {
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		let manifest = FlipbookManifest::from_json(&bytes)?;
		let path = context
			.path()
			.resolve_embed_str(&manifest.image)
			.map_err(|error| PackagedAssetError::invalid(error.to_string()))?;
		// Read through LoadContext so hot reload tracks changes to the source image.
		// Header checks avoid decoding/transcoding pixels twice; Bevy's ImageLoader owns
		// payload validation and its failure propagates through recursive load state.
		let bytes = context.read_asset_bytes(path.clone()).await?;
		KtxHeader::parse(&bytes)?.validate_flipbook(&manifest)?;
		let image = context
			.load_builder()
			.with_settings(|settings: &mut ImageLoaderSettings| {
				settings.sampler = KtxHeader::sampler();
				settings.is_srgb = true;
			})
			.load(path);
		Ok(PackagedFlipbook { manifest, image })
	}

	fn extensions(&self) -> &[&str] {
		&["flipbook.json"]
	}
}

#[derive(Default, TypePath)]
pub(crate) struct VolumeLoader;

impl AssetLoader for VolumeLoader {
	type Asset = StaticVolume;
	type Settings = ();
	type Error = PackagedAssetError;

	async fn load(
		&self,
		reader: &mut dyn Reader,
		_settings: &(),
		context: &mut LoadContext<'_>,
	) -> Result<Self::Asset, Self::Error> {
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		let manifest = VolumeManifest::from_json(&bytes)?;
		let path = context
			.path()
			.resolve_embed_str(&manifest.image)
			.map_err(|error| PackagedAssetError::invalid(error.to_string()))?;
		let bytes = context.read_asset_bytes(path.clone()).await?;
		KtxHeader::parse(&bytes)?.validate_volume(&manifest)?;
		let image = context
			.load_builder()
			.with_settings(|settings: &mut ImageLoaderSettings| {
				settings.sampler = KtxHeader::sampler();
				settings.is_srgb = false;
			})
			.load(path);
		Ok(StaticVolume { manifest, image })
	}

	fn extensions(&self) -> &[&str] {
		&["volume.json"]
	}
}

/// Only container layout is inspected here; ImageLoader remains the pixel decoder.
pub(crate) struct KtxHeader {
	format: u32,
	dimensions: [u32; 3],
	layers: u32,
	faces: u32,
	color_model: u8,
	transfer_function: u8,
	alpha_premultiplied: bool,
}

impl KtxHeader {
	fn sampler() -> ImageSampler {
		ImageSampler::Descriptor(ImageSamplerDescriptor {
			address_mode_u: ImageAddressMode::ClampToEdge,
			address_mode_v: ImageAddressMode::ClampToEdge,
			address_mode_w: ImageAddressMode::ClampToEdge,
			lod_max_clamp: 0.0,
			..ImageSamplerDescriptor::linear()
		})
	}

	pub(crate) fn parse(bytes: &[u8]) -> Result<Self, PackagedAssetError> {
		if bytes.len() < 104 || bytes[..12] != *b"\xabKTX 20\xbb\r\n\x1a\n" {
			return Err(PackagedAssetError::invalid(
				"image must have a complete KTX2 header and level index",
			));
		}
		let word = |offset: usize| {
			u32::from_le_bytes([
				bytes[offset],
				bytes[offset + 1],
				bytes[offset + 2],
				bytes[offset + 3],
			])
		};
		let levels = word(40) as usize;
		if levels == 0
			|| levels > 32
			|| 80 + levels * 24 > bytes.len()
			|| !matches!(word(44), 0 | 2)
		{
			return Err(PackagedAssetError::invalid(
				"KTX2 requires indexed levels and no compression or Zstd",
			));
		}
		for level in 0..levels {
			let index = 80 + level * 24;
			let offset = u64::from(word(index)) | (u64::from(word(index + 4)) << 32);
			let length = u64::from(word(index + 8)) | (u64::from(word(index + 12)) << 32);
			if length == 0
				|| offset < (80 + levels * 24) as u64
				|| offset.checked_add(length).is_none_or(|end| end > bytes.len() as u64)
			{
				return Err(PackagedAssetError::invalid(
					"KTX2 level payload is truncated or overlaps its index",
				));
			}
		}
		let dfd = word(48) as usize;
		let dfd_length = word(52) as usize;
		if dfd_length < 28 || dfd.checked_add(dfd_length).is_none_or(|end| end > bytes.len()) {
			return Err(PackagedAssetError::invalid(
				"KTX2 requires a complete data format descriptor",
			));
		}
		Ok(Self {
			format: word(12),
			dimensions: [word(20), word(24), word(28)],
			layers: word(32),
			faces: word(36),
			color_model: bytes[dfd + 12],
			transfer_function: bytes[dfd + 14],
			alpha_premultiplied: bytes[dfd + 15] & 1 != 0,
		})
	}

	pub(crate) fn validate_flipbook(
		&self,
		manifest: &FlipbookManifest,
	) -> Result<(), PackagedAssetError> {
		if self.dimensions != [manifest.atlas_size[0], manifest.atlas_size[1], 0]
			|| self.layers as usize != manifest.views.len() || self.faces != 1
			// Vulkan UNDEFINED + DFD UASTC, or RGBA8_SRGB + DFD RGBSDA.
			|| !matches!((self.format, self.color_model), (0, 166) | (43, 1))
			|| self.transfer_function != 2
			|| self.alpha_premultiplied != (manifest.alpha_mode == AlphaConvention::Premultiplied)
		{
			return Err(PackagedAssetError::invalid(format!(
				"{} must be an sRGB RGBA/UASTC 2D array matching atlas size, view count and alpha convention",
				manifest.image
			)));
		}
		Ok(())
	}

	pub(crate) fn validate_volume(
		&self,
		manifest: &VolumeManifest,
	) -> Result<(), PackagedAssetError> {
		// VK_FORMAT_R16_SFLOAT = 76. KTX2 3D textures have zero array layers.
		if self.dimensions != manifest.dimensions
			|| self.layers != 0
			|| self.faces != 1
			|| self.format != 76
			|| self.transfer_function != 1
		{
			return Err(PackagedAssetError::invalid(format!(
				"{} must be an R16Float 3D texture matching dimensions",
				manifest.image
			)));
		}
		Ok(())
	}
}
