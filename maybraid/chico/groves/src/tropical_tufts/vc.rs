use std::sync::Arc;

use bevy::prelude::*;
use chico_sbs_trees::{PalmBush, PalmBushParams, QuantizedPlant};
use chico_vegetation_components::{
	FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
};
use clap::Args;
use lod::gen::LodSceneLevel;
use lod::lod_ref::LodRef;
use lod::SceneChunk;
use material_ref::MaterialRef;
use procedural_common::{noise_params_from_scalar_str, NoiseParams};

use super::{
	definition, TropicalTuftsCell, BRIGHT_TUFT, BRIGHT_TUFT_PATCH, DEEP_TUFT, DEEP_TUFT_PATCH,
	JUVENILE_PALM_BUSH, SMALL_PALM_BUSH, YELLOW_GREEN_TUFT, YELLOW_GREEN_TUFT_PATCH,
};
use crate::grove::vc_tuft::{
	grow_tuft_plants, material_from_palette, patch_variant_index, tuft_grove_stick_nodes,
	unit_plant_from_grown, TuftGroveBody, TuftGrovePlant, TuftGroveProxyHeights,
};
use crate::grove::{
	frond_material_from_palette, nest_flattened_plant_chunk, placement_noise,
	remixed_blade_tuft_plant, remixed_sbs_plant, remixed_tuft_plant, FlatTerrainSample,
	GrovePreviewParams,
};

fn default_foliage() -> NoiseParams {
	NoiseParams::from_scalar(0.0, 1.0, 0.06, 1)
}

remixed_blade_tuft_plant!(BrightTuft, BRIGHT_TUFT, default_foliage());
remixed_blade_tuft_plant!(DeepTuft, DEEP_TUFT, default_foliage());
remixed_blade_tuft_plant!(YellowGreenTuft, YELLOW_GREEN_TUFT, default_foliage());
remixed_tuft_plant!(BrightTuftPatch, BRIGHT_TUFT_PATCH, default_foliage());
remixed_tuft_plant!(DeepTuftPatch, DEEP_TUFT_PATCH, default_foliage());
remixed_tuft_plant!(YellowGreenTuftPatch, YELLOW_GREEN_TUFT_PATCH, default_foliage());
remixed_sbs_plant!(SmallPalmBush, PalmBush, PalmBushParams, SMALL_PALM_BUSH);
remixed_sbs_plant!(JuvenilePalmBush, PalmBush, PalmBushParams, JUVENILE_PALM_BUSH);

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct TropicalTuftsParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<TropicalTuftsCell>,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Foliage Surface Noise",
	)]
	pub foliage_noise: NoiseParams,

	#[arg(long, default_value_t = 0)]
	pub merge_collections: usize,

	#[arg(long, default_value_t = 100)]
	pub patch_variants: u32,
}

impl Default for TropicalTuftsParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			merge_collections: 0,
			patch_variants: 100,
		}
	}
}

crate::impl_grove_preview_params!(TropicalTuftsParams, TropicalTuftsCell);

impl TropicalTuftsParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> TropicalTufts {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> TropicalTufts {
		let foliage_noise = self.foliage_noise;
		let variants = self.patch_variants.max(1);
		let mut tuft_grown = Vec::new();
		let mut palms = Vec::new();
		for placed in self.placements_on(world) {
			let mix = placed.variant.palette_mix();
			match placed.variant {
				TropicalTuftsCell::SmallPalmBush | TropicalTuftsCell::JuvenilePalmBush => {
					let variant = patch_variant_index(placed.position, variants);
					let (bush, world_size) = match placed.variant {
						TropicalTuftsCell::SmallPalmBush => SmallPalmBush::grow_num(variant),
						TropicalTuftsCell::JuvenilePalmBush => JuvenilePalmBush::grow_num(variant),
						_ => unreachable!("palm cells only"),
					};
					let material = material_from_palette(mix, placed.position, foliage_noise);
					let frond_material = frond_material_from_palette(
						Some(mix),
						placement_noise(foliage_noise, placed.position).seed,
					);
					palms.push(TropicalTuftsPalm {
						placement: Placement::new(placed.position, 0.0)
							.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
						bush,
						material,
						frond_material,
					});
				}
				cell => {
					let variant = patch_variant_index(placed.position, variants);
					let (patch, world_size) = match cell {
						TropicalTuftsCell::BrightTuft => BrightTuft::grow_num(variant),
						TropicalTuftsCell::DeepTuft => DeepTuft::grow_num(variant),
						TropicalTuftsCell::YellowGreenTuft => YellowGreenTuft::grow_num(variant),
						TropicalTuftsCell::BrightTuftPatch => BrightTuftPatch::grow_num(variant),
						TropicalTuftsCell::DeepTuftPatch => DeepTuftPatch::grow_num(variant),
						TropicalTuftsCell::YellowGreenTuftPatch => {
							YellowGreenTuftPatch::grow_num(variant)
						}
						TropicalTuftsCell::SmallPalmBush | TropicalTuftsCell::JuvenilePalmBush => {
							unreachable!("palm cells handled above")
						}
					};
					let material = material_from_palette(mix, placed.position, foliage_noise);
					tuft_grown.push(unit_plant_from_grown(
						patch,
						world_size,
						placed.position,
						placed.scale,
						material,
					));
				}
			}
		}
		TropicalTufts {
			body: TuftGroveBody::from_plants(
				grow_tuft_plants(tuft_grown, self.merge_collections, &self.extent),
				&self.extent,
				self.cell_extent_xz(),
				TuftGroveProxyHeights::SHORT,
			),
			palms: palms.into(),
		}
	}
}

