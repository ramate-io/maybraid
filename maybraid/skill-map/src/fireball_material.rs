//! Fireball [`Material`]. Vertex aft-bleed on the working fire fill.
//! Rim discard and Hanabi stay off.

use bevy::{
	asset::embedded_asset,
	light::NotShadowCaster,
	prelude::*,
	reflect::TypePath,
	render::render_resource::AsBindGroup,
	shader::ShaderRef,
};

use crate::effects::{FIREBALL_MAX_AGE, FIREBALL_SPEED};

/// Visual capsule is a bit larger than the hit sphere so discard can chew the rim.
pub const FIREBALL_VISUAL_RADIUS: f32 = 1.18;
/// Extra shaft along object Y so a growing aft pull has verts to stretch.
pub const FIREBALL_VISUAL_LENGTH: f32 = 4.2;
/// Aft pull at spawn, in meters. Keep in sync with `TAIL_START` in the shader.
#[allow(dead_code)]
pub const FIREBALL_TAIL_START: f32 = 0.35;
/// Aft pull at `max_age`, in meters. Keep in sync with `TAIL_END` in the shader.
#[allow(dead_code)]
pub const FIREBALL_TAIL_END: f32 = 4.6;

/// Linear mid-flame. The fragment mixes a hotter core from the view-facing term.
const FIRE_FILL: Vec4 = Vec4::new(1.0, 0.28, 0.05, 1.0);

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FireballMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
	/// `x` seed, `y` speed, `z` spawn time, `w` max age.
	#[uniform(1)]
	pub displace: Vec4,
}

impl FireballMaterial {
	pub fn new(seed: u32, intensity: f32, speed: f32, spawn_time: f32) -> Self {
		Self {
			base_color: FIRE_FILL * intensity.max(0.0),
			displace: Vec4::new(seed as f32, speed, spawn_time, FIREBALL_MAX_AGE),
		}
	}
}

impl Default for FireballMaterial {
	fn default() -> Self {
		Self::new(0, 1.0, FIREBALL_SPEED, 0.0)
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

/// Subdivided capsule so the aft pull has verts to stretch.
pub fn fireball_visual_mesh() -> Mesh {
	Capsule3d::new(FIREBALL_VISUAL_RADIUS, FIREBALL_VISUAL_LENGTH)
		.mesh()
		.latitudes(18)
		.longitudes(28)
		.rings(14)
		.build()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn tail_length_at(age: f32, max_age: f32) -> f32 {
		let u = (age / max_age.max(1e-3)).clamp(0.0, 1.0);
		let grow = u * (2.0 - u);
		FIREBALL_TAIL_START + (FIREBALL_TAIL_END - FIREBALL_TAIL_START) * grow
	}

	#[test]
	fn uniform_carries_fill_and_lifetime() {
		let material = FireballMaterial::new(0xA11A_5EED_u32, 1.0, 30.0, 0.25);
		assert_eq!(material.base_color, FIRE_FILL);
		assert_eq!(material.displace.x, 0xA11A_5EED_u32 as f32);
		assert_eq!(material.displace.w, FIREBALL_MAX_AGE);
		assert!(FIREBALL_TAIL_END > FIREBALL_TAIL_START);
		assert!(tail_length_at(0.0, FIREBALL_MAX_AGE) < tail_length_at(1.0, FIREBALL_MAX_AGE));
		assert!((tail_length_at(FIREBALL_MAX_AGE, FIREBALL_MAX_AGE) - FIREBALL_TAIL_END).abs() < 1e-5);
		assert_eq!(FIREBALL_VISUAL_RADIUS, 1.18);
	}

	#[test]
	fn visual_mesh_has_positions() {
		let mesh = fireball_visual_mesh();
		assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
		assert!(mesh.count_vertices() > 64);
	}
}
