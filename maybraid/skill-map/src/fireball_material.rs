//! Fireball [`Material`]. Flight uniforms (`origin`, launch, gravity) are
//! per-shot. A later MaterialRef port should cache the *shader* and marshal
//! this pad per instance — not allocate a handle per unique origin.

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
/// Fraction of distance traveled that the tail unfolds. Leaves a gap at the caster.
pub const FIREBALL_TAIL_TRACE: f32 = 0.92;
/// Cylinder length so rest-pose aft ≈ `speed * max_age * TRACE`. Keep in sync with the shader.
pub const FIREBALL_VISUAL_LENGTH: f32 =
	FIREBALL_SPEED * FIREBALL_MAX_AGE * FIREBALL_TAIL_TRACE - FIREBALL_VISUAL_RADIUS;
/// Unfolded nub at spawn, in meters. Keep in sync with `TAIL_START` in the shader.
#[allow(dead_code)]
pub const FIREBALL_TAIL_START: f32 = 0.45;

/// Linear mid-flame. The fragment mixes a hotter core from the view-facing term.
const FIRE_FILL: Vec4 = Vec4::new(1.0, 0.28, 0.05, 1.0);

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FireballMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
	/// `x` seed, `y` speed, `z` spawn time, `w` max age.
	#[uniform(1)]
	pub displace: Vec4,
	/// World muzzle.
	#[uniform(2)]
	pub origin: Vec4,
	/// Launch velocity (`direction * speed`).
	#[uniform(3)]
	pub launch: Vec4,
	/// World gravity on this flight (`Gravity * GravityScale`).
	#[uniform(4)]
	pub gravity: Vec4,
}

impl FireballMaterial {
	pub fn new(
		seed: u32,
		intensity: f32,
		speed: f32,
		spawn_time: f32,
		origin: Vec3,
		velocity: Vec3,
		gravity: Vec3,
	) -> Self {
		Self {
			base_color: FIRE_FILL * intensity.max(0.0),
			displace: Vec4::new(seed as f32, speed, spawn_time, FIREBALL_MAX_AGE),
			origin: origin.extend(0.0),
			launch: velocity.extend(0.0),
			gravity: gravity.extend(0.0),
		}
	}
}

impl Default for FireballMaterial {
	fn default() -> Self {
		Self::new(
			0,
			1.0,
			FIREBALL_SPEED,
			0.0,
			Vec3::ZERO,
			Vec3::NEG_Z * FIREBALL_SPEED,
			Vec3::NEG_Y * 9.81,
		)
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

/// Long capsule with the ball at the origin and the shaft along `-Y`.
pub fn fireball_visual_mesh() -> Mesh {
	Capsule3d::new(FIREBALL_VISUAL_RADIUS, FIREBALL_VISUAL_LENGTH)
		.mesh()
		.latitudes(14)
		.longitudes(22)
		.rings(72)
		.build()
		.translated_by(Vec3::new(0.0, -FIREBALL_VISUAL_LENGTH * 0.5, 0.0))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn tail_length_at(age: f32, speed: f32) -> f32 {
		(age * speed * FIREBALL_TAIL_TRACE).max(FIREBALL_TAIL_START)
	}

	#[test]
	fn uniform_carries_fill_and_lifetime() {
		let origin = Vec3::new(3.0, 1.5, -8.0);
		let velocity = Vec3::new(0.0, 4.0, -30.0);
		let gravity = Vec3::NEG_Y * 9.81;
		let material =
			FireballMaterial::new(0xA11A_5EED_u32, 1.0, 30.0, 0.25, origin, velocity, gravity);
		assert_eq!(material.origin.truncate(), origin);
		assert_eq!(material.launch.truncate(), velocity);
		assert!((material.gravity.y + 9.81).abs() < 1e-4);
		let later = origin + velocity * 1.0 + 0.5 * gravity;
		assert!(later.y < origin.y + velocity.y);
		assert_eq!(material.base_color, FIRE_FILL);
		assert_eq!(material.displace.x, 0xA11A_5EED_u32 as f32);
		assert_eq!(material.displace.w, FIREBALL_MAX_AGE);
		assert!(tail_length_at(0.0, FIREBALL_SPEED) < tail_length_at(1.0, FIREBALL_SPEED));
		assert!(
			(tail_length_at(FIREBALL_MAX_AGE, FIREBALL_SPEED)
				- FIREBALL_SPEED * FIREBALL_MAX_AGE * FIREBALL_TAIL_TRACE)
				.abs() < 1e-3
		);
		assert!((FIREBALL_VISUAL_LENGTH - 136.82).abs() < 0.02);
	}

	#[test]
	fn visual_mesh_hangs_aft_from_the_ball() {
		let mesh = fireball_visual_mesh();
		let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
			mesh.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			panic!("fireball mesh needs positions");
		};
		let min_y = positions.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
		let max_y = positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
		assert!(mesh.count_vertices() > 64);
		assert!(max_y < FIREBALL_VISUAL_RADIUS + 0.05);
		assert!(min_y < -(FIREBALL_VISUAL_LENGTH + FIREBALL_VISUAL_RADIUS) + 0.25);
	}
}
