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

use crate::flow::ChainFlowStyle;
use crate::params::{is_mythical_chain_recipe, mythical_chain_palette, RECIPE_MYTHICAL_CHAIN};

const SCALAR_VEC4S: usize = MATERIAL_SCALAR_FLOATS / 4;

/// Persistent per-link damage slots packed as eight `vec4`s.
pub const MAX_CHAIN_LINKS: usize = 32;
const LINK_DAMAGE_VEC4S: usize = MAX_CHAIN_LINKS / 4;

/// Registers embedded **`chain_material.wgsl`** and [`MaterialPlugin`].
pub struct ChainMaterialPlugin;

impl Plugin for ChainMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "chain_material.wgsl");
		if !app.is_plugin_added::<MaterialPlugin<ChainMaterial>>() {
			app.add_plugins(MaterialPlugin::<ChainMaterial>::default());
		}
		crate::runtime::register_runtime_systems(app);
		crate::halo::register_halo_systems(app);
	}
}

/// Pipeline key so the additive halo gets its own blend / depth-write state.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChainMaterialKey {
	pub halo: bool,
}

impl From<&ChainMaterial> for ChainMaterialKey {
	fn from(material: &ChainMaterial) -> Self {
		Self { halo: material.params.is_halo() }
	}
}

/// Packed GPU representation of one mythical-chain [`MaterialRef`] plus runtime damage.
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct ChainMaterialUniform {
	pub colors: [Vec4; MATERIAL_PALETTE_SLOTS],
	/// `x` frequency, `y` amplitude, `z` seed, `w` octaves.
	pub noise: Vec4,
	pub scalars: [Vec4; SCALAR_VEC4S],
	pub rasters: [[Vec4; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS],
	/// `x` along-chain hit pos, `y` radius, `z` strength, `w` hit time (seconds).
	pub impact: Vec4,
	/// `x` halo pass (`0` opaque, `1` additive), `y` shell expansion metres.
	pub halo: Vec4,
	/// Packed per-link persistent damage. 8 × 4 = [`MAX_CHAIN_LINKS`].
	pub link_damage: [Vec4; LINK_DAMAGE_VEC4S],
}

impl Default for ChainMaterialUniform {
	fn default() -> Self {
		Self::from_material_ref(&MaterialRef::named(RECIPE_MYTHICAL_CHAIN))
	}
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
			impact: Vec4::ZERO,
			halo: Vec4::new(0.0, ChainFlowStyle::DEFAULT_HALO_WIDTH_M, 0.0, 0.0),
			link_damage: [Vec4::ZERO; LINK_DAMAGE_VEC4S],
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

	pub fn link_count(&self) -> f32 {
		self.scalars[1].x.max(1.0)
	}

	pub fn flow_speed_mps(&self) -> f32 {
		self.scalars[2].x
	}

	pub fn flow_wavelength_m(&self) -> f32 {
		self.scalars[2].y
	}

	pub fn flow_halo_width(&self) -> f32 {
		self.scalars[3].x.max(0.0)
	}

	pub fn flow_halo_gain(&self) -> f32 {
		self.scalars[3].y
	}

	pub fn is_halo(&self) -> bool {
		self.halo.x > 0.5
	}

	pub fn set_halo_pass(&mut self, halo: bool) {
		self.halo.x = if halo { 1.0 } else { 0.0 };
	}

	pub fn set_halo_expansion(&mut self, metres: f32) {
		self.halo.y = metres.max(0.0);
	}

	pub fn set_impact(&mut self, pos: f32, radius: f32, strength: f32, time: f32) {
		self.impact = Vec4::new(pos.clamp(0.0, 1.0), radius.max(1e-4), strength.max(0.0), time);
	}

	pub fn link_damage(&self, link_id: u32) -> f32 {
		if (link_id as usize) >= MAX_CHAIN_LINKS {
			return 0.0;
		}
		let packed = self.link_damage[(link_id as usize) / 4];
		match link_id % 4 {
			0 => packed.x,
			1 => packed.y,
			2 => packed.z,
			_ => packed.w,
		}
	}

	pub fn set_link_damage(&mut self, link_id: u32, value: f32) {
		if (link_id as usize) >= MAX_CHAIN_LINKS {
			return;
		}
		let value = value.clamp(0.0, 1.0);
		let slot = &mut self.link_damage[(link_id as usize) / 4];
		match link_id % 4 {
			0 => slot.x = value,
			1 => slot.y = value,
			2 => slot.z = value,
			_ => slot.w = value,
		}
	}

	pub fn set_link_damage_array(&mut self, values: &[f32; MAX_CHAIN_LINKS]) {
		for (i, value) in values.iter().enumerate() {
			self.set_link_damage(i as u32, *value);
		}
	}

	pub fn copy_link_damage_array(&self, out: &mut [f32; MAX_CHAIN_LINKS]) {
		for (i, slot) in out.iter_mut().enumerate() {
			*slot = self.link_damage(i as u32);
		}
	}

	/// Copy palette / noise / authored scalars. Runtime damage and halo pass stay.
	pub fn apply_authored(&mut self, authored: &Self) {
		self.colors = authored.colors;
		self.noise = authored.noise;
		self.scalars = authored.scalars;
		self.rasters = authored.rasters;
		self.halo.y = authored.flow_halo_width();
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(ChainMaterialKey)]
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
		if self.params.is_halo() {
			AlphaMode::Add
		} else {
			// Dissolve uses `discard` so holes stay UV-locked while the mesh bends.
			AlphaMode::Opaque
		}
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
		key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = None;
		if key.bind_group_data.halo {
			if let Some(depth) = descriptor.depth_stencil.as_mut() {
				depth.depth_write_enabled = Some(false);
			}
		}
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
	use crate::runtime::{apply_chain_hit, ChainRuntimeDamage};

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
		assert!(released.params.fade() > near.params.fade());
		assert!(released.params.fade() > damaged.params.fade());
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

	#[test]
	fn per_link_damage_array_updates_in_place() -> anyhow::Result<()> {
		let mut material = ChainMaterial::from_material_ref(&mythical_chain_ref());
		assert_eq!(material.params.link_damage(6), 0.0);
		apply_chain_hit(&mut material, 6, 0.4, 14.0, 1.25);
		apply_chain_hit(&mut material, 6, 0.3, 14.0, 2.0);
		assert!((material.params.link_damage(6) - 0.7).abs() < 1e-5);
		assert_eq!(material.params.link_damage(5), 0.0);
		assert!((material.params.impact.w - 2.0).abs() < 1e-5);
		assert!(material.params.impact.z > 0.5);
		let runtime = ChainRuntimeDamage::from_uniform(&material.params);
		assert!((runtime.link(6) - 0.7).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn apply_authored_keeps_runtime_damage() -> anyhow::Result<()> {
		let mut owned = ChainMaterial::from_material_ref(&mythical_chain_ref());
		apply_chain_hit(&mut owned, 3, 0.8, 14.0, 4.0);
		let authored =
			ChainMaterial::from_material_ref(&mythical_chain_ref_for(MythicalChainState::Damaged));
		owned.params.apply_authored(&authored.params);
		assert!((owned.params.intact() - MythicalChainState::Damaged.intact()).abs() < 1e-5);
		assert!((owned.params.link_damage(3) - 0.8).abs() < 1e-5);
		assert!(!owned.params.is_halo());
		Ok(())
	}

	#[test]
	fn packs_flow_style_from_the_recipe() -> anyhow::Result<()> {
		use crate::flow::ChainFlowStyle;
		let scalars = MythicalChainScalars::default()
			.with_flow(ChainFlowStyle::default().with_speed_mps(-0.8));
		let material = ChainMaterial::from_material_ref(&mythical_chain_ref_with(scalars, 11));
		assert!((material.params.flow_speed_mps() - (-0.8)).abs() < 1e-5);
		assert!(material.params.flow_wavelength_m() > 0.0);
		assert!(material.params.flow_halo_width() > 0.0);
		Ok(())
	}
}
