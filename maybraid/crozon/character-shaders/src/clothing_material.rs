//! Clothing [`Material`] — [`MaterialRef`] bags plus a tiny vertex sway.

use bevy::{
	asset::embedded_asset,
	mesh::MeshVertexBufferLayoutRef,
	pbr::{MaterialPipeline, MaterialPipelineKey},
	prelude::*,
	reflect::TypePath,
	render::render_resource::{
		AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
	},
	shader::ShaderRef,
};
use material_ref::{
	MaterialId, MaterialRasters, MaterialRef, MATERIAL_PALETTE_SLOTS, MATERIAL_RASTER_CHANNELS,
	MATERIAL_RASTER_WIDTH, MATERIAL_SCALAR_FLOATS,
};

pub const KIND_CLOTH: u32 = 0;
pub const KIND_SPACE_SUIT: u32 = 1;
pub const KIND_TATTERED: u32 = 2;
pub const KIND_HAWAIIAN: u32 = 3;
pub const KIND_WIZARDS_VEINS: u32 = 4;
pub const KIND_GLITTER: u32 = 5;
pub const KIND_SCALES: u32 = 6;

const SCALAR_VEC4S: usize = MATERIAL_SCALAR_FLOATS / 4;
const DEFAULT_CLOTH_COLOR: Vec4 = Vec4::new(0.46, 0.60, 0.72, 1.0);

/// Registers embedded **`clothing_material.wgsl`** and [`MaterialPlugin`].
pub struct ClothingShaderMaterialPlugin;

impl Plugin for ClothingShaderMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "clothing_material.wgsl");
		app.add_plugins(MaterialPlugin::<ClothingShaderMaterial>::default());
	}
}

/// Look index packed into [`ClothingMaterialUniform::kind`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClothingShaderKind {
	Cloth,
	SpaceSuit,
	Tattered,
	Hawaiian,
	WizardsVeins,
	Glitter,
	Scales,
}

impl ClothingShaderKind {
	pub const fn as_u32(self) -> u32 {
		match self {
			Self::Cloth => KIND_CLOTH,
			Self::SpaceSuit => KIND_SPACE_SUIT,
			Self::Tattered => KIND_TATTERED,
			Self::Hawaiian => KIND_HAWAIIAN,
			Self::WizardsVeins => KIND_WIZARDS_VEINS,
			Self::Glitter => KIND_GLITTER,
			Self::Scales => KIND_SCALES,
		}
	}

	pub fn from_recipe_name(name: &str) -> Self {
		match name {
			"clothing_space_suit" => Self::SpaceSuit,
			"clothing_tattered" => Self::Tattered,
			"clothing_hawaiian" => Self::Hawaiian,
			"clothing_scales" => Self::Scales,
			"clothing_wizards_veins" => Self::WizardsVeins,
			"clothing_glitter" => Self::Glitter,
			_ => Self::Cloth,
		}
	}
}

/// Packed GPU representation of one clothing [`MaterialRef`].
///
/// Palette, noise, scalars, and rasters are the shared MaterialRef bags.
/// `kind` is the named-recipe contract (which look the shader runs).
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct ClothingMaterialUniform {
	pub colors: [Vec4; MATERIAL_PALETTE_SLOTS],
	/// `x` frequency, `y` amplitude, `z` seed.
	pub noise: Vec4,
	pub scalars: [Vec4; SCALAR_VEC4S],
	pub rasters: [[Vec4; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS],
	pub kind: u32,
	pub _pad: UVec3,
}

impl ClothingMaterialUniform {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let kind = match &material_ref.name {
			MaterialId::Name(name) => ClothingShaderKind::from_recipe_name(name),
			MaterialId::Default => ClothingShaderKind::Cloth,
		};

		let mut colors = [Vec4::ZERO; MATERIAL_PALETTE_SLOTS];
		if material_ref.palette.is_empty() {
			colors[0] = DEFAULT_CLOTH_COLOR;
		} else {
			for (slot, color) in colors.iter_mut().zip(&material_ref.palette) {
				let linear = LinearRgba::from(*color);
				*slot = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
			}
			let first = colors[0];
			for color in colors.iter_mut().skip(material_ref.palette.len()) {
				*color = first;
			}
		}

		let mut scalars = [Vec4::ZERO; SCALAR_VEC4S];
		let values = material_ref.scalar_values();
		for (i, slot) in scalars.iter_mut().enumerate() {
			let base = i * 4;
			*slot = Vec4::new(
				values.get(base).copied().unwrap_or(0.0),
				values.get(base + 1).copied().unwrap_or(0.0),
				values.get(base + 2).copied().unwrap_or(0.0),
				values.get(base + 3).copied().unwrap_or(0.0),
			);
		}

		let mut rasters = [[Vec4::ZERO; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS];
		for channel in 0..MATERIAL_RASTER_CHANNELS {
			let samples = material_ref.rasters.get_or(channel, 0.0);
			let rows = MaterialRasters::packed_rows(samples);
			rasters[channel] = rows.map(Vec4::from_array);
		}

		Self {
			colors,
			noise: Vec4::new(
				material_ref.noise.frequency.max(1e-6),
				material_ref.noise.amplitude,
				material_ref.noise.seed as f32,
				material_ref.noise.octaves as f32,
			),
			scalars,
			rasters,
			kind: kind.as_u32(),
			_pad: UVec3::ZERO,
		}
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ClothingShaderMaterial {
	#[uniform(0)]
	pub params: ClothingMaterialUniform,
}

impl ClothingShaderMaterial {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		Self { params: ClothingMaterialUniform::from_material_ref(material_ref) }
	}
}

impl Default for ClothingShaderMaterial {
	fn default() -> Self {
		Self::from_material_ref(&MaterialRef::named("clothing_cloth"))
	}
}

impl Material for ClothingShaderMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "clothing_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "clothing_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		// Tattered chews holes with fragment `discard` (Chico leaf cheese).
		// Opaque ignores alpha, so Mask would never punch through.
		AlphaMode::Opaque
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
	}

	fn enable_prepass() -> bool {
		false
	}

	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = None;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use material_ref::MATERIAL_RASTER_SAMPLES;

	use super::*;

	#[test]
	fn from_material_ref_packs_palette_scalars_rasters() {
		let material_ref = MaterialRef::named("clothing_tattered")
			.with_palette([Color::srgb(1.0, 0.0, 0.0)])
			.with_scalars([0.25, 0.5, 0.75])
			.with_raster(0, [1.0; MATERIAL_RASTER_SAMPLES]);
		let material = ClothingShaderMaterial::from_material_ref(&material_ref);
		assert_eq!(material.params.kind, KIND_TATTERED);
		assert!((material.params.colors[0].x - 1.0).abs() < 1e-5);
		assert_eq!(material.params.scalars[0], Vec4::new(0.25, 0.5, 0.75, 0.0));
		assert_eq!(material.params.rasters[0][0], Vec4::new(1.0, 1.0, 1.0, 0.0));
	}
}
