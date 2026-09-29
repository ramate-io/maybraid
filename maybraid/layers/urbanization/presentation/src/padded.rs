//! Padded-cell presentation for `Urbanization<M>`.

use std::collections::HashSet;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use durham_terrain_models::{
	terrain_streaming_enabled, PresentedTerrainScene, TerrainCellLayout, TerrainColliderMeshSource,
	TerrainColliderSystems, TerrainSuperseded, TerrainTrimeshCollider,
};
use lod::gen::{Id, SpatialIndex};
use lod::lod_ref::LodRef;
use lod::presentation::LodPresentKeepRegion;
use lod::{LodPresentSystems, LodViewer};
use richmond_development_models::{
	DevelopmentEntryStore, PaddedStoreView, PaddedTerrainPresenter, PresentedPaddedTerrainScene,
	TerrainWithPads,
};
use richmond_urbanization::UrbanizationLodChan;
use terrain_layer_model::TerrainModel;
use terrain_layer_presentation::TerrainPresenter;
use urbanization_layer_model::{
	Urbanization, UrbanizationGenerationSystems, UrbanizationLayerConfig,
	UrbanizationStreamingEnabled,
};

/// Tick key for the padded presenter's `Local` (same fields as generate).
#[derive(Clone, Copy, Debug, PartialEq)]
struct PaddedTerrainTickKey {
	region: Aabb3d,
	store_rev: u64,
	terrain_rev: u64,
	urban: bool,
	viewer: Option<(i32, i32)>,
}

const PADDED_VIEWER_QUANT_XZ: f32 = 8.0;

fn quantize_viewer_xz(translation: Vec3) -> (i32, i32) {
	(
		(translation.x / PADDED_VIEWER_QUANT_XZ).floor() as i32,
		(translation.z / PADDED_VIEWER_QUANT_XZ).floor() as i32,
	)
}

fn pad_visual_region(layout: &TerrainCellLayout, urban_keep: Option<Aabb3d>) -> Option<Aabb3d> {
	if layout.is_streamed() {
		Some(layout.presentation_region())
	} else {
		urban_keep
	}
}

/// Padded terrain ids replacing raw Durham presentation roots this frame.
#[derive(Resource, Default)]
pub struct UrbanizationPaddedTerrainState {
	pub(crate) wanted: HashSet<Id>,
	/// Raw ids this stream superseded. Only these are handed back, so raw
	/// cells another owner hid (Training's courtyard) stay hidden.
	pub(crate) replaced: HashSet<Id>,
}

/// Presents padded replacements for `Urbanization<M>`.
pub struct PaddedCells;

impl<M> TerrainPresenter<Urbanization<M>> for PaddedCells
where
	M: TerrainModel,
	Urbanization<M>: TerrainModel,
{
	fn install(app: &mut App) {
		app.init_resource::<UrbanizationPaddedTerrainState>().add_systems(
			Update,
			(present_urbanization_padded_terrain, sync_raw_terrain_replacements)
				.chain()
				.after(UrbanizationGenerationSystems)
				.after(crate::UrbanizationHostPresent)
				.run_if(terrain_streaming_enabled)
				.before(LodPresentSystems::Produce)
				.before(TerrainColliderSystems::QueueMeshes),
		);
	}
}

/// Present padded replacements for the urbanization keep and cull stale cells.
/// With urbanization off every padded cell is culled.
#[allow(clippy::too_many_arguments, private_interfaces)]
pub fn present_urbanization_padded_terrain(
	config: Res<UrbanizationLayerConfig>,
	enabled: Res<UrbanizationStreamingEnabled>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	layout: Res<TerrainCellLayout>,
	store: Res<DevelopmentEntryStore>,
	mut presenter: PaddedTerrainPresenter,
	mut state: ResMut<UrbanizationPaddedTerrainState>,
	lod_viewers: Query<&GlobalTransform, With<LodViewer>>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
	mut last: Local<Option<PaddedTerrainTickKey>>,
) {
	let Some(region) = pad_visual_region(&layout, keep.region)
		.filter(|_| config.urbanization.is_some() && enabled.0)
	else {
		if last.is_some() || !state.wanted.is_empty() {
			state.wanted.clear();
			presenter.remove_stale(&state.wanted);
		}
		*last = None;
		return;
	};
	let viewer = lod_viewers
		.iter()
		.next()
		.or_else(|| cameras.iter().next())
		.map(|tf| {
			let t = tf.translation();
			Transform::from_translation(Vec3::new(t.x, 0.0, t.z))
		})
		.unwrap_or(Transform::IDENTITY);
	let key = PaddedTerrainTickKey {
		region,
		store_rev: store.membership_revision(),
		terrain_rev: presenter.terrain_membership_revision(),
		urban: true,
		viewer: Some(quantize_viewer_xz(viewer.translation)),
	};
	if last.as_ref() == Some(&key) {
		return;
	}
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &viewer,
		current_transform: &viewer,
		bounds: &region,
	};
	let view = PaddedStoreView::new(&store);
	let tracked: HashSet<Id> = SpatialIndex::<TerrainWithPads>::tracked_ids_for(&view, region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	presenter.present_tracked(&view, &tracked, &lod_ref);
	state.wanted = tracked;
	*last = Some(key);
}

/// Hand a raw Durham cell to its padded replacement once that fill can bear
/// weight: hide the raw mesh and supersede its collider. Until then the raw
/// cell stays the floor, so there is never a frame with no collider.
pub fn sync_raw_terrain_replacements(
	mut commands: Commands,
	mut state: ResMut<UrbanizationPaddedTerrainState>,
	padded: Query<(
		&PresentedPaddedTerrainScene,
		Has<TerrainTrimeshCollider>,
		Has<TerrainColliderMeshSource>,
	)>,
	mut raw_roots: Query<(Entity, &PresentedTerrainScene, &mut Visibility, Has<TerrainSuperseded>)>,
) {
	let ready: HashSet<Id> = padded
		.iter()
		.filter(|(scene, cooked, collides)| {
			state.wanted.contains(&scene.0) && (*cooked || !*collides)
		})
		.map(|(scene, _, _)| scene.0)
		.collect();
	for (entity, presented, mut visibility, superseded) in &mut raw_roots {
		if ready.contains(&presented.0) {
			if *visibility != Visibility::Hidden {
				*visibility = Visibility::Hidden;
			}
			if !superseded {
				commands.entity(entity).insert(TerrainSuperseded);
			}
		} else if state.replaced.contains(&presented.0) {
			*visibility = Visibility::Inherited;
			if superseded {
				commands.entity(entity).remove::<TerrainSuperseded>();
			}
		}
	}
	state.replaced = ready;
}

#[cfg(test)]
pub(crate) fn quantize_viewer_xz_for_test(translation: Vec3) -> (i32, i32) {
	quantize_viewer_xz(translation)
}
