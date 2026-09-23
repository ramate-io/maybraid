//! White muzzle-flame cone, authored as a [`MaterialRef`] recipe.
//!
//! The shot resolves the handle immediately so the flash is visible the frame it
//! spawns. [`MuzzleFlameMaterialLib`] claims the same recipe when a world fulfill
//! restamps [`MaterialRefRoot`].

use bevy::asset::embedded_asset;
use bevy::ecs::system::SystemParam;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
	AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use material_ref::{
	MaterialId, MaterialLib, MaterialRef, MaterialRefCache, MaterialRefKey, MaterialScalars,
	ReferenceMaterial,
};
use procedural_common::NoiseParams;

/// Recipe name claimed by [`MuzzleFlameMaterialLib`].
pub const MUZZLE_FLAME_RECIPE: &str = "muzzle_flame";

const CORE_COLOR: Color = Color::srgb(1.0, 0.98, 0.94);
const LIMB_COLOR: Color = Color::srgb(1.0, 0.62, 0.18);
const FLAME_GAIN: f32 = 2.8;

/// Cache of resolved [`MuzzleFlameMaterial`] handles.
pub type MuzzleFlameMaterialRefCache = MaterialRefCache<MuzzleFlameMaterial>;

/// Inserts the muzzle-flame material cache. Idempotent.
pub fn init_muzzle_flame_caches(app: &mut App) {
	app.init_resource::<MuzzleFlameMaterialRefCache>();
}

/// The authored white-flame recipe: white core, warm limb, scalar gain.
pub fn muzzle_flame_ref() -> MaterialRef {
	MaterialRef::named(MUZZLE_FLAME_RECIPE)
		.with_palette([CORE_COLOR, LIMB_COLOR])
		.with_scalars([FLAME_GAIN])
}

/// Unlit additive cone. `core.w` is the flame gain.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct MuzzleFlameMaterial {
	#[uniform(0)]
	pub core: Vec4,
	#[uniform(1)]
	pub limb: Vec4,
}

impl MuzzleFlameMaterial {
	fn from_colors(core: Color, limb: Color, gain: f32) -> Self {
		let core = LinearRgba::from(core);
		let limb = LinearRgba::from(limb);
		Self {
			core: Vec4::new(core.red, core.green, core.blue, gain),
			limb: Vec4::new(limb.red, limb.green, limb.blue, 1.0),
		}
	}
}

impl Default for MuzzleFlameMaterial {
	fn default() -> Self {
		Self::from_colors(CORE_COLOR, LIMB_COLOR, FLAME_GAIN)
	}
}

impl ReferenceMaterial for MuzzleFlameMaterial {
	fn with_palette(mut self, palette: &[Color]) -> Self {
		if let Some(core) = palette.first() {
			let core = LinearRgba::from(*core);
			self.core = Vec4::new(core.red, core.green, core.blue, self.core.w);
		}
		if let Some(limb) = palette.get(1) {
			let limb = LinearRgba::from(*limb);
			self.limb = Vec4::new(limb.red, limb.green, limb.blue, self.limb.w);
		}
		self
	}

	fn with_noise_params(self, _noise: &NoiseParams) -> Self {
		self
	}

	fn with_scalars(mut self, scalars: &MaterialScalars) -> Self {
		if let Some(gain) = scalars.get(0) {
			self.core.w = gain;
		}
		self
	}
}

impl Material for MuzzleFlameMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "muzzle_flame.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "muzzle_flame.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Add
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
	}

	fn enable_prepass() -> bool {
		false
	}

	fn enable_shadows() -> bool {
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

/// Embedded shader plus [`MaterialPlugin`].
pub struct MuzzleFlameMaterialPlugin;

impl Plugin for MuzzleFlameMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "muzzle_flame.wgsl");
		if !app.is_plugin_added::<MaterialPlugin<MuzzleFlameMaterial>>() {
			app.add_plugins(MaterialPlugin::<MuzzleFlameMaterial>::default());
		}
	}
}

pub(crate) fn resolve_muzzle_flame(
	materials: &mut Assets<MuzzleFlameMaterial>,
	cache: &mut MuzzleFlameMaterialRefCache,
	material_ref: &MaterialRef,
) -> Handle<MuzzleFlameMaterial> {
	let key = MaterialRefKey::from(material_ref);
	if let Some(handle) = cache.get(&key) {
		return handle;
	}
	let handle = materials.add(MuzzleFlameMaterial::from_material_ref(material_ref));
	cache.insert(key, handle.clone());
	handle
}

/// Claims [`MUZZLE_FLAME_RECIPE`] only.
#[derive(SystemParam)]
pub struct MuzzleFlameMaterialLib<'w> {
	pub materials: ResMut<'w, Assets<MuzzleFlameMaterial>>,
	pub cache: ResMut<'w, MuzzleFlameMaterialRefCache>,
}

impl MaterialLib for MuzzleFlameMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if name == MUZZLE_FLAME_RECIPE => {
				let handle =
					resolve_muzzle_flame(&mut self.materials, &mut self.cache, material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle));
				true
			}
			_ => false,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn recipe_is_a_white_core_with_a_warm_limb() {
		let material = MuzzleFlameMaterial::from_material_ref(&muzzle_flame_ref());
		assert!(material.core.x > 0.95, "core stays white, {:?}", material.core);
		assert!(material.core.y > 0.9);
		assert!(material.limb.z < material.limb.x, "limb is warmer than white");
		assert!((material.core.w - FLAME_GAIN).abs() < 1e-4);
		assert_eq!(muzzle_flame_ref().name, MaterialId::named(MUZZLE_FLAME_RECIPE));
	}
}
