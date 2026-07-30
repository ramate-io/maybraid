//! Debug corner markers for [`super::TessellatedTrianglePanel`] (A red, B blue, C green).

use std::sync::OnceLock;

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

static SPHERE: OnceLock<Handle<Mesh>> = OnceLock::new();
static RED: OnceLock<Handle<StandardMaterial>> = OnceLock::new();
static BLUE: OnceLock<Handle<StandardMaterial>> = OnceLock::new();
static GREEN: OnceLock<Handle<StandardMaterial>> = OnceLock::new();

const MARKER_RADIUS: f32 = 0.12;

fn unlit(color: Color) -> StandardMaterial {
	StandardMaterial {
		base_color: color,
		unlit: true,
		..default()
	}
}

fn init(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
	let _ = SPHERE.set(meshes.add(Mesh::from(Sphere::new(MARKER_RADIUS))));
	let _ = RED.set(materials.add(unlit(Color::srgb(0.95, 0.15, 0.12))));
	let _ = BLUE.set(materials.add(unlit(Color::srgb(0.15, 0.35, 0.95))));
	let _ = GREEN.set(materials.add(unlit(Color::srgb(0.15, 0.85, 0.25))));
}

fn sphere() -> Handle<Mesh> {
	SPHERE
		.get()
		.expect("TessellatedTrianglePanelDebugPlugin must run before panel scenes")
		.clone()
}

fn marker_scene(
	mesh: Handle<Mesh>,
	material: Handle<StandardMaterial>,
	at: Vec3,
) -> Box<dyn Scene> {
	Box::new(bsn! {
		Mesh3d({mesh})
		MeshMaterial3d::<StandardMaterial>({material})
		template_value(Transform::from_translation(at))
		Visibility::default()
	})
}

/// A = red, B = blue, C = green (world-space positions).
pub(super) fn corner_marker_scenes(a: Vec3, b: Vec3, c: Vec3) -> Vec<Box<dyn Scene>> {
	let mesh = sphere();
	vec![
		marker_scene(mesh.clone(), RED.get().expect("red").clone(), a),
		marker_scene(mesh.clone(), BLUE.get().expect("blue").clone(), b),
		marker_scene(mesh, GREEN.get().expect("green").clone(), c),
	]
}

/// Registers sphere mesh + RGB materials for corner debug markers.
pub struct TessellatedTrianglePanelDebugPlugin;

impl Plugin for TessellatedTrianglePanelDebugPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, init_corner_markers);
	}
}

fn init_corner_markers(mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
	init(&mut meshes, &mut materials);
}
