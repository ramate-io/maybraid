//! Helpers for flattening nested tree/bush [`VegetationComponents`] into grove hosts.

use std::collections::HashMap;

use bevy::prelude::{Color, Vec3};
use bevy::scene::prelude::Scene;
use chico_vegetation_components::{
	chico_frond_material_ref, chico_leaf_material_ref, chico_stick_material_ref,
	components_only_host, flattened_components_only_host, FoliageGeometry, FoliageNode, Layers,
	PlacedVegetation, Placement, StickNode, StructuralLod, VegetationComponents,
};
use lod::gen::{LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::{cull_offset_bands_from_factor, SceneChunk};
use material_ref::MaterialRef;

use super::{GroveExtent, PaletteMix, DEFAULT_GROVE_EXTENT_XZ};

/// World-metre bin size for UltraLow merged canopy balls.
pub const ULTRA_LOW_CANOPY_BIN_METERS: f32 = 8.0;

/// Storybook / tall-woody plant Medium used in the tile-band floor.
pub const DEFAULT_PLANT_MEDIUM_FACTOR: f32 = 30.0;

/// Column proxy XZ half-extent as a fraction of characteristic half-height.
const COLUMN_XZ_OF_HALF_HEIGHT: f32 = 0.28;
/// Waialea / tall-palm trunk column: thinner than a conifer, still readable at Low.
const PALM_TRUNK_XZ_OF_HALF_HEIGHT: f32 = 0.10;
/// Crown ball radius as a fraction of characteristic half-height.
const CROWN_RADIUS_OF_HALF: f32 = 0.42;
/// Lift from a mid-tree sphere center toward the canopy, in half-heights.
const CROWN_LIFT_OF_HALF: f32 = 0.65;

/// One plant's canopy proxy (world space) for grove Low / UltraLow.
///
/// `half_extents` is the cheap-ball scale: a sphere uses a uniform value, a conifer
/// column is long on Y, a palm crown is a small ball near the top.
#[derive(Clone, Debug)]
pub struct CanopyProxySite {
	pub center: Vec3,
	pub half_extents: Vec3,
	pub material: MaterialRef,
}

impl CanopyProxySite {
	pub fn from_radius(center: Vec3, radius: f32, material: MaterialRef) -> Self {
		Self { center, half_extents: Vec3::splat(radius.max(0.25)), material }
	}

	/// Characteristic radius for UltraLow bins / distance (`max` axis).
	pub fn radius(&self) -> f32 {
		self.half_extents.max_element()
	}

	fn is_uniform(&self) -> bool {
		let he = self.half_extents;
		(he.x - he.y).abs() < 1e-3 && (he.x - he.z).abs() < 1e-3
	}
}

/// Round a tile factor up to a readable 5 m step (floor 5).
fn round_up_band(value: f32) -> f32 {
	if value <= 5.0 {
		5.0
	} else if value <= 10.0 {
		10.0
	} else {
		(value / 5.0).ceil() * 5.0
	}
}

/// Tile High / Medium / Low for a default 100 m grove ([`DEFAULT_GROVE_EXTENT_XZ`]).
///
/// Medium is the plant→proxy edge. Floor from the large-tree rule of thumb:
/// `medium ≳ plant_medium × (typical_height / 2) / tile_radius + 1`.
/// High stays a short kit band; Low is `1.5 ×` Medium so UltraLow does not slam in.
pub fn grove_bands_for_typical_height(typical_height_m: f32) -> (f32, f32, f32) {
	grove_bands_for_typical_height_and_plant_medium(typical_height_m, DEFAULT_PLANT_MEDIUM_FACTOR)
}

/// Like [`grove_bands_for_typical_height`], with an explicit plant Medium (palms use 36).
pub fn grove_bands_for_typical_height_and_plant_medium(
	typical_height_m: f32,
	plant_medium_factor: f32,
) -> (f32, f32, f32) {
	let tile_radius = DEFAULT_GROVE_EXTENT_XZ * 0.5;
	let raw = plant_medium_factor * (typical_height_m.max(1.0) * 0.5) / tile_radius + 1.0;
	let medium = round_up_band(raw.max(5.0));
	let high = if medium >= 50.0 {
		10.0
	} else if medium >= 8.0 {
		5.0
	} else {
		2.0
	};
	let low = round_up_band(medium * 1.5);
	(high, medium, low)
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

/// Frond material: Chico frond recipe with one palette-picked color.
pub fn frond_material_from_palette(palette: Option<PaletteMix>, seed: i32) -> MaterialRef {
	palette
		.and_then(|p| p.pick_color(seed))
		.map(|c| chico_frond_material_ref().with_palette([c]))
		.unwrap_or_else(chico_frond_material_ref)
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

fn composed_lod_center(lod_center: Vec3, plant_placement: Placement) -> (Vec3, f32) {
	let scale = plant_placement.scale.abs().max_element().max(1e-4);
	let center = plant_placement.compose_child(Placement::new(lod_center, 0.0)).translation;
	(center, scale)
}

/// Mid-tree characteristic sphere: `center.y ≈ tree_radius` after `from_extent`.
fn is_mid_tree_sphere(lod: &StructuralLod) -> bool {
	let r = lod.tree_radius.max(1e-4);
	(lod.center.y - r).abs() < r * 0.28
}

fn foliage_placement_for_proxy(site: &CanopyProxySite) -> Placement {
	if site.is_uniform() {
		Placement::foliage_uniform(site.center, site.radius())
	} else {
		Placement::new(site.center, 0.0).with_scale(site.half_extents.max(Vec3::splat(0.25)))
	}
}

/// World canopy proxy from a plant's [`VegetationComponents::structural_lod`].
///
/// Broadleaf default: a sphere at the lod center with characteristic radius.
pub fn canopy_proxy_site(
	plant: &impl VegetationComponents,
	plant_placement: Placement,
	material: &MaterialRef,
) -> Option<CanopyProxySite> {
	let lod = plant.structural_lod()?;
	let (center, scale) = composed_lod_center(lod.center, plant_placement);
	Some(CanopyProxySite::from_radius(
		center,
		(lod.tree_radius * scale).max(0.25),
		material.clone(),
	))
}

/// Long thin proxy for conifers: full height, footprint much smaller than characteristic radius.
pub fn canopy_proxy_column(
	plant: &impl VegetationComponents,
	plant_placement: Placement,
	material: &MaterialRef,
) -> Option<CanopyProxySite> {
	let lod = plant.structural_lod()?;
	let (center, scale) = composed_lod_center(lod.center, plant_placement);
	let half_h = (lod.tree_radius * scale).max(0.25);
	let xz = (half_h * COLUMN_XZ_OF_HALF_HEIGHT).max(0.25);
	Some(CanopyProxySite {
		center,
		half_extents: Vec3::new(xz, half_h, xz),
		material: material.clone(),
	})
}

/// Crown-only proxy for palms: a ball at the frond cluster, not a mid-trunk sphere.
pub fn canopy_proxy_crown(
	plant: &impl VegetationComponents,
	plant_placement: Placement,
	material: &MaterialRef,
) -> Option<CanopyProxySite> {
	let lod = plant.structural_lod()?;
	let half = lod.tree_radius.max(1e-4);
	let local = if is_mid_tree_sphere(&lod) {
		Vec3::new(lod.center.x, lod.center.y + half * CROWN_LIFT_OF_HALF, lod.center.z)
	} else {
		lod.center
	};
	let (center, scale) = composed_lod_center(local, plant_placement);
	Some(CanopyProxySite::from_radius(
		center,
		(half * CROWN_RADIUS_OF_HALF * scale).max(0.2),
		material.clone(),
	))
}

/// Thin trunk column from the plant base to the crown probe (Y-up).
///
/// Waialea Low has no mid-tree canopy; a crown ball alone floats. Pair with
/// [`canopy_proxy_crown`] via [`canopy_proxy_waialea`].
pub fn canopy_proxy_trunk(
	plant: &impl VegetationComponents,
	plant_placement: Placement,
	material: &MaterialRef,
) -> Option<CanopyProxySite> {
	let lod = plant.structural_lod()?;
	let ground = plant_placement.translation;
	let (crown, _) = composed_lod_center(lod.center, plant_placement);
	let half_h = (crown.y - ground.y).abs() * 0.5;
	if half_h < 0.2 {
		return None;
	}
	let mid = (ground + crown) * 0.5;
	let xz = (half_h * PALM_TRUNK_XZ_OF_HALF_HEIGHT).max(0.12);
	Some(CanopyProxySite {
		center: mid,
		half_extents: Vec3::new(xz, half_h, xz),
		material: material.clone(),
	})
}

/// Waialea Low / UltraLow: trunk column (stick material) + crown ball (canopy material).
pub fn canopy_proxy_waialea(
	plant: &impl VegetationComponents,
	plant_placement: Placement,
	trunk_material: &MaterialRef,
	crown_material: &MaterialRef,
) -> Vec<CanopyProxySite> {
	let mut sites = Vec::with_capacity(2);
	if let Some(trunk) = canopy_proxy_trunk(plant, plant_placement, trunk_material) {
		sites.push(trunk);
	}
	if let Some(crown) = canopy_proxy_crown(plant, plant_placement, crown_material) {
		sites.push(crown);
	}
	sites
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

/// Grove Low: one cheap ball per plant. Uniform sites tumble; columns stay Y-up.
pub fn foliage_low_canopy_balls(
	sites: impl IntoIterator<Item = CanopyProxySite>,
) -> Vec<FoliageNode> {
	sites
		.into_iter()
		.map(|site| {
			FoliageNode::cheap_ball(foliage_placement_for_proxy(&site)).with_material(site.material)
		})
		.collect()
}

/// Grove UltraLow: merge nearby canopy sites into larger balls; one color per bin.
pub fn foliage_ultra_low_merged_balls(
	sites: &[CanopyProxySite],
	bin_meters: f32,
) -> Vec<FoliageNode> {
	let bin = bin_meters.max(1.0);
	let mut bins: HashMap<(i32, i32), (Vec3, Vec3, MaterialRef, u32)> = HashMap::new();
	for site in sites {
		let ix = (site.center.x / bin).floor() as i32;
		let iz = (site.center.z / bin).floor() as i32;
		let entry = bins
			.entry((ix, iz))
			.or_insert_with(|| (Vec3::ZERO, Vec3::ZERO, site.material.clone(), 0));
		entry.0 += site.center;
		entry.1 = entry.1.max(site.half_extents);
		entry.3 = entry.3.saturating_add(1);
		// Keep first material in the bin (stable pick).
	}

	bins.into_iter()
		.map(|((ix, iz), (sum, max_he, material, count))| {
			let n = (count as f32).max(1.0);
			let mean = sum / n;
			let cx = (ix as f32 + 0.5) * bin;
			let cz = (iz as f32 + 0.5) * bin;
			let center = Vec3::new(cx, mean.y, cz).lerp(mean, 0.35);
			let half_extents = Vec3::new(
				(max_he.x.max(bin * 0.35) * n.sqrt()).max(0.5),
				max_he.y.max(0.5),
				(max_he.z.max(bin * 0.35) * n.sqrt()).max(0.5),
			);
			FoliageNode::cheap_ball(foliage_placement_for_proxy(&CanopyProxySite {
				center,
				half_extents,
				material: material.clone(),
			}))
			.with_material(material)
		})
		.collect()
}

/// Map grove structural level: High/Medium nest plant hosts; Low/UltraLow use canopy proxies.
pub fn grove_detail_level(level: LodSceneLevel) -> Option<LodSceneLevel> {
	match level {
		LodSceneLevel::High | LodSceneLevel::Medium => Some(level),
		LodSceneLevel::Low
		| LodSceneLevel::UltraLow
		| LodSceneLevel::Distance(_)
		| LodSceneLevel::Resolution(_) => None,
	}
}

/// Nest one posed plant as [`chico_vegetation_components::ComponentsOnly`]`<`[`PlacedVegetation`]`<T>>`.
pub fn nest_placed_plant_host<T>(
	plant: T,
	placement: Placement,
	stick_material: &MaterialRef,
	ball_material: &MaterialRef,
	frond_material: &MaterialRef,
	lod_ref: &LodRef,
) -> impl Scene + 'static
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	components_only_host(
		PlacedVegetation::new(
			plant,
			placement,
			stick_material.clone(),
			ball_material.clone(),
			frond_material.clone(),
		),
		lod_ref,
	)
}

/// Weighted chunk wrapping [`nest_placed_plant_host`].
pub fn nest_placed_plant_chunk<T>(
	plant: T,
	placement: Placement,
	stick_material: &MaterialRef,
	ball_material: &MaterialRef,
	frond_material: &MaterialRef,
	lod_ref: &LodRef,
) -> SceneChunk
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	SceneChunk::weighted(
		1,
		nest_placed_plant_host(
			plant,
			placement,
			stick_material,
			ball_material,
			frond_material,
			lod_ref,
		),
	)
}

/// Nest one posed plant as [`chico_vegetation_components::FlattenedComponentsOnly`]`<`[`PlacedVegetation`]`<T>>`.
///
/// Kit nodes spawn as posed content (no per-stick / per-ball LOD hosts).
pub fn nest_flattened_plant_host<T>(
	plant: T,
	placement: Placement,
	stick_material: &MaterialRef,
	ball_material: &MaterialRef,
	frond_material: &MaterialRef,
	lod_ref: &LodRef,
) -> impl Scene + 'static
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	flattened_components_only_host(
		PlacedVegetation::new(
			plant,
			placement,
			stick_material.clone(),
			ball_material.clone(),
			frond_material.clone(),
		),
		lod_ref,
	)
}

/// Weighted chunk wrapping [`nest_flattened_plant_host`].
pub fn nest_flattened_plant_chunk<T>(
	plant: T,
	placement: Placement,
	stick_material: &MaterialRef,
	ball_material: &MaterialRef,
	frond_material: &MaterialRef,
	lod_ref: &LodRef,
) -> SceneChunk
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	SceneChunk::weighted(
		1,
		nest_flattened_plant_host(
			plant,
			placement,
			stick_material,
			ball_material,
			frond_material,
			lod_ref,
		),
	)
}

