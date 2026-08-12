//! Helpers for flattening nested tree/bush [`VegetationComponents`] into grove hosts.

use std::collections::HashMap;

use bevy::prelude::{Color, Vec3};
use chico_vegetation_components::{
	chico_leaf_material_ref, chico_stick_material_ref, FoliageGeometry, FoliageNode, Layers,
	Placement, StickNode, VegetationComponents,
};
use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;

use super::{GroveExtent, PaletteMix};

/// World-metre bin size for UltraLow merged canopy balls.
pub const ULTRA_LOW_CANOPY_BIN_METERS: f32 = 8.0;

/// One plant's canopy proxy (world space) for grove Low / UltraLow.
#[derive(Clone, Debug)]
pub struct CanopyProxySite {
	pub center: Vec3,
	pub radius: f32,
	pub material: MaterialRef,
}

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

/// World canopy proxy from a plant's [`VegetationComponents::structural_lod`].
pub fn canopy_proxy_site(
	plant: &impl VegetationComponents,
	plant_placement: Placement,
	material: &MaterialRef,
) -> Option<CanopyProxySite> {
	let lod = plant.structural_lod()?;
	let scale = plant_placement.scale.abs().max_element().max(1e-4);
	let center = plant_placement
		.compose_child(Placement::new(lod.center, 0.0))
		.translation;
	Some(CanopyProxySite {
		center,
		radius: (lod.tree_radius * scale).max(0.25),
		material: material.clone(),
	})
}

/// Like [`canopy_proxy_site`], with an extra local child pose (e.g. crown at trunk tip).
pub fn canopy_proxy_site_nested(
	plant: &impl VegetationComponents,
	plant_placement: Placement,
	local: Placement,
	material: &MaterialRef,
) -> Option<CanopyProxySite> {
	canopy_proxy_site(plant, plant_placement.compose_child(local), material)
}

/// Grove Low: one cheap ball per plant at canopy size.
pub fn foliage_low_canopy_balls(
	sites: impl IntoIterator<Item = CanopyProxySite>,
) -> Vec<FoliageNode> {
	sites
		.into_iter()
		.map(|site| {
			FoliageNode::cheap_ball(Placement::foliage_uniform(site.center, site.radius))
				.with_material(site.material)
		})
		.collect()
}

/// Grove UltraLow: merge nearby canopy sites into larger balls; one color per bin.
pub fn foliage_ultra_low_merged_balls(
	sites: &[CanopyProxySite],
	bin_meters: f32,
) -> Vec<FoliageNode> {
	let bin = bin_meters.max(1.0);
	let mut bins: HashMap<(i32, i32), (Vec3, f32, MaterialRef, u32)> = HashMap::new();
	for site in sites {
		let ix = (site.center.x / bin).floor() as i32;
		let iz = (site.center.z / bin).floor() as i32;
		let entry = bins.entry((ix, iz)).or_insert_with(|| {
			(Vec3::ZERO, 0.0, site.material.clone(), 0)
		});
		entry.0 += site.center;
		entry.1 = entry.1.max(site.radius);
		entry.3 = entry.3.saturating_add(1);
		// Keep first material in the bin (stable pick).
	}

	bins.into_iter()
		.map(|((ix, iz), (sum, max_r, material, count))| {
			let n = (count as f32).max(1.0);
			let mean = sum / n;
			let cx = (ix as f32 + 0.5) * bin;
			let cz = (iz as f32 + 0.5) * bin;
			let center = Vec3::new(cx, mean.y, cz).lerp(mean, 0.35);
			let radius = (max_r.max(bin * 0.35) * n.sqrt()).max(0.5);
			FoliageNode::cheap_ball(Placement::foliage_uniform(center, radius))
				.with_material(material)
		})
		.collect()
}

/// Map grove structural level: High/Medium keep plant emit; Low/UltraLow use canopy proxies.
pub fn grove_detail_level(level: LodSceneLevel) -> Option<LodSceneLevel> {
	match level {
		LodSceneLevel::High | LodSceneLevel::Medium => Some(level),
		LodSceneLevel::Low
		| LodSceneLevel::UltraLow
		| LodSceneLevel::Distance(_)
		| LodSceneLevel::Resolution(_) => None,
	}
}
