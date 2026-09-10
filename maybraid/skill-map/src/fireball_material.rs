//! Unlit fireball [`Material`]: vertex aft-bleed, discarded ragged rim.

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

use crate::effects::FIREBALL_SPEED;

/// Visual capsule is a bit larger than the hit sphere so discard can chew the rim.
pub const FIREBALL_VISUAL_RADIUS: f32 = 1.18;
/// Extra shaft along object Y. Vertex code pulls the rear along `-Y`.
pub const FIREBALL_VISUAL_LENGTH: f32 = 2.2;
/// How far the rear hemisphere is pulled aft, in meters, at reference speed.
pub const FIREBALL_TAIL: f32 = 1.8;

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct FireballUniform {
	pub seed: f32,
	pub intensity: f32,
	pub speed: f32,
	pub time_offset: f32,
	pub radius: f32,
	pub tail: f32,
	pub _pad: Vec2,
}

impl Default for FireballUniform {
	fn default() -> Self {
		Self {
			seed: 0.0,
			intensity: 1.0,
			speed: FIREBALL_SPEED,
			time_offset: 0.0,
			radius: FIREBALL_VISUAL_RADIUS,
			tail: FIREBALL_TAIL,
			_pad: Vec2::ZERO,
		}
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FireballMaterial {
	#[uniform(0)]
	pub params: FireballUniform,
}

impl FireballMaterial {
	pub fn new(seed: u32, intensity: f32, speed: f32, time_offset: f32) -> Self {
		Self {
			params: FireballUniform {
				seed: seed as f32,
				intensity,
				speed,
				time_offset,
				radius: FIREBALL_VISUAL_RADIUS,
				tail: FIREBALL_TAIL,
				_pad: Vec2::ZERO,
			},
		}
	}
}

impl Default for FireballMaterial {
	fn default() -> Self {
		Self { params: FireballUniform::default() }
	}
}

impl Material for FireballMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "fireball_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "fireball_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
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

pub struct FireballMaterialPlugin;

impl Plugin for FireballMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "fireball_material.wgsl");
		app.add_plugins(MaterialPlugin::<FireballMaterial>::default());
		app.add_systems(PostUpdate, disable_fireball_shadow_casters);
	}
}

fn disable_fireball_shadow_casters(
	mut commands: Commands,
	query: Query<Entity, (With<MeshMaterial3d<FireballMaterial>>, Without<NotShadowCaster>)>,
) {
	for entity in &query {
		commands.entity(entity).insert(NotShadowCaster);
	}
}

/// Subdivided capsule so the aft pull has verts to stretch.
pub fn fireball_visual_mesh() -> Mesh {
	Capsule3d::new(FIREBALL_VISUAL_RADIUS, FIREBALL_VISUAL_LENGTH)
		.mesh()
		.latitudes(18)
		.longitudes(28)
		.rings(8)
		.build()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn uniform_carries_seed_and_tail() {
		let material = FireballMaterial::new(0xA11A_5EED_u32, 1.0, 30.0, 0.25);
		assert_eq!(material.params.seed, 0xA11A_5EED_u32 as f32);
		assert!(material.params.tail > 1.0);
		assert_eq!(material.params.radius, FIREBALL_VISUAL_RADIUS);
	}

	#[test]
	fn visual_mesh_has_positions() {
		let mesh = fireball_visual_mesh();
		assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
		assert!(mesh.count_vertices() > 64);
	}
}
