use super::WOODY_LOD;
use std::sync::Arc;

#[cfg(test)]
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_sbs_trees::{DatePalm, DatePalmParams, QuantizedPlant, WaialeaPalm, WaialeaPalmParams};
use chico_vegetation_components::{Placement, VegetationComponents};
use clap::Args;
#[cfg(test)]
use lod::gen::{LodScene, LodSceneLevel};
use lod::lod_ref::LodRef;
use lod::SceneChunk;
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

use super::{
	definition, PalmShadeCell, CLUSTER_DATE_PALM, LOWER_WAIALEA_PALM, SHADE_DATE_PALM,
	TOWER_WAIALEA_PALM,
};
use crate::grove::vc_tuft::patch_variant_index;
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_crown, canopy_proxy_waialea,
	frond_material_from_palette, grove_structural_footprint, nest_flattened_plant_chunk,
	placement_noise, remixed_sbs_plant, stick_material_from_palette, CanopyProxySite,
	FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct PalmShadeParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<PalmShadeCell>,
}

impl Default for PalmShadeParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 }),
		}
	}
}

crate::impl_grove_preview_params!(PalmShadeParams, PalmShadeCell);

impl PalmShadeParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> PalmShade {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> PalmShade {
		PalmShade::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_sbs_plant!(TowerWaialeaPalm, WaialeaPalm, WaialeaPalmParams, TOWER_WAIALEA_PALM);
remixed_sbs_plant!(LowerWaialeaPalm, WaialeaPalm, WaialeaPalmParams, LOWER_WAIALEA_PALM);
remixed_sbs_plant!(ShadeDatePalm, DatePalm, DatePalmParams, SHADE_DATE_PALM);
remixed_sbs_plant!(ClusterDatePalm, DatePalm, DatePalmParams, CLUSTER_DATE_PALM);

#[derive(Clone)]
enum PalmShadeKind {
	Waialea(Arc<WaialeaPalm>),
	Date(Arc<DatePalm>),
}

#[derive(Clone)]
pub struct PalmShadePlant {
	pub placement: Placement,
	kind: PalmShadeKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct PalmShade {
	pub plants: Arc<[PalmShadePlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl PalmShade {
	pub fn from_placements(
		placements: &[GroveCellVariant<PalmShadeCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[PalmShadePlant]> = placements
			.iter()
			.map(|placed| grow_plant(placed, grove_noise, tree_variants))
			.collect::<Vec<_>>()
			.into();
		let (structural_center, footprint_radius) = grove_structural_footprint(extent);
		Self { plants, structural_center, footprint_radius, extent: *extent }
	}

	fn nest_plant_chunks(&self, lod_ref: &LodRef) -> Vec<SceneChunk> {
		if self.plants.is_empty() {
			return Vec::new();
		}
		let n = self.plants.len();
		let plants = Arc::clone(&self.plants);
		let prev = *lod_ref.previous_transform;
		let curr = *lod_ref.current_transform;
		let bounds = *lod_ref.bounds;
		let entity = lod_ref.entity;
		let mut index = 0usize;
		vec![SceneChunk::lazy(n as u32, n, move || {
			if index >= plants.len() {
				return None;
			}
			let plant = &plants[index];
			index += 1;
			let plant_lod = LodRef {
				entity,
				previous_transform: &prev,
				current_transform: &curr,
				bounds: &bounds,
			};
			Some(match &plant.kind {
				PalmShadeKind::Waialea(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
				),
				PalmShadeKind::Date(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
				),
			})
		})]
	}

	fn canopy_sites(&self) -> Vec<CanopyProxySite> {
		self.plants
			.iter()
			.flat_map(|plant| match &plant.kind {
				PalmShadeKind::Waialea(t) => canopy_proxy_waialea(
					t,
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
				),
				PalmShadeKind::Date(t) => {
					canopy_proxy_crown(t, plant.placement, &plant.ball_material)
						.into_iter()
						.collect()
				}
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<PalmShadeCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> PalmShadePlant {
	let variant = patch_variant_index(placed.position, tree_variants);
	let palette_noise = placement_noise(grove_noise, placed.position);
	let stick_seed = palette_noise.seed;
	let canopy_seed = palette_noise.seed.wrapping_add(31);
	let stick_material =
		stick_material_from_palette(Some(placed.variant.stick_palette_mix()), stick_seed);
	let ball_material =
		canopy_ball_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed);
	let frond_material =
		frond_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed);

	let (kind, world_size) = match placed.variant {
		PalmShadeCell::TowerWaialeaPalm => {
			let (tree, world_size) = TowerWaialeaPalm::grow_num(variant);
			(PalmShadeKind::Waialea(tree), world_size)
		}
		PalmShadeCell::LowerWaialeaPalm => {
			let (tree, world_size) = LowerWaialeaPalm::grow_num(variant);
			(PalmShadeKind::Waialea(tree), world_size)
		}
		PalmShadeCell::ShadeDatePalm => {
			let (tree, world_size) = ShadeDatePalm::grow_num(variant);
			(PalmShadeKind::Date(tree), world_size)
		}
		PalmShadeCell::ClusterDatePalm => {
			let (tree, world_size) = ClusterDatePalm::grow_num(variant);
			(PalmShadeKind::Date(tree), world_size)
		}
	};

	PalmShadePlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		kind,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_visual_plant!(PalmShadePlant, PalmShadeKind => [Waialea, Date]);
crate::impl_woody_grove_lod!(PalmShade, WOODY_LOD);

#[cfg(test)]
mod tests;
