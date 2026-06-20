//! [`RenderItem`] for populated Wild Grass groves ([#304](https://github.com/ramate-io/maybraid/issues/304)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::{BladeTuft, BladeTuftShape};
use clap::Args;
use procedural_common::{
	noise_params_from_scalar_str, BuildWithNoise, NoiseConfig, NoiseParams, UnitRange,
};
use render_item::{CascadeChunk, RenderItem};

use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GroveCellVariant, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::skipped_mesh_material::SkippedLeafMeshMaterial;
use crate::wild_grass::{definition, WildGrassCell, WildGrassClump, WildGrassItem};

/// Typical [`StandardMaterial`] Wild Grass instance.
pub type WildGrassStd =
	WildGrass<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>, FlatTerrainSample>;

/// Wild Grass grove preview (leaf material → tall colorful blade tufts).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct WildGrass<LeafM, LeafS, Terrain = FlatTerrainSample>
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
		default_value = "0,1,0.12,1",
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
	resolved_placements: Option<Vec<GroveCellVariant<WildGrassCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> LeafM>,
}

impl<LeafM, LeafS, Terrain> Default for WildGrass<LeafM, LeafS, Terrain>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	fn default() -> Self {
		Self {
			grove: GroveFrontend::default(),
			leaf_material: LeafS::default(),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.12, 1),
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

impl<LeafM, LeafS, Terrain> WildGrass<LeafM, LeafS, Terrain>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	/// Render precomputed placements instead of selecting live from the grove frontend.
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<WildGrassCell>>,
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

	/// Effective vegetation cell footprint (frontend override or authored).
	pub fn cell_extent_xz(&self) -> Vec2 {
		self.grove.definition(definition()).cell_extent_xz
	}

	pub fn placement_cells(&self) -> Vec<gimme_gen::Cell> {
		self.extent.subdivide_xz(self.cell_extent_xz())
	}

	pub fn placements(&self) -> Vec<GroveCellVariant<WildGrassCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
	}
}

/// Sample a clump's authored geometry ranges into a blade tuft shape.
///
/// Blade width is **length-proportional** (`length * width_factor`), so short and tall
/// varietals stay equally grass-thin.
impl BuildWithNoise<BladeTuftShape> for WildGrassClump {
	fn build_with_noise(&self, noise: NoiseParams) -> BladeTuftShape {
		let config = NoiseConfig::new(noise);
		let sample_f32 = |range: UnitRange, salt| {
			let lo = range.start.min(range.end);
			let hi = range.start.max(range.end);
			config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
		};

		let sample_u32 = |range: &std::ops::RangeInclusive<u32>, salt| {
			let lo = *range.start() as usize;
			let hi = (*range.end() as usize).saturating_add(1);
			config.sample_range_usize_4d(lo, hi, 0.0, 0.0, 0.0, salt) as u32
		};

		let blade_length = sample_f32(self.height, 1.0).max(0.05);
		let blade_width = blade_length * sample_f32(self.width_factor, 2.0);

		BladeTuftShape {
			blade_count: sample_u32(&self.blade_count, 3.0),
			blade_length,
			blade_width,
			max_tilt_radians: sample_f32(self.max_tilt_radians, 4.0).max(0.01),
			bend_segments: sample_u32(&self.bend_segments, 5.0).max(1),
			seed: noise.seed,
			..BladeTuftShape::default()
		}
	}
}

fn placement_transform<V>(placed: &GroveCellVariant<V>) -> Transform {
	Transform {
		translation: placed.position,
		rotation: Quat::IDENTITY,
		scale: Vec3::splat(placed.scale.max(1e-4)),
	}
}

impl<LeafM, LeafS, Terrain> RenderItem for WildGrass<LeafM, LeafS, Terrain>
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
				WildGrassItem::Clump(clump) => {
					let mut shape = clump.build_with_noise(noise);
					shape.noise_amplitude = self.foliage_noise.amplitude;
					shape.noise_frequency = self.foliage_noise.frequency;
					let tuft = BladeTuft::from_shape(shape, self.leaf_material.clone());
					tuft.spawn_render_items(commands, cascade_chunk, local)
				}
				WildGrassItem::Patch(patch) => {
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
			WildGrassCell::MeadowGreen,
			WildGrassCell::GoldenGrass,
			WildGrassCell::RedPrairie,
			WildGrassCell::BlueTropical,
			WildGrassCell::PaleField,
			WildGrassCell::BloomingGrass,
		] {
			let WildGrassItem::Clump(clump) = cell.item() else {
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
	fn patch_build_samples_layout_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));
		let WildGrassItem::Patch(patch) = WildGrassCell::MeadowGreenPatch.item() else {
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
	fn palette_resolves_to_authored_color() -> Result<()> {
		for cell in [
			WildGrassCell::MeadowGreen,
			WildGrassCell::GoldenGrass,
			WildGrassCell::RedPrairie,
			WildGrassCell::BlueTropical,
			WildGrassCell::PaleField,
			WildGrassCell::BloomingGrass,
			WildGrassCell::MeadowGreenPatch,
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
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement =
			GroveCellVariant::new(WildGrassCell::MeadowGreen, Vec3::new(1.0, 0.0, 2.0), 1.0);
		let item = WildGrassStd::with_resolved_placements(
			vec![placement.clone()],
			FlatTerrainSample::default(),
			NoiseParams::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements(), vec![placement]);
		Ok(())
	}

	#[test]
	fn default_weights_yield_placements_in_preview_grid() -> Result<()> {
		let mut grass = WildGrassStd {
			terrain: FlatTerrainSample { elevation: 0.35, steepness: 0.15 },
			..Default::default()
		};
		let span = 6.0 * grass.cell_extent_xz();
		grass = grass.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span.x, 1.0, span.y)));
		assert!(!grass.placements().is_empty());
		Ok(())
	}
}
