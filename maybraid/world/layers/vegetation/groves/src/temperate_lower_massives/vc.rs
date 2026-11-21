use super::WOODY_LOD;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, OnceLock};

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
#[cfg(test)]
use bevy::shape::Aabb3d;
use clap::Args;
use lod::lod_ref::LodRef;
#[cfg(test)]
use lod::scene::LodScene;
use lod::scene::LodSceneLevel;
use lod::SceneChunk;
use material_ref::MaterialRef;
use procedural_common::{BuildWithNoise, NoiseParams};
use sbs_trees::{
	BraidOakTree, QuantizedPlant, RorysHeadTrained, RorysHeadTrainedParams, StorybookTree,
	StorybookTreeParams,
};
use vegetation_components::{
	scene_children, structural_shadow_proxy_scene, FlattenedComponentsOnly, PlacedVegetation,
	Placement, StickNode, VegetationComponents, FLATTENED_KIT_CHUNK_WEIGHT,
};

use crate::grove::vc_compose::flattened_plant_metadata;

use super::{
	definition, StorybookFieldArea, TemperateLowerMassivesCell, TemperateLowerMassivesItem,
	LOWER_MASSIVE_STORYBOOK, RARE_LOWER_MASSIVE_RORY,
};
use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_rory, canopy_proxy_site,
	frond_material_from_palette, grove_structural_footprint, nest_flattened_plant_chunk,
	placement_noise, remixed_sbs_plant, stick_material_from_palette, CanopyProxySite,
	FlatTerrainSample, FlattenedPlantList, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct TemperateLowerMassivesParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<TemperateLowerMassivesCell>,
}

impl Default for TemperateLowerMassivesParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 }),
		}
	}
}

crate::impl_grove_preview_params!(TemperateLowerMassivesParams, TemperateLowerMassivesCell);

