//! Distance-fade sky dome. An inverted sphere follows the camera so far
//! terrain and forest wash to blue. This is an aesthetic mask, not a cull clock.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::f32::consts::PI;

/// Start fading at this XZ radius (m). Inside is clear.
pub const DEFAULT_INNER_FADE_M: f32 = 800.0;
/// Fully washed at this XZ radius (m). Matches the 2 km generate ring.
pub const DEFAULT_OUTER_RADIUS_M: f32 = 2_000.0;

/// Clear / dome blue used by the vegetation-on-terrain and world playgrounds.
pub const SKY_BLUE: Color = Color::hsla(201.0, 0.69, 0.62, 1.0);

#[derive(Component)]
pub struct SkyDome;

pub struct SkyDomePlugin {
	pub inner_fade_m: f32,
	pub outer_radius_m: f32,
	pub color: Color,
}

impl Default for SkyDomePlugin {
	fn default() -> Self {
		Self {
			inner_fade_m: DEFAULT_INNER_FADE_M,
			outer_radius_m: DEFAULT_OUTER_RADIUS_M,
			color: SKY_BLUE,
		}
	}
}

impl Plugin for SkyDomePlugin {
	fn build(&self, app: &mut App) {
		let settings = DomeSettings {
			inner_fade_m: self.inner_fade_m,
			outer_radius_m: self.outer_radius_m.max(self.inner_fade_m + 1.0),
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
	outer_radius_m: f32,
	color: Color,
}

fn spawn_sky_dome(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	settings: Res<DomeSettings>,
) {
	let mesh =
		meshes.add(fade_sphere(settings.outer_radius_m, settings.inner_fade_m, settings.color));
	let material = materials.add(StandardMaterial {
		base_color: Color::WHITE,
		unlit: true,
		alpha_mode: AlphaMode::Blend,
		cull_mode: None,
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

/// UV sphere. Vertex alpha rises with XZ radius so the horizon washes blue
/// and the volume over the viewer stays clear.
fn fade_sphere(outer_m: f32, inner_m: f32, color: Color) -> Mesh {
	let rings = 24u32;
	let segs = 48u32;
	let rgba = color.to_linear();
	let span = (outer_m - inner_m).max(1.0);

	let mut positions = Vec::new();
	let mut normals = Vec::new();
	let mut colors = Vec::new();
	let mut indices = Vec::new();

	for ring in 0..=rings {
		let v = ring as f32 / rings as f32;
		let theta = v * PI;
		let y = outer_m * theta.cos();
		let ring_r = outer_m * theta.sin();
		for seg in 0..=segs {
			let u = seg as f32 / segs as f32;
			let phi = u * 2.0 * PI;
			let x = ring_r * phi.cos();
			let z = ring_r * phi.sin();
			positions.push([x, y, z]);
			let len = (x * x + y * y + z * z).sqrt().max(1e-5);
			// Inward normals: we look at the inner surface.
			normals.push([-x / len, -y / len, -z / len]);
			let xz = (x * x + z * z).sqrt();
			let alpha = ((xz - inner_m) / span).clamp(0.0, 1.0);
			colors.push([rgba.red, rgba.green, rgba.blue, alpha]);
		}
	}

	let verts_per_ring = segs + 1;
	for ring in 0..rings {
		for seg in 0..segs {
			let a = ring * verts_per_ring + seg;
			let b = a + verts_per_ring;
			// Reverse winding so faces point inward.
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
	fn fade_sphere_has_inward_alpha_ramp() {
		let mesh = fade_sphere(2_000.0, 800.0, SKY_BLUE);
		let colors = mesh.attribute(Mesh::ATTRIBUTE_COLOR).expect("colors");
		let bevy::mesh::VertexAttributeValues::Float32x4(colors) = colors else {
			panic!("expected rgba colors");
		};
		assert!(colors.iter().any(|c| c[3] < 0.05), "near-axis vertices stay clear");
		assert!(colors.iter().any(|c| c[3] > 0.95), "horizon vertices wash opaque");
	}
}
