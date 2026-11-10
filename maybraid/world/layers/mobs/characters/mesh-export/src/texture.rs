//! Solid PBR maps for Procreate OBJ import (`map_Kd` / `map_metallic` / `map_roughness`).

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::export::MeshGroup;

/// 2048² matches Procreate's documented 2K painting size and fits consumer iPads.
/// All maps for a material use this size — Procreate picks painting resolution from
/// the imported images and requires every map of a material to match.
pub const TEXTURE_RESOLUTION: u32 = 2048;

#[derive(Debug, Clone)]
pub struct MaterialTextures {
	pub color_file: String,
	pub metallic_file: String,
	pub roughness_file: String,
}

impl MaterialTextures {
	pub fn for_stem(stem: &str, material: &str) -> Self {
		Self {
			color_file: format!("{stem}_{material}_color.png"),
			metallic_file: format!("{stem}_metallic.png"),
			roughness_file: format!("{stem}_roughness.png"),
		}
	}
}

pub struct TextureSet;

impl TextureSet {
	pub fn write_for_groups(
		dir: &Path,
		stem: &str,
		groups: &[MeshGroup],
	) -> anyhow::Result<Vec<MaterialTextures>> {
		std::fs::create_dir_all(dir)?;
		let metallic = format!("{stem}_metallic.png");
		let roughness = format!("{stem}_roughness.png");
		write_solid_png(&dir.join(&metallic), [0, 0, 0])?;
		write_solid_png(&dir.join(&roughness), [166, 166, 166])?;

		let mut maps = Vec::new();
		for group in groups {
			let color_file = format!("{stem}_{}_color.png", group.material_name);
			let rgb = [
				(group.diffuse[0].clamp(0.0, 1.0) * 255.0).round() as u8,
				(group.diffuse[1].clamp(0.0, 1.0) * 255.0).round() as u8,
				(group.diffuse[2].clamp(0.0, 1.0) * 255.0).round() as u8,
			];
			write_solid_png(&dir.join(&color_file), rgb)?;
			maps.push(MaterialTextures {
				color_file,
				metallic_file: metallic.clone(),
				roughness_file: roughness.clone(),
			});
		}
		Ok(maps)
	}
}

pub fn write_solid_png(path: &Path, rgb: [u8; 3]) -> anyhow::Result<()> {
	let width = TEXTURE_RESOLUTION;
	let height = TEXTURE_RESOLUTION;
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn solid_png_is_2048_rgba() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let path = dir.path().join("color.png");
		write_solid_png(&path, [10, 20, 30])?;
		let data = std::fs::read(&path)?;
		let decoder = png::Decoder::new(std::io::Cursor::new(data));
		let reader = decoder.read_info()?;
		let info = reader.info();
		if info.width != TEXTURE_RESOLUTION || info.height != TEXTURE_RESOLUTION {
			return Err(anyhow::anyhow!("size {}x{}", info.width, info.height));
		}
		Ok(())
	}
}
