//! Far-band primitive massing for [`crate::ComponentsOnly`] buildings.
//!
//! Low draws one cuboid per structural footprint. UltraLow draws the union box.
//! Shared unit mesh + material must be initialized by [`MassingSilhouettePlugin`].

use std::sync::OnceLock;

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::bounding::Aabb2d;
use lod::gen::LodSceneLevel;

use crate::scene_children::scene_children;
use crate::structural_probe::BuildingStructuralLodProbe;
use crate::BuildingComponents;

const MIN_EDGE: f32 = 0.25;

/// Shared unit cuboid massing (must be initialized by [`MassingSilhouettePlugin`]).
#[derive(Resource, Debug, Clone)]
pub struct MassingSilhouetteAssets {
	pub cuboid: Handle<Mesh>,
	pub material: Handle<StandardMaterial>,
}

static CUBOID: OnceLock<Handle<Mesh>> = OnceLock::new();
static MATERIAL: OnceLock<Handle<StandardMaterial>> = OnceLock::new();

impl MassingSilhouetteAssets {
	pub fn cuboid() -> Handle<Mesh> {
		CUBOID
			.get()
			.expect("MassingSilhouettePlugin must run before massing scenes")
			.clone()
	}

	pub fn material() -> Handle<StandardMaterial> {
		MATERIAL
			.get()
			.expect("MassingSilhouettePlugin must run before massing scenes")
			.clone()
	}

	fn init(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
		let _ = CUBOID.set(meshes.add(Mesh::from(Cuboid::from_length(1.0))));
		let _ = MATERIAL.set(materials.add(StandardMaterial {
			base_color: Color::srgb(0.48, 0.44, 0.40),
			perceptual_roughness: 0.92,
			..default()
		}));
	}
}

/// Named bands that replace nested kit hosts with primitive boxes.
pub fn is_massing_level(level: LodSceneLevel) -> bool {
	matches!(
		level,
		LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_)
	)
}

/// Unit-cube transform: translation is the box center; scale is full edge lengths.
pub fn massing_box_transform(rect: &Aabb2d, y0: f32, height: f32) -> Transform {
	let sx = (rect.max.x - rect.min.x).max(MIN_EDGE);
	let sz = (rect.max.y - rect.min.y).max(MIN_EDGE);
	let sy = height.max(MIN_EDGE);
	Transform {
		translation: Vec3::new(
			(rect.min.x + rect.max.x) * 0.5,
			y0 + sy * 0.5,
			(rect.min.y + rect.max.y) * 0.5,
		),
		scale: Vec3::new(sx, sy, sz),
		..Transform::default()
	}
}

/// Posed massing cuboid (unit mesh scaled by `transform`).
pub fn massing_box_scene(transform: Transform) -> impl Scene + 'static {
	let mesh = MassingSilhouetteAssets::cuboid();
	let material = MassingSilhouetteAssets::material();
	bsn! {
		Mesh3d({mesh})
		MeshMaterial3d::<StandardMaterial>({material})
		NotShadowCaster
		template_value(transform)
		Visibility::Inherited
	}
}

fn massing_vertical(
	building: &impl BuildingComponents,
	probe: &BuildingStructuralLodProbe,
) -> (f32, f32) {
	let nodes = crate::building_bounds(building);
	let y0 = nodes.min.y.min(probe.y0);
	let top = nodes.max.y.max(probe.y0 + probe.height);
	(y0, (top - y0).max(MIN_EDGE))
}

/// Primitive boxes for a far structural band. Empty when the building has no probe.
pub fn massing_scene(
	building: &impl BuildingComponents,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	let Some(probe) = building.structural_lod() else {
		return scene_children(Vec::new());
	};
	let (y0, height) = massing_vertical(building, &probe);
	let children: Vec<Box<dyn Scene>> = probe
		.massing_footprints(level)
		.iter()
		.map(|rect| {
			Box::new(massing_box_scene(massing_box_transform(rect, y0, height))) as Box<dyn Scene>
		})
		.collect();
	scene_children(children)
}

pub struct MassingSilhouettePlugin;

impl Plugin for MassingSilhouettePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, init_massing_silhouettes);
	}
}

fn init_massing_silhouettes(
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	MassingSilhouetteAssets::init(&mut meshes, &mut materials);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn box_transform_centers_and_scales() {
		let rect = Aabb2d { min: Vec2::new(-4.0, -2.0), max: Vec2::new(4.0, 2.0) };
		let tf = massing_box_transform(&rect, 1.0, 6.0);
		assert!((tf.translation - Vec3::new(0.0, 4.0, 0.0)).length() < 1e-4);
		assert!((tf.scale - Vec3::new(8.0, 6.0, 4.0)).length() < 1e-4);
	}

	#[test]
	fn massing_levels() {
		assert!(is_massing_level(LodSceneLevel::Low));
		assert!(is_massing_level(LodSceneLevel::UltraLow));
		assert!(!is_massing_level(LodSceneLevel::High));
		assert!(!is_massing_level(LodSceneLevel::Medium));
	}
}
