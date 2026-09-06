//! Forest presentation against Durham terrain after Richmond pad modulation.

use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_forests::{ChicoGrove, ForestIndex};
use chico_groves::{GroveHeightModulation, ModulatedGroveSample};
use chico_sbs_trees_playground::forest_stream::ForestPresenterState;
use chico_vegetation_on_terrain_playground::{
	DurhamGroveSample, StoredDurhamTerrain, WorldBaseTerrain,
};
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, Version};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use richmond_development_models::{DevelopmentCell, DevelopmentIndex, PadComplex};

struct DevelopmentPadModulation<'a>(&'a PadComplex);

impl GroveHeightModulation for DevelopmentPadModulation<'_> {
	fn modulate_height(&self, base_height: f32, x: f32, z: f32) -> f32 {
		self.0.modify_elevation(base_height, x, z)
	}
}

/// Present forest groves against Durham terrain with all overlapping
/// development pads blended in one pass.
#[derive(SystemParam)]
pub struct DevelopmentForestPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, ForestPresenterState>,
	development: DevelopmentIndex<'w>,
	base: Res<'w, WorldBaseTerrain>,
}

impl RegionPresenter<ChicoGrove, ForestIndex> for DevelopmentForestPresenter<'_, '_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented_version(id)
	}

	fn handle(&mut self, id: Id, version: Version, grove: &ChicoGrove, lod_ref: &LodRef) {
		let bounds = grove.aabb();
		let ids = <DevelopmentCell as GenerationScheme<DevelopmentIndex<'_>>>::original_ids_for(
			&mut self.development,
			bounds,
		);
		for OriginalId(development_id) in ids {
			let _ = GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate(
				&mut self.development,
				development_id,
				lod_ref,
			);
		}

		let pads = self.development.store.merged_pad_complex(bounds);
		let terrain = StoredDurhamTerrain::new(
			self.development.terrain_store(),
			self.development.layout(),
			&self.base.0,
		);
		let raw = DurhamGroveSample::from_terrain(terrain);
		let modulation = DevelopmentPadModulation(&pads);
		let world = ModulatedGroveSample::new(raw, &modulation);
		self.state
			.present_with_world(&mut self.commands, id, version, grove, lod_ref, &world);
	}

	fn hide(&mut self, id: Id) {
		self.state.hide(&mut self.commands, id);
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.state.is_hidden(id)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented_ids()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		self.state.remove_stale(&mut self.commands, wanted);
	}

	fn cull(
		&mut self,
		spatial_index: &ForestIndex,
		keep: &HashSet<Id>,
		despawn_budget: u32,
	) -> u32 {
		self.state.cull(&mut self.commands, spatial_index, keep, despawn_budget)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy::math::Vec2;
	use chico_groves::{FlatTerrainSample, GroveWorldSample};
	use richmond_development_models::PadParams;

	#[test]
	fn pad_modulation_sets_exact_terrace_and_preserves_base_outside() -> Result<()> {
		let pad = PadComplex::building_skirt(
			Vec2::ZERO,
			Vec2::splat(10.0),
			0.0,
			12.0,
			PadParams::default(),
		);
		let base = FlatTerrainSample { elevation: 3.0, steepness: 0.0 };
		let modulation = DevelopmentPadModulation(&pad);
		let sample = ModulatedGroveSample::new(base, &modulation);

		assert!((sample.height_at(Vec3::ZERO) - 12.0).abs() < 1e-5);
		assert!((sample.height_at(Vec3::new(1_000.0, 0.0, 1_000.0)) - 3.0).abs() < 1e-5);
		Ok(())
	}
}
