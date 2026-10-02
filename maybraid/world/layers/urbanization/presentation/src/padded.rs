//! Padded-cell presentation for `Urbanization<M>`.

use std::collections::HashSet;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use durham::{
	PresentedTerrainScene, TerrainColliderMeshSource, TerrainSuperseded, TerrainTrimeshCollider,
};
use lod::gen::{Id, SpatialIndex};
use lod::lod_ref::LodRef;
use lod::{LodPresentGate, LodPresentSystems, LodViewer};
use richmond::{
	DevelopmentEntryStore, PaddedStoreView, PaddedTerrainPresenter, PresentedPaddedTerrainScene,
	TerrainWithPads,
};
use layer_stack::LodPresentGateSync;
use terrain_layer_model::{
	terrain_streaming, TerrainExtent, TerrainLayerSystems, TerrainModel,
};
use terrain_layer_presentation::TerrainPresenter;
use urbanization_layer_model::{
	urbanization_visual_region, Urbanization, UrbanizationGenerationSystems, UrbanizationLayerRegion,
};

/// Tick key for the padded presenter's `Local` (same fields as generate).
#[derive(Clone, Copy, Debug, PartialEq)]
struct PaddedTerrainTickKey {
	region: Aabb3d,
	store_rev: u64,
	terrain_rev: u64,
	viewer: Option<(i32, i32)>,
}

const PADDED_VIEWER_QUANT_XZ: f32 = 8.0;

fn quantize_viewer_xz(translation: Vec3) -> (i32, i32) {
	(
		(translation.x / PADDED_VIEWER_QUANT_XZ).floor() as i32,
		(translation.z / PADDED_VIEWER_QUANT_XZ).floor() as i32,
	)
}


/// Padded terrain ids replacing raw Durham presentation roots this frame.
#[derive(Resource, Default)]
pub struct UrbanizationPaddedTerrainState {
	pub(crate) wanted: HashSet<Id>,
	/// Raw ids this stream superseded. Only these are handed back, so raw
	/// cells another owner hid stay hidden.
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
		app.init_resource::<UrbanizationPaddedTerrainState>()
			.init_resource::<LodPresentGate<(Urbanization<M>, PaddedCells)>>()
			.add_systems(
			Update,
			(present_urbanization_padded_terrain::<M>, sync_raw_terrain_replacements)
				.chain()
				.after(UrbanizationGenerationSystems)
				.after(crate::UrbanizationHostPresent)
				.after(LodPresentGateSync)
				.run_if(terrain_streaming::<M>)
				.before(LodPresentSystems::Produce)
				.before(TerrainLayerSystems::<M::Base>::QueueColliders),
		);
	}
}

/// Present padded replacements for the urbanization keep and cull stale cells.
/// An inactive subscription culls every padded cell.
#[allow(clippy::too_many_arguments, private_interfaces)]
pub fn present_urbanization_padded_terrain<M>(
	gate: Res<LodPresentGate<(Urbanization<M>, PaddedCells)>>,
	layer: Res<UrbanizationLayerRegion>,
	extent: Res<TerrainExtent<M::Base>>,
	store: Res<DevelopmentEntryStore>,
	mut presenter: PaddedTerrainPresenter,
	mut state: ResMut<UrbanizationPaddedTerrainState>,
	lod_viewers: Query<&GlobalTransform, With<LodViewer>>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
	mut last: Local<Option<PaddedTerrainTickKey>>,
) where
	M: TerrainModel,
	Urbanization<M>: TerrainModel,
{
	let Some(region) = urbanization_visual_region(&*extent, layer.region).filter(|_| gate.open)
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
	let mut now_replaced = HashSet::new();
	for (entity, presented, mut visibility, superseded) in &mut raw_roots {
		if ready.contains(&presented.0) {
			let ours = state.replaced.contains(&presented.0);
			if ours || *visibility != Visibility::Hidden {
				if *visibility != Visibility::Hidden {
					*visibility = Visibility::Hidden;
				}
				if !superseded {
					commands.entity(entity).insert(TerrainSuperseded);
				}
				now_replaced.insert(presented.0);
			}
		} else if state.replaced.contains(&presented.0) {
			*visibility = Visibility::Inherited;
			if superseded {
				commands.entity(entity).remove::<TerrainSuperseded>();
			}
		}
	}
	state.replaced = now_replaced;
}

#[cfg(test)]
pub(crate) fn quantize_viewer_xz_for_test(translation: Vec3) -> (i32, i32) {
	quantize_viewer_xz(translation)
}
