//! CPU ball-stick preview of the existing Storybook Tree path.
//!
//! Used for matched-camera before/after captures. This is the authored kit
//! topology (sticks + cheap-ball canopy), not a Chico GLB host.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use lod::scene::LodSceneLevel;
use sbs_geometry::StorybookTreeSbs;
use sbs_trees::{StorybookTree, StorybookTreeParams};
use vegetation_components::{FoliageGeometry, VegetationComponents};
use vegetation_tree_field::TreePatch;

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
	let patches: Vec<TreePatch> = world.query::<&TreePatch>().iter(world).cloned().collect();
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
			let mut geometry = StorybookTreeSbs::default();
			geometry.scale.tree_height = plant.descriptor.height_m.max(1.0);
			geometry.canopy_noise.seed = plant.descriptor.plant_id as i32;
			let tree = StorybookTree::from_params(&StorybookTreeParams { geometry });
			let root = plant.descriptor.root;
			for node in tree.stick_nodes_for_level(LodSceneLevel::High).flatten() {
				if let Some(collection) = node.collection {
					for member in collection.members {
						spawn_stick(world, cylinder.clone(), wood.clone(), root, member.placement);
					}
				} else {
					spawn_stick(world, cylinder.clone(), wood.clone(), root, node.placement);
				}
			}
			for node in tree.foliage_nodes_for_level(LodSceneLevel::High).flatten() {
				match node.geometry {
					FoliageGeometry::CheapBall | FoliageGeometry::LayeredBall => {
						spawn_ball(world, sphere.clone(), leaf.clone(), root, node.placement);
					}
					FoliageGeometry::CheapBallCollection(collection) => {
						for placement in collection.placements {
							spawn_ball(world, sphere.clone(), leaf.clone(), root, placement);
						}
					}
					_ => spawn_ball(world, sphere.clone(), leaf.clone(), root, node.placement),
				}
			}
		}
	}
}

fn spawn_stick(
	world: &mut World,
	mesh: Handle<Mesh>,
	material: Handle<StandardMaterial>,
	root: Vec3,
	placement: vegetation_components::Placement,
) {
	world.spawn((
		KitPreview,
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform {
			translation: root
				+ placement.translation
				+ placement.rotation() * Vec3::Y * (placement.scale.y * 0.5),
			rotation: placement.rotation(),
			scale: placement.scale,
		},
		NotShadowCaster,
	));
}

fn spawn_ball(
	world: &mut World,
	mesh: Handle<Mesh>,
	material: Handle<StandardMaterial>,
	root: Vec3,
	placement: vegetation_components::Placement,
) {
	world.spawn((
		KitPreview,
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform {
			translation: root + placement.translation,
			rotation: placement.rotation(),
			scale: placement.scale,
		},
		NotShadowCaster,
	));
}
