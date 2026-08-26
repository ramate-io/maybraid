use bevy::prelude::*;
use chico_sbs_trees::{PalmBush, PalmBushParams, QuantizedPlant};
use chico_vegetation_components::{
	FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
};
use clap::Args;
use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

use super::{
	definition, TropicalTuftsCell, TropicalTuftsItem, BRIGHT_TUFT, BRIGHT_TUFT_PATCH, DEEP_TUFT,
	DEEP_TUFT_PATCH, YELLOW_GREEN_TUFT, YELLOW_GREEN_TUFT_PATCH,
};
use crate::grove::vc_tuft::{
	grow_tuft_plants, material_from_palette, patch_variant_index, tuft_grove_stick_nodes,
	unit_plant_from_grown, TuftGroveBody, TuftGrovePlant, TuftGroveProxyHeights,
};
use crate::grove::{
	flatten_foliage_nodes, frond_material_from_palette, placement_noise, remixed_blade_tuft_plant,
	remixed_tuft_plant, FlatTerrainSample, GrovePreviewParams,
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
					let TropicalTuftsItem::PalmBush(palm) = placed.variant.item() else {
						unreachable!("palm cells only");
					};
					let noise = placement_noise(foliage_noise, placed.position);
					let geometry = palm.build_with_noise(noise);
					let mut params = PalmBushParams::default();
					params.geometry = geometry;
					let material = material_from_palette(mix, placed.position, foliage_noise);
					let frond_material = frond_material_from_palette(Some(mix), noise.seed);
					palms.push(TropicalTuftsPalm {
						placement: Placement::new(placed.position, 0.0)
							.with_scale(Vec3::splat(placed.scale.max(1e-4))),
						bush: params.build(),
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
				grow_tuft_plants(tuft_grown, self.merge_collections),
				&self.extent,
				self.cell_extent_xz(),
				TuftGroveProxyHeights::SHORT,
			),
			palms,
		}
	}
}

#[derive(Clone)]
struct TropicalTuftsPalm {
	placement: Placement,
	bush: PalmBush,
	material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct TropicalTufts {
	body: TuftGroveBody,
	palms: Vec<TropicalTuftsPalm>,
}

impl TropicalTufts {
	pub fn plants(&self) -> &[TuftGrovePlant] {
		&self.body.plants
	}
}

impl VegetationComponents for TropicalTufts {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		tuft_grove_stick_nodes(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let mut nodes = self.body.foliage_for_level(level).flatten();
		// Palm companions: High/Medium authored fronds; Low/UltraLow shared five-chord star.
		let palm_level = match level {
			LodSceneLevel::Medium => LodSceneLevel::High,
			other => other,
		};
		for palm in &self.palms {
			nodes.extend(flatten_foliage_nodes(
				&palm.bush,
				palm.placement,
				&palm.material,
				&palm.frond_material,
				palm_level,
			));
		}
		Layers::from_free(nodes)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(self.body.structural_lod())
	}
}

crate::impl_tuft_grove_lod!(TropicalTufts);
