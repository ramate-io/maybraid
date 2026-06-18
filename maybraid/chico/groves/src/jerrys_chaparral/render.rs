//! [`RenderItem`] for populated Jerry's Chaparral groves ([#318](https://github.com/ramate-io/maybraid/issues/318)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::anchors::high_bush::{
	DEFAULT_ANCHOR_LIFT_FRACTION, DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
	DEFAULT_SEGMENT_LENGTH_FRACTION_LO, DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
	DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
};
use chico_sbs_geometry::{FriendsConiferSbs, RorysHeadTrainedSbs};
use chico_sbs_trees::friends_conifer::FriendsConifer;
use chico_sbs_trees::rorys_head_trained::RorysHeadTrained;
use chico_tree_components::{HighBushFoliageStyle, HighBushShoots, HighBushShootsShape};
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{
	noise_params_from_scalar_str, BuildWithNoise, NoiseConfig, NoiseParams, UnitRange,
};
use render_item::{CascadeChunk, RenderItem};

use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GrovePlacedCell, TerrainSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::jerrys_chaparral::{
	definition, JerrysChaparralBush, JerrysChaparralCell, JerrysChaparralFriendsConifer,
	JerrysChaparralItem, JerrysChaparralRoryHead,
};
use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};

/// Uniform terrain tuned for chaparral placement constraints (RFC min elevation > 0).
#[derive(Debug, Clone, Copy, PartialEq, Args)]
#[command(next_help_heading = "Terrain")]
pub struct ChaparralFlatTerrain {
	#[arg(long, default_value_t = 0.35)]
	pub elevation: f32,
	#[arg(long, default_value_t = 0.15)]
	pub steepness: f32,
}

impl Default for ChaparralFlatTerrain {
	fn default() -> Self {
		Self { elevation: 0.35, steepness: 0.15 }
	}
}

impl TerrainSample for ChaparralFlatTerrain {
	fn elevation_at(&self, _position: Vec3) -> f32 {
		self.elevation
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		self.steepness
	}
}

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Jerry's Chaparral instance.
pub type JerrysChaparralStd = JerrysChaparral<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	ChaparralFlatTerrain,
>;

/// Jerry's Chaparral grove preview (Rory forms, high bushes, and small Friend's Conifer accents).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct JerrysChaparral<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	#[command(flatten, next_help_heading = "Grove")]
	pub grove: GroveFrontend,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	#[arg(
		long,
		default_value = "0,1.0,1.0,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "The noise applied to the chains of sticks in trees and bushes",
	)]
	pub tree_chain_noise: NoiseParams,

	#[arg(
		long,
		default_value = "0,1.0,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Stick Surface Noise",
	)]
	pub stick_surface_noise: NoiseParams,

	#[arg(
		long,
		default_value = "0,1.0,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Leaf Surface Noise",
	)]
	pub leaf_surface_noise: NoiseParams,

	#[arg(skip)]
	pub extent: GroveExtent,

	#[command(flatten, next_help_heading = "Terrain")]
	pub terrain: Terrain,

	#[arg(skip)]
	resolved_placements: Option<Vec<GrovePlacedCell<JerrysChaparralCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for JerrysChaparral<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	fn default() -> Self {
		Self {
			grove: GroveFrontend::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			tree_chain_noise: NoiseParams::from_scalar(0.0, 1.0, 1.0, 1),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			extent: GroveExtent::new(
				Vec3::ZERO,
				Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
			),
			terrain: Terrain::default(),
			resolved_placements: None,
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, LeafM, LeafS, Terrain> JerrysChaparral<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<JerrysChaparralCell>>,
		terrain: Terrain,
		tree_chain_noise: NoiseParams,
		stick_surface_noise: NoiseParams,
		leaf_surface_noise: NoiseParams,
		stick_material: StickS,
		leaf_material: LeafS,
	) -> Self {
		Self {
			grove: GroveFrontend::default(),
			stick_material,
			leaf_material,
			tree_chain_noise,
			stick_surface_noise,
			leaf_surface_noise,
			extent: GroveExtent::new(
				Vec3::ZERO,
				Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
			),
			terrain,
			resolved_placements: Some(resolved_placements),
			__marker: PhantomData,
		}
	}

	pub fn with_extent(mut self, extent: GroveExtent) -> Self {
		self.extent = extent;
		self
	}

	pub fn with_terrain(mut self, terrain: Terrain) -> Self {
		self.terrain = terrain;
		self
	}

	pub fn cell_extent_xz(&self) -> Vec2 {
		self.grove.definition(definition()).cell_extent_xz
	}

	pub fn placement_cells(&self) -> Vec<gimme_gen::Cell> {
		self.extent.subdivide_xz(self.cell_extent_xz())
	}

	pub fn placements(&self) -> Vec<GrovePlacedCell<JerrysChaparralCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
	}
}

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn sample_u32(config: &NoiseConfig, range: &std::ops::RangeInclusive<u32>, salt: f32) -> u32 {
	let lo = *range.start() as usize;
	let hi = (*range.end() as usize).saturating_add(1);
	config.sample_range_usize_4d(lo, hi, 0.0, 0.0, 0.0, salt) as u32
}

