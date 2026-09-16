//! Unlit sun disk, moon, and sparse stars. Camera-parented; no shadow casters.

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::f32::consts::PI;

use crate::dome::DomeSettings;
use crate::SkySun;

/// Just inside the dome so the disk does not clip the wash shell.
pub const CELESTIAL_DISTANCE_FACTOR: f32 = 0.82;
pub const SUN_DISK_RADIUS_M: f32 = 55.0;
pub const SUN_CORONA_RADIUS_M: f32 = 140.0;
pub const MOON_RADIUS_M: f32 = 22.0;
pub const MOON_YAW_OFFSET: f32 = 120.0 * PI / 180.0;
pub const MOON_LIFT: f32 = 0.38;
pub const STAR_COUNT: u32 = 48;
pub const STAR_SIZE_M: f32 = 5.5;

pub const SUN_DISK_COLOR: Color = Color::hsla(42.0, 0.55, 0.92, 1.0);
pub const SUN_CORONA_COLOR: Color = Color::hsla(32.0, 0.70, 0.70, 1.0);
pub const MOON_COLOR: Color = Color::hsla(215.0, 0.12, 0.78, 1.0);
pub const STAR_COLOR: Color = Color::hsla(220.0, 0.08, 0.86, 1.0);

/// Visible sun disk. Local translation is the camera→sun axis.
#[derive(Component, Debug, Clone, Copy)]
pub struct SkySunDisk;

/// Cooler, smaller companion opposite the key.
#[derive(Component, Debug, Clone, Copy)]
pub struct SkyMoon;

impl SkyMoon {
	/// Offset ~120° from the sun and lifted so it sits higher in the dome.
	pub fn direction_from_sun(sun_disk: Vec3) -> Vec3 {
		let yawed = Quat::from_axis_angle(Vec3::Y, MOON_YAW_OFFSET) * sun_disk;
		(yawed + Vec3::Y * MOON_LIFT).normalize_or_zero()
	}
}

/// Sparse upper-hemisphere points. One mesh, dim enough for afternoon.
#[derive(Component, Debug, Clone, Copy)]
pub struct SkyStars;

impl SkyStars {
	pub fn mesh(distance: f32) -> Mesh {
		let mut positions = Vec::new();
		let mut normals = Vec::new();
		let mut colors = Vec::new();
		let mut indices = Vec::new();
		let rgba = STAR_COLOR.to_linear();
		let color = [rgba.red, rgba.green, rgba.blue, 0.22];

		for i in 0..STAR_COUNT {
			let dir = Self::direction(i);
			let center = dir * distance;
			push_tetrahedron(
				&mut positions,
				&mut normals,
				&mut colors,
				&mut indices,
				center,
				STAR_SIZE_M,
				color,
			);
		}

		let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
		mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
		mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
		mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
		mesh.insert_indices(Indices::U32(indices));
		mesh
	}

	/// Deterministic upper-hemisphere sample. Skips the low band so stars stay near zenith.
	pub fn direction(index: u32) -> Vec3 {
		let u = hash01(index.wrapping_mul(2));
		let v = hash01(index.wrapping_mul(2).wrapping_add(1));
		let y = 0.42 + 0.58 * u;
		let phi = v * 2.0 * PI;
		let r = (1.0 - y * y).max(0.0).sqrt();
		Vec3::new(r * phi.cos(), y, r * phi.sin()).normalize_or_zero()
	}
}

