//! Canopy bump-outs from forest selection. Does not grow grove tiles.

use std::collections::{HashMap, HashSet};

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_bumpout::{BumpOut, BumpOutNeighborhood, BumpOutStyle};
use chico_forests::{
	blend_selection_neighborhood, BumpOutSelectionSample, ForestIndex, BUMP_OUT_PRESENT_RADIUS_M,
};
use durham_terrain_models::{
	cascade_chunk_for_cell, ComposedTerrain, Terrain, TerrainCellLayout, TerrainEntryStore,
	TerrainStoreView, TERRAIN_CELL_SIZE,
};
use lod::gen::{Id, SpatialIndex, TrackedId};
use lod_cascade::Chunk;
use procedural_common::NoiseParams;
use render_item::sdf::cpu_shot::CpuShotBuilder;
use terrain_chunk_ref::TerrainChunkRef;

use crate::{PlaygroundConfig, TerrainPresentPending, TerrainPresentationDirty};

pub(crate) type WorldTerrainBuilder = CpuShotBuilder<ComposedTerrain>;

#[derive(Component, Debug, Clone, Copy)]
struct PresentedCanopyBumpOut;

#[derive(Resource, Default)]
pub(crate) struct CanopyBumpOutState {
	presented: HashMap<Id, Entity>,
	spec_key: Option<String>,
}

pub(crate) fn present_canopy_bump_outs(
	mut commands: Commands,
	config: Res<PlaygroundConfig>,
	forest: Res<ForestIndex>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	terrain_dirty: Res<TerrainPresentationDirty>,
	pending: Res<TerrainPresentPending>,
	camera: Query<&Transform, With<Camera3d>>,
	mut state: ResMut<CanopyBumpOutState>,
) {
	if terrain_dirty.0 || pending.0 {
		return;
	}

	let spec_key = config.forest.map(|spec| spec.key());
	if spec_key != state.spec_key {
		clear_presented(&mut commands, &mut state);
		state.spec_key = spec_key.clone();
	}

	let Some(_spec) = config.forest else {
		clear_presented(&mut commands, &mut state);
		return;
	};

	let Ok(camera) = camera.single() else {
		return;
	};
	let region = xz_radius_aabb(camera.translation, BUMP_OUT_PRESENT_RADIUS_M);
	let view = TerrainStoreView::new(&store, &layout);
	let wanted: HashSet<Id> = view
		.tracked_ids_for(region)
		.into_iter()
		.filter_map(|TrackedId(id)| {
			let terrain = view.get(id)?;
			let bounds = view.get_bounds(id)?;
			let size = (bounds.max.x - bounds.min.x).max(1e-3);
			if size > TERRAIN_CELL_SIZE * 1.5 {
				return None;
			}
			let samples = blend_selection_neighborhood(&forest, bounds);
			if samples.iter().all(|sample| sample.density <= 0.001) {
				return None;
			}
			let _ = terrain;
			Some(id)
		})
		.collect();

	let stale: Vec<Id> =
		state.presented.keys().copied().filter(|id| !wanted.contains(id)).collect();
	for id in stale {
		if let Some(entity) = state.presented.remove(&id) {
			commands.entity(entity).despawn();
		}
	}

	for id in wanted {
		if state.presented.contains_key(&id) {
			continue;
		}
		let Some(terrain) = view.get(id) else {
			continue;
		};
		let Some(bounds) = view.get_bounds(id) else {
			continue;
		};
		let samples = blend_selection_neighborhood(&forest, bounds);
		let Some(bump_out) = bump_out_from_samples(samples, bump_out_noise(&forest.noise)) else {
			continue;
		};
		let terrain_ref = terrain_chunk_ref(terrain);
		let entity = bump_out.spawn(&mut commands, terrain_ref);
		commands.entity(entity).insert(PresentedCanopyBumpOut);
		state.presented.insert(id, entity);
	}
}

fn clear_presented(commands: &mut Commands, state: &mut CanopyBumpOutState) {
	for entity in state.presented.values() {
		commands.entity(*entity).despawn();
	}
	state.presented.clear();
}

fn bump_out_from_samples(
	samples: [BumpOutSelectionSample; 9],
	noise: NoiseParams,
) -> Option<BumpOut> {
	let neighborhood = BumpOutNeighborhood::new(
		samples.map(|sample| sample.density),
		samples.map(|sample| sample.bite_size),
		samples.map(|sample| sample.bite_size_deviation),
		samples.map(|sample| sample.height_m),
		samples.map(|sample| sample.height_deviation_m),
	);
	if neighborhood.densities.iter().all(|density| *density <= 0.001) {
		return None;
	}
	Some(
		BumpOut::from_neighborhood(neighborhood, samples[4].palette, noise).with_style(
			BumpOutStyle::new(0.065, 0.88, 0.18)
				.with_cheese(0.88, 1.0)
				.with_fragment_height(4.5, 0.85),
		),
	)
}

fn bump_out_noise(forest: &NoiseParams) -> NoiseParams {
	NoiseParams {
		seed: forest.seed.wrapping_add(307),
		frequency: 0.045,
		amplitude: forest.amplitude,
		octaves: 3,
		..*forest
	}
}

fn terrain_chunk_ref(terrain: &Terrain) -> TerrainChunkRef<WorldTerrainBuilder> {
	let cascade = cascade_chunk_for_cell(terrain.cell, terrain.res_2);
	let extent = cascade.extent.unwrap_or(Vec3::splat(cascade.size));
	let chunk = Chunk::from_min_max(cascade.origin, cascade.origin + extent, None);
	TerrainChunkRef::new(
		CpuShotBuilder::new(terrain.sdf.clone()).with_wall_faces(terrain.wall_faces),
		chunk,
		terrain.res_2,
	)
}

fn xz_radius_aabb(center: Vec3, radius: f32) -> Aabb3d {
	let r = radius.max(0.0);
	Aabb3d::from_min_max(
		Vec3::new(center.x - r, -1_000_000.0, center.z - r),
		Vec3::new(center.x + r, 1_000_000.0, center.z + r),
	)
}
