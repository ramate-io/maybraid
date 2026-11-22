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
use vegetation_tree_field::{TreePatch, TreeRecipe};

#[derive(Component)]
pub struct KitPreview;

pub fn clear_kit(world: &mut World) {
	let ids: Vec<_> = world.query_filtered::<Entity, With<KitPreview>>().iter(world).collect();
	for id in ids {
		world.entity_mut(id).despawn();
	}
}

pub fn spawn_kit_from_height_sites(world: &mut World, sites: &[vegetation_groves::GrovePlantSite]) {
	if sites.is_empty() {
		return;
	}
	let placements: Vec<(Vec3, f32)> =
		sites.iter().map(|site| (site.root, site.height_m)).collect();
	let patch = TreePatch::from_height_sites(
		TreePatch::seed_from_origin(placements[0].0),
		&placements,
		TreeRecipe::storybook(),
	);
	spawn_kit_plants(world, std::slice::from_ref(&patch));
}

pub fn spawn_kit_bushes(world: &mut World, sites: &[vegetation_groves::GrovePlantSite]) {
	let meshes = kit_meshes(world);
	for site in sites {
		spawn_ball(
			world,
			meshes.sphere.clone(),
			meshes.leaf.clone(),
			Vec3::ZERO,
			vegetation_components::Placement::new(site.root, 0.0)
				.with_scale(Vec3::splat(site.height_m.max(0.4) * 0.45)),
		);
	}
}

pub fn spawn_kit_grass(world: &mut World, sites: &[vegetation_groves::GrovePlantSite]) {
	let meshes = kit_meshes(world);
	for site in sites {
		let height = site.height_m.max(0.15);
		spawn_stick(
			world,
			meshes.cylinder.clone(),
			meshes.leaf.clone(),
			Vec3::ZERO,
			vegetation_components::Placement::new(site.root, 0.0)
				.with_scale(Vec3::new(0.03, height, 0.03)),
		);
	}
}

pub fn spawn_kit_from_patches(world: &mut World) {
	clear_kit(world);
	let patches: Vec<TreePatch> = world.query::<&TreePatch>().iter(world).cloned().collect();
	if patches.is_empty() {
		return;
	}
	spawn_kit_plants(world, &patches);
}

struct KitMeshes {
	cylinder: Handle<Mesh>,
	sphere: Handle<Mesh>,
	wood: Handle<StandardMaterial>,
	leaf: Handle<StandardMaterial>,
}

fn kit_meshes(world: &mut World) -> KitMeshes {
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
	KitMeshes { cylinder, sphere, wood, leaf }
}

fn spawn_kit_plants(world: &mut World, patches: &[TreePatch]) {
	let meshes = kit_meshes(world);
	for patch in patches {
		for plant in patch.plants() {
			let mut geometry = StorybookTreeSbs::default();
			geometry.scale.tree_height = plant.descriptor.height_m.max(1.0);
			geometry.canopy_noise.seed = plant.descriptor.plant_id as i32;
			let tree = StorybookTree::from_params(&StorybookTreeParams { geometry });
			let root = plant.descriptor.root;
			for node in tree.stick_nodes_for_level(LodSceneLevel::High).flatten() {
				if let Some(collection) = node.collection {
					for member in collection.members {
						spawn_stick(
							world,
							meshes.cylinder.clone(),
							meshes.wood.clone(),
							root,
							member.placement,
						);
					}
				} else {
					spawn_stick(
						world,
						meshes.cylinder.clone(),
						meshes.wood.clone(),
						root,
						node.placement,
					);
				}
			}
			for node in tree.foliage_nodes_for_level(LodSceneLevel::High).flatten() {
				match node.geometry {
					FoliageGeometry::CheapBall | FoliageGeometry::LayeredBall => {
						spawn_ball(
							world,
							meshes.sphere.clone(),
							meshes.leaf.clone(),
							root,
							node.placement,
						);
					}
					FoliageGeometry::CheapBallCollection(collection) => {
						for placement in collection.placements {
							spawn_ball(
								world,
								meshes.sphere.clone(),
								meshes.leaf.clone(),
								root,
								placement,
							);
						}
					}
					_ => spawn_ball(
						world,
						meshes.sphere.clone(),
						meshes.leaf.clone(),
						root,
						node.placement,
					),
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
