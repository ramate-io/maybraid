//! Training Ground is a seeded FinePatch of the Maybraid world, not Discovery's
//! moving rings.
//!
//! While [`TrainingGrounds`] is set, Durham presents a four-cell origin window,
//! hopscotch stays off, the forest shrinks to one grove tile, and a Training
//! pose is not written. [`crate::training_plaza`] stamps one seeded Richmond
//! development on that patch once the surface exists.

use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::PlaygroundConfig;
use durham_terrain_models::{
	TerrainCellLayout, TerrainCoverage, TerrainLayoutPinned, TerrainPresentEnabled,
	TerrainPresentPending, TerrainPresentationAssets, TerrainPresentationDirty,
	TerrainPresenterState, WORLD_FINE_HALF_EXTENT_CELLS, playable_world_cell_layout,
	retarget_presentation_assets, training_grounds_cell_layout,
};
use richmond_developments_on_terrain_playground::UrbanizationStreamingEnabled;

/// Set by the game shell while Training Ground is the live session.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingGrounds(pub bool);

/// Forest stream radius used by [`PlaygroundConfig::world_defaults`].
const WORLD_FOREST_STREAM_RADIUS: u32 = 1;
/// Half-extent of [`training_grounds_cell_layout`].
const TRAINING_FINE_HALF_EXTENT_CELLS: i32 = 2;

/// What the world fill should be for a Training / Discovery session.
#[derive(Clone, Debug, PartialEq)]
pub struct TrainingFill {
	pub layout: TerrainCellLayout,
	pub coverage: TerrainCoverage,
	pub terrain_radius: i32,
	pub present: bool,
	pub pin_layout: bool,
	pub urbanization: bool,
	pub forest_stream_radius: u32,
}

impl TrainingFill {
	pub fn for_grounds(enabled: bool) -> Self {
		if enabled {
			Self {
				layout: training_grounds_cell_layout(),
				coverage: TerrainCoverage::FinePatch,
				terrain_radius: TRAINING_FINE_HALF_EXTENT_CELLS,
				present: true,
				pin_layout: true,
				urbanization: false,
				forest_stream_radius: 0,
			}
		} else {
			Self {
				layout: playable_world_cell_layout(),
				coverage: TerrainCoverage::PlayableWorld,
				terrain_radius: WORLD_FINE_HALF_EXTENT_CELLS,
				present: false,
				pin_layout: false,
				urbanization: true,
				forest_stream_radius: WORLD_FOREST_STREAM_RADIUS,
			}
		}
	}
}

/// Shrink Durham and the forest to the training patch, and keep hopscotch off.
/// Restores the playable rings when the shell clears [`TrainingGrounds`].
///
/// Runs in [`Update`] before generate so [`OnEnter`] shell look (after
/// [`PreUpdate`]) retargets the fill on the same frame.
pub(crate) fn apply_training_grounds(
	grounds: Res<TrainingGrounds>,
	mut active: Local<bool>,
	mut layout: ResMut<TerrainCellLayout>,
	mut coverage: ResMut<TerrainCoverage>,
	mut present: ResMut<TerrainPresentEnabled>,
	mut pinned: ResMut<TerrainLayoutPinned>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut assets: ResMut<TerrainPresentationAssets>,
	mut forest: Option<ResMut<PlaygroundConfig>>,
	mut urban: Option<ResMut<UrbanizationStreamingEnabled>>,
) {
	if *active == grounds.0 {
		return;
	}
	*active = grounds.0;
	let fill = TrainingFill::for_grounds(grounds.0);
	*layout = fill.layout.clone();
	*coverage = fill.coverage;
	present.0 = fill.present;
	pinned.0 = fill.pin_layout;
	dirty.0 = true;
	pending.0 = true;
	retarget_presentation_assets(&mut assets, fill.coverage, fill.terrain_radius);
	if let Some(urban) = urban.as_deref_mut() {
		urban.0 = fill.urbanization;
	}
	if let Some(config) = forest.as_deref_mut() {
		if let Some(spec) = config.forest.as_mut() {
			spec.stream_radius = fill.forest_stream_radius;
		}
		config.coverage = fill.coverage;
		config.terrain_radius = fill.terrain_radius;
	}
}

/// Drop raw training terrain meshes once present is turned back off.
pub(crate) fn clear_training_terrain_present(
	present: Res<TerrainPresentEnabled>,
	mut was_present: Local<bool>,
	mut commands: Commands,
	mut state: ResMut<TerrainPresenterState>,
) {
	if *was_present && !present.0 {
		state.clear(&mut commands);
	}
	*was_present = present.0;
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use durham_terrain_models::TerrainConfig;

	use super::*;

	#[test]
	fn apply_training_grounds_retargets_the_layout() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(true));
		world.insert_resource(playable_world_cell_layout());
		world.insert_resource(TerrainCoverage::PlayableWorld);
		world.insert_resource(TerrainPresentEnabled(false));
		world.insert_resource(TerrainLayoutPinned(false));
		world.insert_resource(TerrainPresentationDirty(false));
		world.insert_resource(TerrainPresentPending(false));
		world.insert_resource(TerrainPresentationAssets {
			config: TerrainConfig::new(42),
			material: Handle::default(),
			lod_bands: Vec::new(),
			outer_add_walls: true,
			fine_grid_max_radius: Some(WORLD_FINE_HALF_EXTENT_CELLS),
			macro_seam_half_extents: Vec::new(),
			macro_cell_min_size: None,
			macro_res_2: None,
		});
		world
			.run_system_once(apply_training_grounds)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<TerrainCellLayout>(), training_grounds_cell_layout());
		assert_eq!(*world.resource::<TerrainCoverage>(), TerrainCoverage::FinePatch);
		assert!(world.resource::<TerrainPresentEnabled>().0);
		assert!(world.resource::<TerrainLayoutPinned>().0);
		assert!(world.resource::<TerrainPresentationDirty>().0);
		Ok(())
	}

	#[test]
	fn training_fill_is_a_pinned_raw_fine_patch() {
		let fill = TrainingFill::for_grounds(true);
		assert_eq!(fill.layout, training_grounds_cell_layout());
		assert_eq!(fill.coverage, TerrainCoverage::FinePatch);
		assert!(fill.present);
		assert!(fill.pin_layout);
		assert!(!fill.urbanization);
		assert_eq!(fill.forest_stream_radius, 0);
	}

	#[test]
	fn discovery_fill_restores_the_playable_rings() {
		let fill = TrainingFill::for_grounds(false);
		assert_eq!(fill.layout, playable_world_cell_layout());
		assert_eq!(fill.coverage, TerrainCoverage::PlayableWorld);
		assert!(!fill.present);
		assert!(!fill.pin_layout);
		assert!(fill.urbanization);
		assert_eq!(fill.forest_stream_radius, WORLD_FOREST_STREAM_RADIUS);
	}
}
