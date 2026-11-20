//! CPU ball-stick preview of the existing small-bush path (tapered sticks + crown balls).
//!
//! Used for matched-camera before/after captures. This is the authored topology, not the
//! Chico GLB kit host.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use vegetation_bush_field::BushPatch;

#[derive(Component)]
pub struct KitPreview;

pub fn clear_kit(world: &mut World) {
	let ids: Vec<_> = world.query_filtered::<Entity, With<KitPreview>>().iter(world).collect();
	for id in ids {
		world.entity_mut(id).despawn();
	}
}

pub fn spawn_kit_from_patches(world: &mut World) {
	clear_kit(world);
	let patches: Vec<BushPatch> = world
		.query::<&BushPatch>()
		.iter(world)
		.cloned()
		.collect();
	if patches.is_empty() {
		return;
	}
	let mut meshes = world.resource_mut::<Assets<Mesh>>();
	let cylinder = meshes.add(Mesh::from(Cylinder::new(1.0, 1.0)));
	let sphere = meshes.add(Mesh::from(Sphere::new(1.0)));
	drop(meshes);
	let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
	let wood = materials.add(StandardMaterial {
		base_color: Color::srgb(0.32, 0.18, 0.10),
		perceptual_roughness: 0.85,
		..default()
	});
	let leaf = materials.add(StandardMaterial {
		base_color: Color::srgb(0.28, 0.52, 0.14),
		perceptual_roughness: 0.7,
		cull_mode: None,
		double_sided: true,
		..default()
	});
	drop(materials);

	for patch in &patches {
		for plant in patch.plants() {
			for branch in &plant.branches {
				let a = branch.world_sample(&plant.branches, &plant.descriptor, 0.02, Vec2::ZERO);
				let b = branch.world_sample(&plant.branches, &plant.descriptor, 1.0, Vec2::ZERO);
				let delta = b.position - a.position;
				let length = delta.length().max(0.04);
				let mid = (a.position + b.position) * 0.5;
				let radius = (branch.root_radius_m + branch.tip_radius_m) * 0.5;
				let rotation = Quat::from_rotation_arc(Vec3::Y, delta.normalize_or(Vec3::Y));
				world.spawn((
					KitPreview,
					Mesh3d(cylinder.clone()),
					MeshMaterial3d(wood.clone()),
					Transform {
						translation: mid,
						rotation,
						scale: Vec3::new(radius, length, radius),
					},
					NotShadowCaster,
				));
			}
			for leaf_desc in &plant.leaves {
				let (origin, _) =
					leaf_desc.world_base(&plant.branches, &plant.descriptor, Vec2::ZERO);
				world.spawn((
					KitPreview,
					Mesh3d(sphere.clone()),
					MeshMaterial3d(leaf.clone()),
					Transform::from_translation(origin)
						.with_scale(Vec3::splat((leaf_desc.size_m * 0.42).clamp(0.035, 0.11))),
					NotShadowCaster,
				));
			}
		}
	}
}
