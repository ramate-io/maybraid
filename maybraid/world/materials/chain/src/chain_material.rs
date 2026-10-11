//! Mythical-chain [`Material`] — metal, energy veins, and durability fade.

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

use crate::params::{is_mythical_chain_recipe, mythical_chain_palette, RECIPE_MYTHICAL_CHAIN};

const SCALAR_VEC4S: usize = MATERIAL_SCALAR_FLOATS / 4;

/// Registers embedded **`chain_material.wgsl`** and [`MaterialPlugin`].
pub struct ChainMaterialPlugin;

impl Plugin for ChainMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "chain_material.wgsl");
		if !app.is_plugin_added::<MaterialPlugin<ChainMaterial>>() {
			app.add_plugins(MaterialPlugin::<ChainMaterial>::default());
		}
	}
}

/// Packed GPU representation of one mythical-chain [`MaterialRef`].
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct ChainMaterialUniform {
	pub colors: [Vec4; MATERIAL_PALETTE_SLOTS],
	/// `x` frequency, `y` amplitude, `z` seed, `w` octaves.
	pub noise: Vec4,
	pub scalars: [Vec4; SCALAR_VEC4S],
	pub rasters: [[Vec4; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS],
}

impl ChainMaterialUniform {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let defaults = mythical_chain_palette();
		let mut colors = [Vec4::ZERO; MATERIAL_PALETTE_SLOTS];
		for (index, slot) in colors.iter_mut().enumerate() {
			let color = material_ref
				.palette
				.get(index)
				.copied()
				.or_else(|| defaults.get(index).copied())
				.unwrap_or(Color::BLACK);
			let linear = LinearRgba::from(color);
			*slot = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
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
		for (channel, raster) in rasters.iter_mut().enumerate() {
			let samples = material_ref.rasters.get_or(channel, 0.0);
			let rows = MaterialRasters::packed_rows(samples);
			*raster = rows.map(Vec4::from_array);
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
		}
	}

	pub fn intact(&self) -> f32 {
		self.scalars[0].x
	}

	pub fn glow_intensity(&self) -> f32 {
		self.scalars[0].y
	}

	pub fn fade(&self) -> f32 {
		self.scalars[0].z
	}

	pub fn damage(&self) -> f32 {
		self.scalars[0].w
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ChainMaterial {
	#[uniform(0)]
	pub params: ChainMaterialUniform,
}

impl ChainMaterial {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		Self { params: ChainMaterialUniform::from_material_ref(material_ref) }
	}
}

impl Default for ChainMaterial {
	fn default() -> Self {
		Self::from_material_ref(&MaterialRef::named(RECIPE_MYTHICAL_CHAIN))
	}
}

impl Material for ChainMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "chain_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "chain_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		// Dissolve uses `discard` so holes stay UV-locked while the mesh bends.
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

/// Named recipes claimed by [`crate::ChainMaterialLib`].
pub fn claims_recipe(name: &str) -> bool {
	is_mythical_chain_recipe(name)
}

/// True when this [`MaterialRef`] is a mythical-chain recipe.
pub fn claims_material_ref(material_ref: &MaterialRef) -> bool {
	match &material_ref.name {
		MaterialId::Name(name) => claims_recipe(name),
		MaterialId::Default => false,
	}
}

#[cfg(test)]
mod tests {
	use material_ref::{hash_material_ref, MaterialRefKey};

	use super::*;
	use crate::params::{
		mythical_chain_ref, mythical_chain_ref_for, mythical_chain_ref_with, MythicalChainScalars,
		MythicalChainState,
	};

	#[test]
	fn packs_authored_intact_recipe() -> anyhow::Result<()> {
		let material = ChainMaterial::from_material_ref(&mythical_chain_ref());
		assert!((material.params.intact() - 1.0).abs() < 1e-5);
		assert!((material.params.fade() - 0.0).abs() < 1e-5);
		assert!(material.params.colors[1].z > material.params.colors[1].x);
		assert!(claims_recipe(RECIPE_MYTHICAL_CHAIN));
		assert!(!claims_recipe("laser_hex"));
		Ok(())
	}

	#[test]
	fn states_pack_distinct_scalars() -> anyhow::Result<()> {
		let intact =
			ChainMaterial::from_material_ref(&mythical_chain_ref_for(MythicalChainState::Intact));
		let damaged =
			ChainMaterial::from_material_ref(&mythical_chain_ref_for(MythicalChainState::Damaged));
		let near = ChainMaterial::from_material_ref(&mythical_chain_ref_for(
			MythicalChainState::NearBreaking,
		));
		let released =
			ChainMaterial::from_material_ref(&mythical_chain_ref_for(MythicalChainState::Released));
		assert!(intact.params.intact() > damaged.params.intact());
		assert!(damaged.params.intact() > near.params.intact());
		assert!(near.params.fade() > damaged.params.fade());
		assert!(released.params.fade() > near.params.fade());
		Ok(())
	}

	#[test]
	fn distinct_durability_is_a_distinct_material_key() -> anyhow::Result<()> {
		let left = mythical_chain_ref_for(MythicalChainState::Intact);
		let right = mythical_chain_ref_for(MythicalChainState::Damaged);
		assert_ne!(MaterialRefKey::from(&left), MaterialRefKey::from(&right));
		assert_ne!(hash_material_ref(&left), hash_material_ref(&right));
		let same_seed_other_chain = mythical_chain_ref_with(
			MythicalChainScalars::for_state(MythicalChainState::Intact),
			29,
		);
		assert_ne!(MaterialRefKey::from(&left), MaterialRefKey::from(&same_seed_other_chain));
		Ok(())
	}
}
