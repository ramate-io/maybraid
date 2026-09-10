//! Fireball [`Material`]. Default vertex + fire fill; tail / discard / embers next.

use bevy::{
	asset::embedded_asset,
	light::NotShadowCaster,
	prelude::*,
	reflect::TypePath,
	render::render_resource::AsBindGroup,
	shader::ShaderRef,
};

/// Visual capsule is a bit larger than the hit sphere so discard can chew the rim.
pub const FIREBALL_VISUAL_RADIUS: f32 = 1.18;
/// Extra shaft along object Y. Vertex code will pull the rear along `-Y`.
pub const FIREBALL_VISUAL_LENGTH: f32 = 2.2;
/// How far the rear hemisphere is pulled aft, in meters, at reference speed.
#[allow(dead_code)]
pub const FIREBALL_TAIL: f32 = 1.8;

/// Linear mid-flame. The fragment mixes a hotter core from the view-facing term.
const FIRE_FILL: Vec4 = Vec4::new(1.0, 0.28, 0.05, 1.0);

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FireballMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
}

impl FireballMaterial {
	pub fn new(_seed: u32, intensity: f32, _speed: f32, _time_offset: f32) -> Self {
		Self { base_color: FIRE_FILL * intensity.max(0.0) }
	}
}

impl Default for FireballMaterial {
	fn default() -> Self {
		Self { base_color: FIRE_FILL }
	}
}

impl Material for FireballMaterial {
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

	fn enable_shadows() -> bool {
		false
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

/// Subdivided capsule so a later aft pull has verts to stretch.
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
	fn uniform_is_a_solid_fill() {
		let material = FireballMaterial::new(0xA11A_5EED_u32, 1.0, 30.0, 0.25);
		assert_eq!(material.base_color, FIRE_FILL);
		assert!(FIREBALL_TAIL > 1.0);
		assert_eq!(FIREBALL_VISUAL_RADIUS, 1.18);
	}

	#[test]
	fn visual_mesh_has_positions() {
		let mesh = fireball_visual_mesh();
		assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
		assert!(mesh.count_vertices() > 64);
	}
}
