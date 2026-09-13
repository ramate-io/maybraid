//! Procedural cuboid stand-ins with deferred [`MaterialRef`] paint.
//!
//! Cuboids occupy the remapped kit AABB in [`crate::kit_space`]. Swap in
//! [`crate::parts::PartKind::asset_path`] GLBs later; do not merge scenes.

use bevy::prelude::{
	Handle, Mesh, Mesh3d, MeshMaterial3d, StandardMaterial, Transform, Visibility,
};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::LodLazyPending;
use material_ref::{MaterialRef, MaterialRefRoot};
use richmond_building_components::{pose, scene_children, FurnitureNode};

use crate::fill::{abutment_wall, posed_assembly};
use crate::parts::PlacedPart;
use crate::plugin::FurnitureKitMeshes;

/// Procedural mesh with a placeholder [`StandardMaterial`] and deferred paint.
pub fn posed_mesh_material_ref(
	mesh: Handle<Mesh>,
	placeholder: Handle<StandardMaterial>,
	material: MaterialRef,
	transform: Transform,
) -> impl Scene + 'static {
	bsn! {
		Mesh3d({mesh})
		MeshMaterial3d::<StandardMaterial>({placeholder})
		template_value(MaterialRefRoot(material))
		LodLazyPending
		template_value(transform)
		Visibility::default()
	}
}

/// Instance each part as its own posed cuboid (no [`scene_ref`] merge).
pub fn assembly_scene(parts: &[PlacedPart], kits: &FurnitureKitMeshes) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = parts
		.iter()
		.map(|part| {
			Box::new(posed_mesh_material_ref(
				kits.unit_cube.clone(),
				kits.placeholder.clone(),
				part.material.clone(),
				pose(part.placement),
			)) as Box<dyn Scene>
		})
		.collect();
	scene_children(children)
}

/// Slot wireframe + painted parts + optional abutment wall.
pub fn filled_slot_scene(node: &FurnitureNode, kits: &FurnitureKitMeshes) -> impl Scene + 'static {
	use richmond_building_components::furniture::FurnitureWireframeAssets;
	use richmond_building_components::wireframe_box_with_handles;

	let mut children: Vec<Box<dyn Scene>> = vec![Box::new(wireframe_box_with_handles(
		FurnitureWireframeAssets::unit_cube(),
		FurnitureWireframeAssets::material_for(node.geometry),
		pose(node.placement),
	))];
	if let Some(parts) = posed_assembly(node) {
		children.push(Box::new(assembly_scene(&parts, kits)));
	}
	if let Some(wall) = abutment_wall(node) {
		children.push(Box::new(wireframe_box_with_handles(
			kits.unit_cube.clone(),
			kits.wall.clone(),
			pose(wall),
		)));
	}
	scene_children(children)
}

/// Group several filled slots under one root.
pub fn filled_slots_scene<'a>(
	nodes: impl IntoIterator<Item = &'a FurnitureNode>,
	kits: &FurnitureKitMeshes,
) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = nodes
		.into_iter()
		.map(|node| Box::new(filled_slot_scene(node, kits)) as Box<dyn Scene>)
		.collect();
	scene_children(children)
}