fn span_fraction(canopy_spread: f32, height: f32) -> f32 {
	(canopy_spread / height.max(0.5)).clamp(0.25, 1.20)
}

impl BuildWithNoise<RorysHeadTrainedSbs> for JerrysChaparralRoryHead {
	fn build_with_noise(&self, noise: NoiseParams) -> RorysHeadTrainedSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(0.75);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);

		let mut geometry = RorysHeadTrainedSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.canopy_noise = noise;
		let span = span_fraction(canopy_spread, height);
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.95, span * 1.15);
		geometry
	}
}

impl BuildWithNoise<HighBushShootsShape> for JerrysChaparralBush {
	fn build_with_noise(&self, noise: NoiseParams) -> HighBushShootsShape {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(0.5);
		let leaf_radius = sample_f32(&config, self.leaf_radius, 2.0).max(0.01);

		HighBushShootsShape {
			height,
			anchor_lift_fraction: DEFAULT_ANCHOR_LIFT_FRACTION,
			shoot_count: sample_u32(&config, &self.shoot_count, 3.0),
			radial_strength: sample_f32(&config, self.radial_strength, 5.0),
			vertical_bias: sample_f32(&config, self.vertical_bias, 6.0),
			branch_depth: sample_u32(&config, &self.branch_depth, 4.0) as usize,
			segment_length_fraction_lo: DEFAULT_SEGMENT_LENGTH_FRACTION_LO,
			segment_length_fraction_hi: DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
			segment_radius_fraction_lo: DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
			segment_radius_fraction_hi: DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
			leaf_radius_fraction: leaf_radius / height,
			foliage_style: HighBushFoliageStyle::PlaneSplay,
			chain_noise: noise,
		}
	}
}

struct ConiferSamples {
	geometry: FriendsConiferSbs,
	apex_canopy_spawn_fraction: f32,
	splay_radius_fraction_of_height: f32,
}

impl BuildWithNoise<ConiferSamples> for JerrysChaparralFriendsConifer {
	fn build_with_noise(&self, noise: NoiseParams) -> ConiferSamples {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(1.5);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 3.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = FriendsConiferSbs::default();
		geometry.scale.stalk_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.projection.length_fraction_of_height =
			UnitRange::new(span * 0.95, (span * 0.35).max(0.03));
		geometry.growth.branch_depth = 2;
		geometry.rings.anchors_per_ring = 3;
		geometry.canopy_noise = noise;

		ConiferSamples {
			geometry,
			apex_canopy_spawn_fraction: canopy_density.clamp(0.25, 0.55),
			splay_radius_fraction_of_height: (canopy_spread / height).clamp(0.014, 0.06),
		}
	}
}

fn placement_transform<V>(placed: &GrovePlacedCell<V>) -> Transform {
	Transform {
		translation: placed.position,
		rotation: Quat::IDENTITY,
		scale: Vec3::splat(placed.scale.max(1e-4)),
	}
}

