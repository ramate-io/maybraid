//! Distance-fade sky dome. An inverted sphere follows the camera so far
//! terrain and forest wash to blue. This is an aesthetic mask, not a cull clock.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::f32::consts::PI;

/// Start a light haze at this XZ radius (m).
pub const DEFAULT_INNER_FADE_M: f32 = 350.0;
/// Haze reaches [`SkyDomePlugin::max_alpha`] at this XZ radius (m).
pub const DEFAULT_OUTER_FADE_M: f32 = 1_200.0;
/// Sphere mesh radius. Larger than the fade so the shell stays off the near ground.
pub const DEFAULT_SPHERE_RADIUS_M: f32 = 2_800.0;
/// Peak wash. Stay well under 1 so ridges are not cut out by an opaque band.
pub const DEFAULT_MAX_ALPHA: f32 = 0.32;

/// Clear / dome blue used by the world playground.
pub const SKY_BLUE: Color = Color::hsla(201.0, 0.69, 0.62, 1.0);

#[derive(Component)]
pub struct SkyDome;

pub struct SkyDomePlugin {
	pub inner_fade_m: f32,
	pub outer_fade_m: f32,
	pub sphere_radius_m: f32,
	pub max_alpha: f32,
	pub color: Color,
}

impl Default for SkyDomePlugin {
	fn default() -> Self {
		Self {
			inner_fade_m: DEFAULT_INNER_FADE_M,
			outer_fade_m: DEFAULT_OUTER_FADE_M,
			sphere_radius_m: DEFAULT_SPHERE_RADIUS_M,
			max_alpha: DEFAULT_MAX_ALPHA,
			color: SKY_BLUE,
		}
	}
}

impl Plugin for SkyDomePlugin {
	fn build(&self, app: &mut App) {
		let settings = DomeSettings {
			inner_fade_m: self.inner_fade_m,
			outer_fade_m: self.outer_fade_m.max(self.inner_fade_m + 1.0),
			sphere_radius_m: self.sphere_radius_m.max(self.outer_fade_m),
			max_alpha: self.max_alpha.clamp(0.0, 1.0),
			color: self.color,
		};
		app.insert_resource(settings)
			.add_systems(Startup, spawn_sky_dome)
			.add_systems(Update, follow_camera);
	}
}

#[derive(Resource, Clone, Copy)]
struct DomeSettings {
	inner_fade_m: f32,
	outer_fade_m: f32,
	sphere_radius_m: f32,
	max_alpha: f32,
	color: Color,
}

fn spawn_sky_dome(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	settings: Res<DomeSettings>,
) {
	let mesh = meshes.add(fade_sphere(*settings));
	let material = materials.add(StandardMaterial {
		base_color: Color::WHITE,
		unlit: true,
		alpha_mode: AlphaMode::Blend,
		cull_mode: None,
		// Geometry shaders apply DistanceFog; this dome is a separate XZ wash.
		fog_enabled: false,
		..default()
	});
	commands.spawn((
		SkyDome,
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::IDENTITY,
		Visibility::Visible,
	));
}

fn follow_camera(
	camera: Query<&GlobalTransform, With<Camera3d>>,
	mut dome: Query<&mut Transform, With<SkyDome>>,
) {
	let Ok(cam) = camera.single() else {
		return;
	};
	let Ok(mut tf) = dome.single_mut() else {
		return;
	};
	tf.translation = cam.translation();
	tf.rotation = Quat::IDENTITY;
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
	let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

/// UV sphere. Vertex alpha rises with XZ radius so the horizon washes blue
/// and the volume over the viewer stays clear.
fn fade_sphere(settings: DomeSettings) -> Mesh {
	let rings = 48u32;
	let segs = 64u32;
	let rgba = settings.color.to_linear();
	let radius = settings.sphere_radius_m;

	let mut positions = Vec::new();
	let mut normals = Vec::new();
	let mut colors = Vec::new();
	let mut indices = Vec::new();

	for ring in 0..=rings {
		let v = ring as f32 / rings as f32;
		let theta = v * PI;
		let y = radius * theta.cos();
		let ring_r = radius * theta.sin();
		for seg in 0..=segs {
			let u = seg as f32 / segs as f32;
			let phi = u * 2.0 * PI;
			let x = ring_r * phi.cos();
			let z = ring_r * phi.sin();
			positions.push([x, y, z]);
			let len = (x * x + y * y + z * z).sqrt().max(1e-5);
			normals.push([-x / len, -y / len, -z / len]);
			let xz = (x * x + z * z).sqrt();
			let fade = smoothstep(settings.inner_fade_m, settings.outer_fade_m, xz);
			let alpha = settings.max_alpha * fade * fade;
			colors.push([rgba.red, rgba.green, rgba.blue, alpha]);
		}
	}

	let verts_per_ring = segs + 1;
	for ring in 0..rings {
		for seg in 0..segs {
			let a = ring * verts_per_ring + seg;
			let b = a + verts_per_ring;
			indices.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
		}
	}

	let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
	mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
	mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
	mesh.insert_indices(Indices::U32(indices));
	mesh
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fade_sphere_stays_a_wash_not_a_wall() {
		let mesh = fade_sphere(DomeSettings {
			inner_fade_m: DEFAULT_INNER_FADE_M,
			outer_fade_m: DEFAULT_OUTER_FADE_M,
			sphere_radius_m: DEFAULT_SPHERE_RADIUS_M,
			max_alpha: DEFAULT_MAX_ALPHA,
			color: SKY_BLUE,
		});
		let colors = mesh.attribute(Mesh::ATTRIBUTE_COLOR).expect("colors");
		let bevy::mesh::VertexAttributeValues::Float32x4(colors) = colors else {
			panic!("expected rgba colors");
		};
		assert!(colors.iter().any(|c| c[3] < 0.02), "near-axis vertices stay clear");
		let peak = colors.iter().map(|c| c[3]).fold(0.0_f32, f32::max);
		assert!(peak > 0.15 && peak <= DEFAULT_MAX_ALPHA + 1e-4, "peak={peak}");
	}
}
