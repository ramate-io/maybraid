use bevy::ecs::system::SystemParam;
use bevy::{
	asset::embedded_asset,
	light::NotShadowCaster,
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
	MaterialId, MaterialLib, MaterialRef, MaterialRefCache, MaterialRefKey, MaterialRefPlugin,
	StandardMaterialLib, StandardMaterialRefCache,
};

use crate::{
	BumpOutNeighborhood, BumpOutStyle, BUMP_OUT_NEIGHBORHOOD_WIDTH, CHICO_BUMP_OUT_MATERIAL,
};

/// Packed, fixed-layout GPU representation of one bump-out material reference.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct BumpOutUniform {
	pub colors: [Vec4; 3],
	/// `x` frequency, `y` displacement amplitude, `z` seed.
	pub noise: Vec4,
	/// `x` coverage softness, `y` roughness, `z` normal soften, `w` cheese amount.
	pub style: Vec4,
	/// `x` cheese scale, `y` fragment-height frequency, `z` fragment-height amplitude.
	pub detail: Vec4,
	pub density_rows: [Vec4; BUMP_OUT_NEIGHBORHOOD_WIDTH],
	pub height_rows: [Vec4; BUMP_OUT_NEIGHBORHOOD_WIDTH],
}

impl BumpOutUniform {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let neighborhood = BumpOutNeighborhood::from_material_ref(material_ref);
		let style = BumpOutStyle::from_material_ref(material_ref);
		let fallback = [
			Vec4::new(0.16, 0.36, 0.14, 1.0),
			Vec4::new(0.24, 0.52, 0.20, 1.0),
			Vec4::new(0.38, 0.64, 0.24, 1.0),
		];
		let mut colors = fallback;
		for (target, color) in colors.iter_mut().zip(&material_ref.palette) {
			let linear = LinearRgba::from(*color);
			*target = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
		}

		Self {
			colors,
			noise: Vec4::new(
				material_ref.noise.frequency.max(1e-6),
				material_ref.noise.amplitude,
				material_ref.noise.seed as f32,
				0.0,
			),
			style: Vec4::new(
				style.coverage_softness.max(0.0),
				style.roughness.clamp(0.0, 1.0),
				style.normal_soften.clamp(0.0, 1.0),
				style.cheese_amount.clamp(0.0, 1.0),
			),
			detail: Vec4::new(
				style.cheese_scale.max(1e-4),
				style.fragment_height_frequency.max(1e-4),
				style.fragment_height_amplitude.max(0.0),
				0.0,
			),
			density_rows: rows(neighborhood.densities),
			height_rows: rows(neighborhood.heights),
		}
	}
}

impl Default for BumpOutUniform {
	fn default() -> Self {
		Self::from_material_ref(&MaterialRef::named(CHICO_BUMP_OUT_MATERIAL))
	}
}

/// Vertex-displaced, noise-masked material used by ground-cover and canopy bump outs.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct BumpOutMaterial {
	#[uniform(0)]
	pub uniform: BumpOutUniform,
}

impl BumpOutMaterial {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		Self { uniform: BumpOutUniform::from_material_ref(material_ref) }
	}
}

impl Material for BumpOutMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "bump_out_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "bump_out_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
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

/// Registers the embedded shader and Bevy material asset.
pub struct BumpOutMaterialPlugin;

impl Plugin for BumpOutMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "bump_out_material.wgsl");
		app.add_plugins(MaterialPlugin::<BumpOutMaterial>::default())
			.add_systems(PostUpdate, disable_bump_out_shadow_casters);
	}
}

fn disable_bump_out_shadow_casters(
	mut commands: Commands,
	query: Query<Entity, (With<MeshMaterial3d<BumpOutMaterial>>, Without<NotShadowCaster>)>,
) {
	for entity in &query {
		commands.entity(entity).insert(NotShadowCaster);
	}
}

pub type BumpOutMaterialRefCache = MaterialRefCache<BumpOutMaterial>;

/// Standalone material library used by the bump-out playground.
///
/// Production apps with a combined domain library can call [`BumpOutMaterial::from_material_ref`]
/// from that library instead of installing a competing [`MaterialRefPlugin`].
#[derive(SystemParam)]
pub struct BumpOutMaterialLib<'w> {
	pub standard: StandardMaterialLib<'w>,
	pub materials: ResMut<'w, Assets<BumpOutMaterial>>,
	pub cache: ResMut<'w, BumpOutMaterialRefCache>,
}

impl BumpOutMaterialLib<'_> {
	pub fn resolve(&mut self, material_ref: &MaterialRef) -> Handle<BumpOutMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.cache.get(&key) {
			return handle;
		}
		let handle = self.materials.add(BumpOutMaterial::from_material_ref(material_ref));
		self.cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for BumpOutMaterialLib<'_> {
	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		match &material_ref.name {
			MaterialId::Name(name) if name == CHICO_BUMP_OUT_MATERIAL => {
				let handle = self.resolve(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert((MeshMaterial3d(handle), NotShadowCaster));
			}
			_ => self.standard.fulfill(entity, material_ref, commands),
		}
	}
}

/// Registers caches and deferred [`MaterialRef`] fulfillment for a standalone bump-out app.
pub struct BumpOutMaterialRefPlugin;

impl Plugin for BumpOutMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<StandardMaterialRefCache>()
			.init_resource::<BumpOutMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<BumpOutMaterialLib<'_>>::default());
	}
}

fn rows(values: [f32; 9]) -> [Vec4; BUMP_OUT_NEIGHBORHOOD_WIDTH] {
	[
		Vec4::new(values[0], values[1], values[2], 0.0),
		Vec4::new(values[3], values[4], values[5], 0.0),
		Vec4::new(values[6], values[7], values[8], 0.0),
	]
}

#[cfg(test)]
mod tests {
	use procedural_common::NoiseParams;

	use super::*;

	#[test]
	fn material_ref_maps_named_neighborhood_rows() {
		let neighborhood = BumpOutNeighborhood::new(
			[0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
			[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
		);
		let material_ref = neighborhood.material_ref(
			[Color::srgb(0.1, 0.5, 0.2)],
			NoiseParams::from_scalar(17.0, 0.2, 1.5, 2),
		);
		let material_ref = BumpOutStyle::new(0.07, 0.8, 0.3)
			.with_cheese(0.65, 1.4)
			.with_fragment_height(5.0, 0.75)
			.apply_to(material_ref);
		let uniform = BumpOutUniform::from_material_ref(&material_ref);

		assert_eq!(uniform.density_rows[1], Vec4::new(0.3, 0.4, 0.5, 0.0));
		assert_eq!(uniform.height_rows[2], Vec4::new(7.0, 8.0, 9.0, 0.0));
		assert!((uniform.noise.y - 1.5).abs() < 1e-6);
		assert_eq!(uniform.style, Vec4::new(0.07, 0.8, 0.3, 0.65));
		assert_eq!(uniform.detail, Vec4::new(1.4, 5.0, 0.75, 0.0));
	}
}