#[derive(Clone)]
struct TropicalTuftsPalm {
	placement: Placement,
	bush: Arc<PalmBush>,
	material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct TropicalTufts {
	body: TuftGroveBody,
	palms: Arc<[TropicalTuftsPalm]>,
}

impl TropicalTufts {
	pub fn plants(&self) -> &[TuftGrovePlant] {
		&self.body.plants
	}

	pub fn palm_count(&self) -> usize {
		self.palms.len()
	}

	#[cfg(test)]
	pub(crate) fn palms_share_unit_arc(&self) -> bool {
		self.palms.len() >= 2 && Arc::ptr_eq(&self.palms[0].bush, &self.palms[1].bush)
	}

	/// High/Medium palm hosts — one lazy producer so begin does not rebuild crowns.
	fn nest_palm_chunks(&self, lod_ref: &LodRef) -> Vec<SceneChunk> {
		if self.palms.is_empty() {
			return Vec::new();
		}
		let n = self.palms.len();
		let palms = Arc::clone(&self.palms);
		let prev = *lod_ref.previous_transform;
		let curr = *lod_ref.current_transform;
		let bounds = *lod_ref.bounds;
		let entity = lod_ref.entity;
		let mut index = 0usize;
		vec![SceneChunk::lazy(n as u32, n, move || {
			if index >= palms.len() {
				return None;
			}
			let palm = &palms[index];
			index += 1;
			let plant_lod = LodRef {
				entity,
				previous_transform: &prev,
				current_transform: &curr,
				bounds: &bounds,
			};
			Some(nest_flattened_plant_chunk(
				Arc::clone(&palm.bush),
				palm.placement,
				&MaterialRef::default(),
				&palm.material,
				&palm.frond_material,
				&plant_lod,
			))
		})]
	}
}

impl VegetationComponents for TropicalTufts {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		tuft_grove_stick_nodes(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		self.body.foliage_for_level(level)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(self.body.structural_lod())
	}
}

impl lod::gen::LodScene for TropicalTufts {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> lod::gen::LodSceneLevel {
		self.structural_lod()
			.map(|band| crate::grove::grove_lod_level(band, lod_ref))
			.unwrap_or(LodSceneLevel::High)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		self.structural_lod()
			.map(|band| crate::grove::grove_lod_status(band, lod_ref))
			.unwrap_or(lod::gen::LodSceneStatus::Unchanged)
	}

	fn scene_lod_culls(
		&self,
		lod_ref: &LodRef,
		_current: LodSceneLevel,
	) -> lod::gen::LodSceneCulls {
		self.structural_lod()
			.map(|band| crate::grove::grove_lod_culls(band, lod_ref))
			.unwrap_or(lod::gen::LodSceneCulls::None)
	}

	fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		level: LodSceneLevel,
	) -> impl bevy::scene::prelude::Scene + 'static {
		chico_vegetation_components::flattened_component_scene(self, lod_ref, level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		let tufts =
			chico_vegetation_components::flattened_vegetation_scene_chunks(self, lod_ref, level);
		match level {
			LodSceneLevel::High | LodSceneLevel::Medium => {
				let palms = self.nest_palm_chunks(lod_ref);
				if palms.is_empty() {
					tufts
				} else {
					let mut parts = Vec::with_capacity(1 + palms.len());
					parts.push(tufts);
					parts.extend(palms);
					SceneChunk::chunks(parts)
				}
			}
			_ => tufts,
		}
	}

	fn scene_bounds(&self) -> bevy::math::bounding::Aabb3d {
		self.structural_lod()
			.map(|p| p.footprint_aabb())
			.unwrap_or_else(|| chico_vegetation_components::vegetation_bounds(self))
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl bevy::scene::prelude::Scene + 'static {
		lod::lod_host_scene_pending(self.scene_lod_level(lod_ref), self.scene_bounds())
	}
}