impl<StickM, StickS, LeafM, LeafS, Terrain> RenderItem
	for JerrysChaparral<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material + WithPalette + Default + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	LeafM: Material + WithPalette + Default + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let mut out = Vec::new();
		for placed in self.placements() {
			let local = transform.mul_transform(placement_transform(&placed));
			let chain_noise = placement_noise(self.tree_chain_noise, placed.position);
			let build_noise = placement_noise(self.grove.noise, placed.position);
			let foliage_noise = placement_noise(self.leaf_surface_noise, placed.position);

			let entities = match placed.variant.item() {
				JerrysChaparralItem::RoryHead(rory) => {
					let geometry = rory.build_with_noise(build_noise);
					let mut tree = RorysHeadTrained::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = geometry;
					tree.stick_material = self.stick_material.clone();
					tree.leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.leaf_surface_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					let stick_seed = chain_noise.seed as i32;
					let canopy_seed = build_noise.seed as i32 + 31;
					patch_spawned_leaf_material::<StickM>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
				JerrysChaparralItem::Bush(bush) => {
					let mut shape = bush.build_with_noise(build_noise);
					shape.chain_noise = chain_noise;
					let entities = HighBushShoots::<StickM, StickS, LeafM, LeafS>::spawn_from_shape(
						shape,
						self.stick_surface_noise,
						foliage_noise,
						self.stick_material.clone(),
						self.leaf_material.clone(),
						commands,
						cascade_chunk,
						local,
					);
					let stick_seed = chain_noise.seed as i32;
					let canopy_seed = build_noise.seed as i32 + 31;
					patch_spawned_leaf_material::<StickM>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
				JerrysChaparralItem::FriendsConifer(conifer) => {
					let samples = conifer.build_with_noise(build_noise);
					let mut tree = FriendsConifer::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = samples.geometry;
					tree.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
					tree.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
					tree.stick_material = self.stick_material.clone();
					tree.leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.leaf_surface_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					let stick_seed = chain_noise.seed as i32;
					let canopy_seed = build_noise.seed as i32 + 31;
					patch_spawned_leaf_material::<StickM>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
			};
			out.extend(entities);
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn rory_bush_and_conifer_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));

		for cell in [JerrysChaparralCell::DryRoryHeadTrained, JerrysChaparralCell::ManzanitaRory] {
			let JerrysChaparralItem::RoryHead(rory) = cell.item() else {
				anyhow::bail!("expected rory item for {cell:?}");
			};
			let geom = rory.build_with_noise(noise);
			assert!(geom.scale.tree_height >= rory.height.start.min(rory.height.end));
			assert!(geom.scale.tree_height <= rory.height.start.max(rory.height.end));
		}

		let JerrysChaparralItem::Bush(bush) = JerrysChaparralCell::ChaparralHighBush.item() else {
			anyhow::bail!("expected bush item");
		};
		let shape = bush.build_with_noise(noise);
		assert!(shape.height >= bush.height.start.min(bush.height.end));
		assert!(shape.height <= bush.height.start.max(bush.height.end));
		assert!(bush.shoot_count.contains(&shape.shoot_count));
		assert!(bush.branch_depth.contains(&(shape.branch_depth as u32)));

		let JerrysChaparralItem::FriendsConifer(conifer) =
			JerrysChaparralCell::SmallFriendsConifer.item()
		else {
			anyhow::bail!("expected conifer item");
		};
		let samples = conifer.build_with_noise(noise);
		assert!(
			samples.geometry.scale.stalk_height >= conifer.height.start.min(conifer.height.end)
		);
		assert!(
			samples.geometry.scale.stalk_height <= conifer.height.start.max(conifer.height.end)
		);
		assert!((0.25..=0.55).contains(&samples.apex_canopy_spawn_fraction));
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			JerrysChaparralCell::DryRoryHeadTrained,
			JerrysChaparralCell::ChaparralHighBush,
			JerrysChaparralCell::SmallFriendsConifer,
			JerrysChaparralCell::ManzanitaRory,
		] {
			for (palette, label) in
				[(cell.stick_palette_mix(), "stick"), (cell.canopy_palette_mix(), "canopy")]
			{
				let mut allowed = Vec::new();
				for slot in palette.slots {
					allowed.extend(slot.start.resolve());
					allowed.extend(slot.end.resolve());
				}
				assert!(!allowed.is_empty(), "unresolved {label} tokens for {cell:?}");
			}
		}
		Ok(())
	}

	#[test]
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement = GrovePlacedCell::new(
			JerrysChaparralCell::ChaparralHighBush,
			Vec3::new(1.0, 0.0, 2.0),
			1.0,
		);
		let item = JerrysChaparralStd::with_resolved_placements(
			vec![placement.clone()],
			ChaparralFlatTerrain::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			SkippedStickMeshMaterial::<ChicoStickMaterial>::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements(), vec![placement]);
		Ok(())
	}

	#[test]
	fn default_weights_yield_moderate_placements_in_preview_grid() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let grove = JerrysChaparralStd::default()
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.24..=0.52).contains(&placed_share),
			"expected chaparral fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn default_extent_includes_bush_rory_and_conifer_placements() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ * 2.0;
		let grove = JerrysChaparralStd::default()
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let placements = grove.placements();
		assert!(!placements.is_empty());
		assert!(
			placements
				.iter()
				.any(|p| { matches!(p.variant, JerrysChaparralCell::ChaparralHighBush) }),
			"expected at least one chaparral high bush"
		);
		assert!(
			placements.iter().any(|p| {
				matches!(
					p.variant,
					JerrysChaparralCell::DryRoryHeadTrained | JerrysChaparralCell::ManzanitaRory
				)
			}),
			"expected at least one rory form"
		);
		assert!(
			placements
				.iter()
				.any(|p| { matches!(p.variant, JerrysChaparralCell::SmallFriendsConifer) }),
			"expected at least one small friends conifer over enlarged extent"
		);
		Ok(())
	}
}