pub(crate) fn spawn_sky_celestial(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	settings: Res<DomeSettings>,
	sun: Query<&Transform, With<SkySun>>,
	dome: Query<Entity, With<crate::SkyDome>>,
) {
	let Ok(sun) = sun.single() else {
		return;
	};
	let Ok(parent) = dome.single() else {
		return;
	};
	let distance = settings.sphere_radius_m * CELESTIAL_DISTANCE_FACTOR;
	let sun_dir = SkySun::disk_direction(sun);
	let moon_dir = SkyMoon::direction_from_sun(sun_dir);

	let disk = commands
		.spawn((
			Name::new("sky-sun-disk"),
			SkySunDisk,
			Mesh3d(meshes.add(Sphere::new(SUN_DISK_RADIUS_M))),
			MeshMaterial3d(materials.add(unlit_opaque(SUN_DISK_COLOR))),
			Transform::from_translation(SkySun::disk_offset(sun, distance)),
			NotShadowCaster,
			ChildOf(parent),
		))
		.id();
	commands.spawn((
		Name::new("sky-sun-corona"),
		Mesh3d(meshes.add(Sphere::new(SUN_CORONA_RADIUS_M))),
		MeshMaterial3d(materials.add(unlit_add(SUN_CORONA_COLOR, 0.22))),
		Transform::IDENTITY,
		NotShadowCaster,
		ChildOf(disk),
	));
	commands.spawn((
		Name::new("sky-moon"),
		SkyMoon,
		Mesh3d(meshes.add(Sphere::new(MOON_RADIUS_M))),
		MeshMaterial3d(materials.add(unlit_blend(MOON_COLOR, 0.55))),
		Transform::from_translation(moon_dir * distance),
		NotShadowCaster,
		ChildOf(parent),
	));
	commands.spawn((
		Name::new("sky-stars"),
		SkyStars,
		Mesh3d(meshes.add(SkyStars::mesh(distance))),
		MeshMaterial3d(materials.add(unlit_blend(STAR_COLOR, 0.22))),
		Transform::IDENTITY,
		NotShadowCaster,
		ChildOf(parent),
	));
}

fn unlit_opaque(color: Color) -> StandardMaterial {
	StandardMaterial {
		base_color: color,
		unlit: true,
		alpha_mode: AlphaMode::Opaque,
		cull_mode: None,
		fog_enabled: false,
		..default()
	}
}

fn unlit_blend(color: Color, alpha: f32) -> StandardMaterial {
	let mut linear = color.to_linear();
	linear.alpha = alpha;
	StandardMaterial {
		base_color: Color::from(linear),
		unlit: true,
		alpha_mode: AlphaMode::Blend,
		cull_mode: None,
		fog_enabled: false,
		..default()
	}
}

fn unlit_add(color: Color, alpha: f32) -> StandardMaterial {
	let mut linear = color.to_linear();
	linear.alpha = alpha;
	StandardMaterial {
		base_color: Color::from(linear),
		unlit: true,
		alpha_mode: AlphaMode::Add,
		cull_mode: None,
		fog_enabled: false,
		..default()
	}
}

fn push_tetrahedron(
	positions: &mut Vec<[f32; 3]>,
	normals: &mut Vec<[f32; 3]>,
	colors: &mut Vec<[f32; 4]>,
	indices: &mut Vec<u32>,
	center: Vec3,
	size: f32,
	color: [f32; 4],
) {
	let base = positions.len() as u32;
	let verts = [
		center + Vec3::new(size, size, size),
		center + Vec3::new(size, -size, -size),
		center + Vec3::new(-size, size, -size),
		center + Vec3::new(-size, -size, size),
	];
	for v in verts {
		positions.push(v.to_array());
		let n = (v - center).normalize_or_zero();
		normals.push(n.to_array());
		colors.push(color);
	}
	indices.extend_from_slice(&[
		base,
		base + 1,
		base + 2,
		base,
		base + 2,
		base + 3,
		base,
		base + 3,
		base + 1,
		base + 1,
		base + 3,
		base + 2,
	]);
}

fn hash01(x: u32) -> f32 {
	let mut h = x.wrapping_mul(747796405).wrapping_add(2891336453);
	h = ((h >> ((h >> 28) + 4)) ^ h).wrapping_mul(277803737);
	((h >> 22) ^ h) as f32 / u32::MAX as f32
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sun_disk_sits_opposite_the_shine_ray() {
		let sun = SkySun::pose();
		let dir = SkySun::disk_direction(&sun);
		assert!(dir.y > 0.0, "disk should sit in the sky");
		assert!(SkySun::shine_direction(&sun).dot(dir) < -0.99);
	}

	#[test]
	fn moon_is_offset_and_higher() {
		let sun = SkySun::disk_direction(&SkySun::pose());
		let moon = SkyMoon::direction_from_sun(sun);
		assert!(moon.y > sun.y);
		let sun_xz = Vec3::new(sun.x, 0.0, sun.z).normalize_or_zero();
		let moon_xz = Vec3::new(moon.x, 0.0, moon.z).normalize_or_zero();
		assert!(sun_xz.dot(moon_xz) < 0.0, "moon should sit away from the sun on XZ");
	}

	#[test]
	fn stars_stay_in_the_upper_hemisphere() {
		for i in 0..STAR_COUNT {
			let dir = SkyStars::direction(i);
			assert!(dir.y > 0.4, "star {i} y={}", dir.y);
		}
	}
}
