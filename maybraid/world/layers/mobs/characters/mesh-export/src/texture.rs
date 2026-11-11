//! Per-component color maps. Each independently paintable part gets its own PNG.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::export::MeshGroup;

/// Default painting resolution per component (Procreate 2K is optional; 1024 fits consumer iPads).
pub const DEFAULT_TEXTURE_RESOLUTION: u32 = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureConfig {
	pub resolution: u32,
}

impl Default for TextureConfig {
	fn default() -> Self {
		Self { resolution: DEFAULT_TEXTURE_RESOLUTION }
	}
}

impl TextureConfig {
	pub fn new(resolution: u32) -> Self {
		Self { resolution: resolution.max(1) }
	}

	pub fn write_solid_png(&self, path: &Path, rgb: [u8; 3]) -> anyhow::Result<()> {
		let width = self.resolution;
		let height = self.resolution;
		let mut pixels = vec![0_u8; (width as usize) * (height as usize) * 4];
		for pixel in pixels.chunks_exact_mut(4) {
			pixel[0] = rgb[0];
			pixel[1] = rgb[1];
			pixel[2] = rgb[2];
			pixel[3] = 255;
		}
		let file = File::create(path)?;
		let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
		encoder.set_color(png::ColorType::Rgba);
		encoder.set_depth(png::BitDepth::Eight);
		let mut writer = encoder.write_header()?;
		writer.write_image_data(&pixels)?;
		writer.finish()?;
		Ok(())
	}

	pub fn write_for_groups(
		&self,
		dir: &Path,
		groups: &[MeshGroup],
	) -> anyhow::Result<Vec<String>> {
		std::fs::create_dir_all(dir)?;
		let mut files = Vec::new();
		for group in groups {
			let file = group.color_file();
			self.write_solid_png(&dir.join(&file), group.diffuse_rgb())?;
			files.push(file);
		}
		Ok(files)
	}
}

/// Decoded solid (or near-solid) PNG for export audits.
#[derive(Debug, Clone)]
pub struct SolidPng {
	pub width: u32,
	pub height: u32,
	pub rgb: [u8; 3],
	pub uniform: bool,
}

impl SolidPng {
	pub fn load(path: &Path) -> anyhow::Result<Self> {
		let data = std::fs::read(path)?;
		let decoder = png::Decoder::new(std::io::Cursor::new(data));
		let mut reader = decoder.read_info()?;
		let mut buf = vec![0; reader.output_buffer_size()];
		let info = reader.next_frame(&mut buf)?;
		let width = info.width;
		let height = info.height;
		let channels = info.color_type.samples();
		if buf.len() < channels {
			return Err(anyhow::anyhow!("{}: empty PNG", path.display()));
		}
		let rgb =
			[buf[0], buf.get(1).copied().unwrap_or(buf[0]), buf.get(2).copied().unwrap_or(buf[0])];
		let mut uniform = true;
		for pixel in buf.chunks_exact(channels) {
			if pixel[0] != rgb[0]
				|| pixel.get(1).copied().unwrap_or(rgb[1]) != rgb[1]
				|| pixel.get(2).copied().unwrap_or(rgb[2]) != rgb[2]
			{
				uniform = false;
				break;
			}
		}
		Ok(Self { width, height, rgb, uniform })
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn solid_png_is_configured_rgba() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let path = dir.path().join("color.png");
		let config = TextureConfig::new(16);
		config.write_solid_png(&path, [10, 20, 30])?;
		let png = SolidPng::load(&path)?;
		if png.width != 16 || png.height != 16 || png.rgb != [10, 20, 30] || !png.uniform {
			return Err(anyhow::anyhow!("{png:?}"));
		}
		Ok(())
	}
}