pub fn grove_lod_level(band: StructuralLod, lod_ref: &LodRef) -> LodSceneLevel {
	band.level_for(lod_ref.current_transform)
}

pub fn grove_lod_status(band: StructuralLod, lod_ref: &LodRef) -> LodSceneStatus {
	band.status_for_lod_ref(lod_ref)
}

pub fn grove_lod_culls(band: StructuralLod, lod_ref: &LodRef) -> LodSceneCulls {
	let factor =
		lod_ref.current_transform.translation.distance(band.center) / band.tree_radius.max(1e-4);
	cull_offset_bands_from_factor(factor, band.high_factor, band.medium_factor, band.low_factor)
}

/// High/Medium → nested plant host chunks; Low/UltraLow → canopy-ball vegetation chunks.
pub fn woody_grove_scene_chunks(
	level: LodSceneLevel,
	lod_ref: &LodRef,
	plant_chunks: Vec<SceneChunk>,
	vegetation: &impl VegetationComponents,
) -> SceneChunk {
	match grove_detail_level(level) {
		Some(_) => {
			if plant_chunks.is_empty() {
				SceneChunk::primitive(chico_vegetation_components::scene_children(Vec::new()))
			} else {
				SceneChunk::chunks(plant_chunks)
			}
		}
		None => {
			chico_vegetation_components::flattened_canopy_proxy_chunks(vegetation, lod_ref, level)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	struct LodPlant(StructuralLod);

	impl VegetationComponents for LodPlant {
		fn structural_lod(&self) -> Option<StructuralLod> {
			Some(self.0)
		}
	}

	#[test]
	fn grove_bands_scale_with_typical_height() {
		assert_eq!(grove_bands_for_typical_height(180.0), (10.0, 55.0, 85.0));
		assert_eq!(grove_bands_for_typical_height(160.0), (10.0, 50.0, 75.0));
		assert_eq!(grove_bands_for_typical_height(40.0), (5.0, 15.0, 25.0));
		assert_eq!(grove_bands_for_typical_height_and_plant_medium(32.0, 36.0), (5.0, 15.0, 25.0));
		assert_eq!(grove_bands_for_typical_height(20.0), (5.0, 10.0, 15.0));
		assert_ne!(grove_bands_for_typical_height(180.0).1, 20.0);
	}

	#[test]
	fn column_proxy_is_taller_than_wide() {
		let plant = LodPlant(StructuralLod::from_extent(Vec3::Y * 80.0, 20.0, 160.0));
		let site = canopy_proxy_column(&plant, Placement::IDENTITY, &chico_leaf_material_ref())
			.expect("column");
		assert!(site.half_extents.y > site.half_extents.x * 2.0);
		assert!((site.half_extents.x - site.half_extents.z).abs() < 1e-4);
		let nodes = foliage_low_canopy_balls([site]);
		assert_eq!(nodes.len(), 1);
		let p = nodes[0].placement;
		assert!(p.scale.y > p.scale.x * 2.0);
		assert_eq!(p.pitch, 0.0);
		assert_eq!(p.roll, 0.0);
	}

	#[test]
	fn crown_proxy_sits_near_canopy_not_trunk_mid() {
		let mid = StructuralLod::from_extent(Vec3::Y * 20.0, 8.0, 40.0);
		let plant = LodPlant(mid);
		let site = canopy_proxy_crown(&plant, Placement::IDENTITY, &chico_leaf_material_ref())
			.expect("crown");
		assert!(site.center.y > mid.center.y);
		assert!(site.radius() < mid.tree_radius * 0.6);

		let crown_lod = StructuralLod::new(Vec3::Y * 34.0, 20.0);
		let already = canopy_proxy_crown(
			&LodPlant(crown_lod),
			Placement::IDENTITY,
			&chico_leaf_material_ref(),
		)
		.expect("already-crown");
		assert!((already.center.y - 34.0).abs() < 1e-3);
		assert!(already.radius() < 12.0);
	}

	#[test]
	fn waialea_low_proxy_keeps_trunk_and_crown() {
		let crown = StructuralLod::new(Vec3::Y * 10.0, 6.0);
		let plant = LodPlant(crown);
		let placement = Placement::new(Vec3::new(2.0, 0.0, -1.0), 0.0).with_scale(Vec3::splat(2.0));
		let sites = canopy_proxy_waialea(
			&plant,
			placement,
			&chico_stick_material_ref(),
			&chico_leaf_material_ref(),
		);
		assert_eq!(sites.len(), 2);
		let trunk = &sites[0];
		let ball = &sites[1];
		assert!(trunk.half_extents.y > trunk.half_extents.x * 2.0);
		assert!((trunk.center.x - 2.0).abs() < 1.0);
		assert!(trunk.center.y < ball.center.y);
		assert!((ball.half_extents.x - ball.half_extents.z).abs() < 1e-3);
	}

	#[test]
	fn ultra_low_keeps_column_aspect_in_one_bin() {
		let material = chico_leaf_material_ref();
		let sites = [
			CanopyProxySite {
				center: Vec3::new(1.0, 40.0, 1.0),
				half_extents: Vec3::new(8.0, 40.0, 8.0),
				material: material.clone(),
			},
			CanopyProxySite {
				center: Vec3::new(2.0, 42.0, 2.0),
				half_extents: Vec3::new(9.0, 44.0, 9.0),
				material,
			},
		];
		let merged = foliage_ultra_low_merged_balls(&sites, 8.0);
		assert_eq!(merged.len(), 1);
		let p = merged[0].placement;
		assert!(p.scale.y > p.scale.x);
	}

	#[test]
	fn frond_palette_uses_chico_frond_recipe() {
		use crate::grove::{PaletteMix, PaletteSlot};
		use chico_vegetation_components::CHICO_FROND_MATERIAL;
		use material_ref::MaterialId;

		let named = frond_material_from_palette(None, 0);
		assert_eq!(named.name, MaterialId::named(CHICO_FROND_MATERIAL));
		assert!(named.palette.is_empty());

		const MIX: PaletteMix = PaletteMix::new(&[PaletteSlot::new("palm_green", "palm_green")]);
		let tinted = frond_material_from_palette(Some(MIX), 3);
		assert_eq!(tinted.name, MaterialId::named(CHICO_FROND_MATERIAL));
		assert_eq!(tinted.palette.len(), 1);
	}
}
