//! [`RenderItem`] for populated Tall Grass groves ([#302](https://github.com/ramate-io/maybraid/issues/302)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::BladeTuft;
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::SkippedLeafMeshMaterial;
use chico_groves::tall_grass::{definition, TallGrassCell, TallGrassItem};
use chico_groves::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveCellVariant, GroveExtent,
	GroveFrontend, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};

/// Typical [`StandardMaterial`] Tall Grass instance.
pub type TallGrassStd =
	TallGrass<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>, FlatTerrainSample>;

/// Tall Grass grove preview (leaf material → mid-height blade tufts).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TallGrass<LeafM, LeafS, Terrain = FlatTerrainSample>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	#[command(flatten, next_help_heading = "Grove")]
	pub grove: GroveFrontend,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Foliage Surface Noise",
	)]
	pub foliage_noise: NoiseParams,

	#[arg(skip)]
	pub extent: GroveExtent,

	#[command(flatten, next_help_heading = "Terrain")]
	pub terrain: Terrain,

	#[arg(skip)]
	resolved_placements: Option<Vec<GroveCellVariant<TallGrassCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> LeafM>,
}

impl<LeafM, LeafS, Terrain> Default for TallGrass<LeafM, LeafS, Terrain>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	fn default() -> Self {
		Self {
			grove: GroveFrontend::default(),
			leaf_material: LeafS::default(),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
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

impl<LeafM, LeafS, Terrain> TallGrass<LeafM, LeafS, Terrain>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<TallGrassCell>>,
		terrain: Terrain,
		foliage_noise: NoiseParams,
		leaf_material: LeafS,
	) -> Self {
		Self {
			grove: GroveFrontend::default(),
			leaf_material,
			foliage_noise,
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

	pub fn placements(&self) -> Vec<GroveCellVariant<TallGrassCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
	}
}

fn placement_transform<V>(placed: &GroveCellVariant<V>) -> Transform {
	Transform {
		translation: placed.position,
		rotation: Quat::IDENTITY,
		scale: Vec3::splat(placed.scale.max(1e-4)),
	}
}

impl<LeafM, LeafS, Terrain> RenderItem for TallGrass<LeafM, LeafS, Terrain>
where
	LeafM: Material + WithPalette + Default + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
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
			let noise = placement_noise(self.foliage_noise, placed.position);
			let entities = match placed.variant.item() {
				TallGrassItem::Clump(clump) => {
					let mut shape = clump.build_with_noise(noise);
					shape.noise_amplitude = self.foliage_noise.amplitude;
					shape.noise_frequency = self.foliage_noise.frequency;
					let tuft = BladeTuft::from_shape(shape, self.leaf_material.clone());
					tuft.spawn_render_items(commands, cascade_chunk, local)
				}
				TallGrassItem::Patch(patch) => {
					let mut item = patch.build_tuft_patch(noise, self.leaf_material.clone());
					item.shape.noise_amplitude = self.foliage_noise.amplitude;
					item.shape.noise_frequency = self.foliage_noise.frequency;
					item.spawn_render_items(commands, cascade_chunk, local)
				}
			};
			patch_spawned_leaf_material::<LeafM>(
				&entities,
				placed.variant.palette_mix(),
				noise.seed,
				commands,
			);
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
	fn clump_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));
		for cell in [
			TallGrassCell::RiverGreen,
			TallGrassCell::PaleReed,
			TallGrassCell::TropicalBlade,
			TallGrassCell::HawaiianRed,
		] {
			let TallGrassItem::Clump(clump) = cell.item() else {
				anyhow::bail!("expected clump item for {cell:?}");
			};
			let shape = clump.build_with_noise(noise);
			assert!(shape.blade_length >= clump.height.start.min(clump.height.end));
			assert!(shape.blade_length <= clump.height.start.max(clump.height.end));
			let factor = shape.blade_width / shape.blade_length;
			assert!(factor >= clump.width_factor.start.min(clump.width_factor.end));
			assert!(factor <= clump.width_factor.start.max(clump.width_factor.end));
			assert!(clump.blade_count.contains(&shape.blade_count));
			assert!(clump.bend_segments.contains(&shape.bend_segments));
			assert!(shape.max_tilt_radians >= clump.max_tilt_radians.start);
			assert!(shape.max_tilt_radians <= clump.max_tilt_radians.end);
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			TallGrassCell::RiverGreen,
			TallGrassCell::PaleReed,
			TallGrassCell::TropicalBlade,
			TallGrassCell::HawaiianRed,
			TallGrassCell::RiverGreenPatch,
			TallGrassCell::PaleReedPatch,
			TallGrassCell::TropicalBladePatch,
			TallGrassCell::HawaiianRedPatch,
		] {
			let palette = cell.palette_mix();
			let mut allowed = Vec::new();
			for slot in palette.slots {
				allowed.extend(slot.start.resolve());
				allowed.extend(slot.end.resolve());
			}
			assert!(!allowed.is_empty(), "unresolved palette tokens for {cell:?}");
			let material = StandardMaterial::with_palette(StandardMaterial::default(), palette, 7);
			assert!(allowed.contains(&material.base_color));
		}
		Ok(())
	}

	#[test]
	fn patch_build_samples_layout_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));
		let TallGrassItem::Patch(patch) = TallGrassCell::RiverGreenPatch.item() else {
			anyhow::bail!("expected patch item");
		};
		let item = patch.build_tuft_patch::<StandardMaterial, _>(
			noise,
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert!(patch.clump_count.contains(&item.clump_count));
		assert!(item.patch_extent_xz >= patch.patch_extent_xz.start);
		assert!(item.patch_extent_xz <= patch.patch_extent_xz.end);
		assert_eq!(item.clump_anchors().len(), item.clump_count as usize);
		assert!(item.shape.base_spread >= patch.base_spread.start);
		assert!(item.shape.base_spread <= patch.base_spread.end);
		Ok(())
	}

	#[test]
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement =
			GroveCellVariant::new(TallGrassCell::RiverGreen, Vec3::new(1.0, 0.0, 2.0), 1.0);
		let item = TallGrassStd::with_resolved_placements(
			vec![placement.clone()],
			FlatTerrainSample::default(),
			NoiseParams::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements(), vec![placement]);
		Ok(())
	}

	#[test]
	fn default_weights_yield_dense_placements_in_preview_grid() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let grass = TallGrassStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.40, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grass.placement_cells().len();
		let placements = grass.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.55..=0.85).contains(&placed_share),
			"expected dense tall-grass fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		assert!(!placements.is_empty());
		Ok(())
	}
}
