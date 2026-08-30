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
	MaterialId, MaterialLib, MaterialRasters, MaterialRef, MaterialRefCache, MaterialRefKey,
	MaterialRefPlugin, StandardMaterialLib, StandardMaterialRefCache, MATERIAL_PALETTE_SLOTS,
	MATERIAL_RASTER_CHANNELS, MATERIAL_RASTER_WIDTH, MATERIAL_SCALAR_FLOATS,
};

use crate::{BumpOutStyle, CHICO_BUMP_OUT_MATERIAL, RASTER_BITE_SIZE, RASTER_DENSITY};

const SCALAR_VEC4S: usize = MATERIAL_SCALAR_FLOATS / 4;

/// Packed, fixed-layout GPU representation of one material reference.
///
/// Channel meaning is a shader contract. Bump-out uses rasters 0–4 and scalars 0–6.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct BumpOutUniform {
	pub colors: [Vec4; MATERIAL_PALETTE_SLOTS],
	/// `x` broad noise frequency, `y` amplitude, `z` seed.
	pub noise: Vec4,
	pub scalars: [Vec4; SCALAR_VEC4S],
	pub rasters: [[Vec4; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS],
}

impl BumpOutUniform {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let fallback = [
			Vec4::new(0.16, 0.36, 0.14, 1.0),
			Vec4::new(0.24, 0.52, 0.20, 1.0),
			Vec4::new(0.38, 0.64, 0.24, 1.0),
		];
		let mut colors = [Vec4::ZERO; MATERIAL_PALETTE_SLOTS];
		if material_ref.palette.is_empty() {
			for (slot, color) in colors.iter_mut().zip(fallback) {
				*slot = color;
			}
		} else {
			for (slot, color) in colors.iter_mut().zip(&material_ref.palette) {
				let linear = LinearRgba::from(*color);
				*slot = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
			}
			let first = colors[0];
			for color in colors.iter_mut().take(3).skip(material_ref.palette.len()) {
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
		if values.is_empty() {
			let style = BumpOutStyle::default();
			let packed = style.as_values();
			scalars[0] = Vec4::new(packed[0], packed[1], packed[2], packed[3]);
			scalars[1] = Vec4::new(packed[4], packed[5], packed[6], 0.0);
		}

		let mut rasters = [[Vec4::ZERO; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS];
		for channel in 0..MATERIAL_RASTER_CHANNELS {
			let default = match channel {
				RASTER_DENSITY => 1.0,
				RASTER_BITE_SIZE => 12.0,
				_ => 0.0,
			};
			let samples = material_ref.rasters.get_or(channel, default);
			let rows = MaterialRasters::packed_rows(samples);
			rasters[channel] = rows.map(|row| Vec4::from_array(row));
		}

		Self {
			colors,
			noise: Vec4::new(
				material_ref.noise.frequency.max(1e-6),
				material_ref.noise.amplitude,
				material_ref.noise.seed as f32,
				0.0,
			),
			scalars,
			rasters,
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

#[cfg(test)]
mod tests {
	use procedural_common::NoiseParams;

	use super::*;
	use crate::{
		BumpOutNeighborhood, RASTER_AVERAGE_HEIGHT, RASTER_BITE_SIZE, RASTER_BITE_SIZE_DEVIATION,
		RASTER_DENSITY, RASTER_HEIGHT_DEVIATION,
	};

	#[test]
	fn material_ref_maps_raster_channels() {
		let neighborhood = BumpOutNeighborhood::new(
			[0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
			[10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0],
			[0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
			[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
			[0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3],
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

		assert_eq!(uniform.rasters[RASTER_DENSITY][1], Vec4::new(0.3, 0.4, 0.5, 0.0));
		assert_eq!(uniform.rasters[RASTER_BITE_SIZE][0], Vec4::new(10.0, 11.0, 12.0, 0.0));
		assert_eq!(uniform.rasters[RASTER_BITE_SIZE_DEVIATION][2], Vec4::new(0.6, 0.7, 0.8, 0.0));
		assert_eq!(uniform.rasters[RASTER_AVERAGE_HEIGHT][2], Vec4::new(7.0, 8.0, 9.0, 0.0));
		assert_eq!(uniform.rasters[RASTER_HEIGHT_DEVIATION][1], Vec4::new(0.8, 0.9, 1.0, 0.0));
		assert!((uniform.noise.y - 1.5).abs() < 1e-6);
		assert_eq!(uniform.scalars[0], Vec4::new(0.07, 0.8, 0.3, 0.65));
		assert_eq!(uniform.scalars[1], Vec4::new(1.4, 5.0, 0.75, 0.0));
	}
}