impl TemperateLowerMassivesParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> TemperateLowerMassives {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> TemperateLowerMassives {
		TemperateLowerMassives::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_sbs_plant!(
	LowerMassiveStorybook,
	StorybookTree,
	StorybookTreeParams,
	LOWER_MASSIVE_STORYBOOK
);
remixed_sbs_plant!(
	RareLowerMassiveRory,
	RorysHeadTrained,
	RorysHeadTrainedParams,
	RARE_LOWER_MASSIVE_RORY
);

enum StorybookFieldTrial {
	Tile { xz: Vec2, area: StorybookFieldArea, replace_kits: bool },
	Scout,
}

static STORYBOOK_FIELD_TRIAL: OnceLock<StorybookFieldTrial> = OnceLock::new();

/// Select a fixed grove area before world generation admits replacement.
pub fn configure_storybook_field_area(
	xz: Vec2,
	area: StorybookFieldArea,
	replace_kits: bool,
) -> bool {
	xz.is_finite()
		&& STORYBOOK_FIELD_TRIAL
			.set(StorybookFieldTrial::Tile { xz, area, replace_kits })
			.is_ok()
}

/// Report one eligible High LOD tile while retaining the normal grove kits.
pub fn scout_storybook_field_tile() {
	let _ = STORYBOOK_FIELD_TRIAL.set(StorybookFieldTrial::Scout);
}

/// Exact inputs from the existing world generator; the rendering adapter owns its recipe.
#[derive(Component, Clone, Copy, Default)]
pub struct StorybookFieldSample {
	/// Canonical tile X/Z origin bits and the stable index in its grown plant list.
	pub slot: [u32; 3],
	pub root: Vec3,
	pub up: Vec3,
	pub height: f32,
	pub seed: u32,
}

#[derive(Clone)]
enum TemperateLowerMassivesKind {
	Oak(Arc<BraidOakTree>),
	Storybook(Arc<StorybookTree>),
	Rory(Arc<RorysHeadTrained>),
}

#[derive(Clone)]
pub struct TemperateLowerMassivesPlant {
	pub placement: Placement,
	kind: TemperateLowerMassivesKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct TemperateLowerMassives {
	pub plants: Arc<[TemperateLowerMassivesPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl TemperateLowerMassives {
	pub fn from_placements(
		placements: &[GroveCellVariant<TemperateLowerMassivesCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[TemperateLowerMassivesPlant]> = placements
			.iter()
			.map(|placed| grow_plant(placed, grove_noise, tree_variants))
			.collect::<Vec<_>>()
			.into();
		let (structural_center, footprint_radius) = grove_structural_footprint(extent);
		Self { plants, structural_center, footprint_radius, extent: *extent }
	}

	fn uses_tree_field(&self, level: LodSceneLevel) -> bool {
		let Some(trial) = STORYBOOK_FIELD_TRIAL.get() else { return false };
		let (anchor, area, replace_kits) = match *trial {
			StorybookFieldTrial::Scout => {
				self.report_scout_tile(level);
				return false;
			}
			StorybookFieldTrial::Tile { xz, area, replace_kits } => (xz, area, replace_kits),
		};
		if !area.contains_tile(anchor, self.extent) {
			return false;
		}
		let mut placements = Vec::new();
		for plant in self.plants.iter() {
			if let TemperateLowerMassivesKind::Storybook(tree) = &plant.kind {
				let sample = storybook_field_sample(tree, plant, [0; 3]);
				placements.push((
					sample.root.to_array().map(f32::to_bits),
					sample.up.to_array().map(f32::to_bits),
					sample.height.to_bits(),
					sample.seed,
				));
			}
		}
		let eligible = (1..=area.plants_per_tile() as usize).contains(&placements.len());
		let center = (self.extent.min() + self.extent.max()) * 0.5;
		let tile = Vec2::new(center.x, center.z);
		placements.sort_unstable();
		let mut digest = DefaultHasher::new();
		placements.hash(&mut digest);
		info!(
			target: "maybraid.tree_field",
			?level, ?tile, ?area, plants = placements.len(), woody_plants = self.plants.len(), placement_digest = digest.finish(),
			eligible, field = eligible && replace_kits, "natural Storybook tile inventory"
		);
		eligible && replace_kits
	}

	fn report_scout_tile(&self, level: LodSceneLevel) {
		static REPORTED: OnceLock<()> = OnceLock::new();
		if level != LodSceneLevel::High || REPORTED.get().is_some() {
			return;
		}
		let storybooks = self
			.plants
			.iter()
			.filter(|plant| matches!(&plant.kind, TemperateLowerMassivesKind::Storybook(_)))
			.count();
		if (4..=StorybookFieldArea::Tile.plants_per_tile() as usize).contains(&storybooks) {
			REPORTED.get_or_init(|| {
				let min = self.extent.min();
				let max = self.extent.max();
				let center = (min + max) * 0.5;
				info!(
					target: "maybraid.tree_field",
					min_x = min.x, min_z = min.z, max_x = max.x, max_z = max.z,
					center_x = center.x, center_z = center.z, storybooks, woody_plants = self.plants.len(),
					"natural Storybook scout tile"
				);
			});
		}
	}

	fn nest_plant_chunks(&self, lod_ref: &LodRef, level: LodSceneLevel) -> Vec<SceneChunk> {
		let use_tree_field = self.uses_tree_field(level);
		let min = self.extent.min();
		let mut slot = [min.x.to_bits(), min.z.to_bits(), 0];
		FlattenedPlantList::capture(&self.plants, lod_ref).nest_chunks(move |plant, plant_lod| {
			let plant_slot = slot;
			slot[2] += 1;
			match &plant.kind {
				TemperateLowerMassivesKind::Oak(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					plant_lod,
					level,
				),
				TemperateLowerMassivesKind::Storybook(t) if use_tree_field => SceneChunk::weighted(
					FLATTENED_KIT_CHUNK_WEIGHT,
					storybook_field_host(t, plant, plant_slot),
				),
				TemperateLowerMassivesKind::Storybook(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					plant_lod,
					level,
				),
				TemperateLowerMassivesKind::Rory(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					plant_lod,
					level,
				),
			}
		})
	}

	fn canopy_sites(&self) -> Vec<CanopyProxySite> {
		self.plants
			.iter()
			.flat_map(|plant| {
				let material = &plant.ball_material;
				match &plant.kind {
					TemperateLowerMassivesKind::Oak(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					TemperateLowerMassivesKind::Storybook(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					TemperateLowerMassivesKind::Rory(t) => vec![
						canopy_proxy_rory(t, plant.placement, &plant.stick_material, material)
							.crown,
					],
				}
			})
			.collect()
	}

	fn proxy_trunks(&self) -> Vec<StickNode> {
		self.plants
			.iter()
			.filter_map(|plant| match &plant.kind {
				TemperateLowerMassivesKind::Rory(t) => {
					canopy_proxy_rory(
						t,
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
					)
					.trunk
				}
				_ => None,
			})
			.collect()
	}
}

fn storybook_field_host(
	tree: &Arc<StorybookTree>,
	plant: &TemperateLowerMassivesPlant,
	slot: [u32; 3],
) -> impl Scene + 'static {
	let host = FlattenedComponentsOnly(PlacedVegetation::new(
		Arc::clone(tree),
		plant.placement,
		plant.stick_material.clone(),
		plant.ball_material.clone(),
		plant.frond_material.clone(),
	));
	let canopy = structural_shadow_proxy_scene(&host);
	let sample = storybook_field_sample(tree, plant, slot);
	(
		flattened_plant_metadata(host),
		bsn! { ~{sample} },
		scene_children(canopy.into_iter().collect()),
	)
}

fn storybook_field_sample(
	tree: &StorybookTree,
	plant: &TemperateLowerMassivesPlant,
	slot: [u32; 3],
) -> StorybookFieldSample {
	StorybookFieldSample {
		slot,
		root: plant.placement.translation,
		up: plant.placement.rotation() * Vec3::Y,
		height: tree.geometry.height() * plant.placement.scale.y,
		seed: tree.geometry.canopy_noise.seed as u32,
	}
}

fn grow_plant(
	placed: &GroveCellVariant<TemperateLowerMassivesCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> TemperateLowerMassivesPlant {
	let variant = patch_variant_index(placed.position, tree_variants);
	let build_noise = variant_noise(grove_noise, variant);
	let palette_noise = placement_noise(grove_noise, placed.position);
	let stick_seed = palette_noise.seed;
	let canopy_seed = palette_noise.seed.wrapping_add(31);
	let stick_material =
		stick_material_from_palette(Some(placed.variant.stick_palette_mix()), stick_seed);
	let ball_material =
		canopy_ball_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed);
	let frond_material =
		frond_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed);

	match placed.variant.item() {
		TemperateLowerMassivesItem::BraidOak(oak) => {
			let world_size = oak.build_with_noise(build_noise).height();
			TemperateLowerMassivesPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: TemperateLowerMassivesKind::Oak(BraidOakTree::grow_num(variant).0),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		TemperateLowerMassivesItem::Storybook(_) => {
			let (tree, world_size) = LowerMassiveStorybook::grow_num(variant);
			TemperateLowerMassivesPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: TemperateLowerMassivesKind::Storybook(tree),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		TemperateLowerMassivesItem::Rory(_) => {
			let (tree, world_size) = RareLowerMassiveRory::grow_num(variant);
			TemperateLowerMassivesPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: TemperateLowerMassivesKind::Rory(tree),
				stick_material,
				ball_material,
				frond_material,
			}
		}
	}
}

crate::impl_woody_grove_lod!(TemperateLowerMassives, WOODY_LOD, trunks);

#[cfg(test)]
mod tests;
