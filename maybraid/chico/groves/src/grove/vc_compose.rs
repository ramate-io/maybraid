//! Helpers for flattening nested tree/bush [`VegetationComponents`] into grove hosts.

use bevy::prelude::{Color, Vec3};
use chico_vegetation_components::{
	chico_leaf_material_ref, chico_stick_material_ref, FoliageGeometry, FoliageNode, Layers,
	Placement, StickNode, VegetationComponents,
};
use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;

use super::{GroveExtent, PaletteMix};

/// Stick material: Chico stick recipe with one palette-picked color.
pub fn stick_material_from_palette(palette: Option<PaletteMix>, seed: i32) -> MaterialRef {
	palette
		.and_then(|p| p.pick_color(seed))
		.map(|c| chico_stick_material_ref().with_palette([c]))
		.unwrap_or_else(chico_stick_material_ref)
}

/// Canopy ball material: Chico leaf recipe with one palette-picked color.
pub fn canopy_ball_material_from_palette(palette: Option<PaletteMix>, seed: i32) -> MaterialRef {
	palette
		.and_then(|p| p.pick_color(seed))
		.map(|c| chico_leaf_material_ref().with_palette([c]))
		.unwrap_or_else(chico_leaf_material_ref)
}

/// Frond / standard foliage material: default green with palette.
pub fn frond_material_from_palette(palette: Option<PaletteMix>, seed: i32) -> MaterialRef {
	palette
		.and_then(|p| p.pick_color(seed))
		.map(|c| MaterialRef::default().with_palette([c]))
		.unwrap_or_default()
}

fn is_frond_geometry(geometry: &FoliageGeometry) -> bool {
	matches!(
		geometry,
		FoliageGeometry::FrondCollection(_)
			| FoliageGeometry::StraightFrond
			| FoliageGeometry::StraightFrondSegment
	)
}

/// Stamp stick nodes: compose plant pose and apply stick material.
pub fn flatten_stick_nodes(
	plant: &impl VegetationComponents,
	plant_placement: Placement,
	stick_material: &MaterialRef,
	level: LodSceneLevel,
) -> Vec<StickNode> {
	plant
		.stick_nodes_for_level(level)
		.flatten()
		.into_iter()
		.map(|mut node| {
			node.placement = plant_placement.compose_child(node.placement);
			node.with_material(stick_material.clone())
		})
		.collect()
}

/// Stamp foliage nodes: compose plant pose; balls → leaf palette, fronds → default palette.
pub fn flatten_foliage_nodes(
	plant: &impl VegetationComponents,
	plant_placement: Placement,
	ball_material: &MaterialRef,
	frond_material: &MaterialRef,
	level: LodSceneLevel,
) -> Vec<FoliageNode> {
	plant
		.foliage_nodes_for_level(level)
		.flatten()
		.into_iter()
		.map(|mut node| {
			node.placement = plant_placement.compose_child(node.placement);
			let material = if is_frond_geometry(&node.geometry) {
				frond_material.clone()
			} else {
				ball_material.clone()
			};
			node.with_material(material)
		})
		.collect()
}

/// Flatten stick + foliage under an extra local child placement (e.g. crown at trunk tip).
pub fn flatten_foliage_nodes_nested(
	plant: &impl VegetationComponents,
	plant_placement: Placement,
	local: Placement,
	ball_material: &MaterialRef,
	frond_material: &MaterialRef,
	level: LodSceneLevel,
) -> Vec<FoliageNode> {
	let composed = plant_placement.compose_child(local);
	flatten_foliage_nodes(plant, composed, ball_material, frond_material, level)
}

pub fn layers_from_nodes<T>(nodes: Vec<T>) -> Layers<T> {
	Layers::from_free(nodes)
}

/// Grove footprint structural center / radius from extent (Monster Grass style).
pub fn grove_structural_footprint(extent: &GroveExtent) -> (Vec3, f32) {
	let span = extent.max() - extent.min();
	let half = span * 0.5;
	let footprint_radius = half.x.max(half.z).max(1.0);
	let center = extent.min() + Vec3::new(half.x, half.y.max(1.0), half.z);
	(center, footprint_radius)
}

/// Convenience: pick color or fall back.
pub fn pick_or_default(palette: Option<PaletteMix>, seed: i32) -> Option<Color> {
	palette.and_then(|p| p.pick_color(seed))
}
