//! Per-frame visual-cell traversal: cull, pick LOD, list cached batches.

use bevy::log::info_span;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::{LodSceneLevel, VisualLodPrimitive};
use vegetation_components::StructuralLod;
use vegetation_groves::{
	ORCHARD_STRUCTURAL_HIGH_FACTOR, ORCHARD_STRUCTURAL_LOW_FACTOR, ORCHARD_STRUCTURAL_MEDIUM_FACTOR,
};

use crate::packed::cache::{PackedGroveCache, PackedGroveCell};
use crate::packed::instances::PackedBatch;
use crate::packed::mode::PackMode;
use crate::ChicoGroveHost;

/// One visible tile's selected batches for this view.
#[derive(Clone, Debug)]
pub struct SelectedTile {
	pub id: lod::gen::Id,
	pub level: LodSceneLevel,
	pub batches: Vec<PackedBatch>,
	pub upload: bool,
}

/// Frame draw list. Rebuilt every tick; tile instance buffers are not.
#[derive(Resource, Default, Debug)]
pub struct PackedGroveSelection {
	pub tiles: Vec<SelectedTile>,
	pub frame: u32,
	pub draws: usize,
	pub instances: usize,
}

/// Orchard structural band from viewer distance (same factors as the host).
pub fn select_band(center: Vec3, radius: f32, viewer: Vec3) -> LodSceneLevel {
	StructuralLod::new(center, radius)
		.with_factors(
			ORCHARD_STRUCTURAL_HIGH_FACTOR,
			ORCHARD_STRUCTURAL_MEDIUM_FACTOR,
			ORCHARD_STRUCTURAL_LOW_FACTOR,
		)
		.with_preserve_ultra_low(true)
		.level_for(&Transform::from_translation(viewer))
}

pub fn select_packed_tiles(
	cameras: Query<(&GlobalTransform, Option<&Projection>), With<Camera3d>>,
	mut cache: ResMut<PackedGroveCache>,
	mut selection: ResMut<PackedGroveSelection>,
	frame: Option<Res<bevy::diagnostic::FrameCount>>,
) {
	if !PackMode::current().packs_orchard() {
		selection.tiles.clear();
		return;
	}
	let _span = info_span!("packed_grove_select").entered();
	let frame_n = frame.map(|count| count.0).unwrap_or(0);
	selection.frame = frame_n;
	selection.tiles.clear();
	selection.draws = 0;
	selection.instances = 0;

	let Some((camera, projection)) = cameras.iter().next() else {
		return;
	};
	let camera_pos = camera.translation();
	let forward = camera.forward();
	let far = match projection {
		Some(Projection::Perspective(p)) => p.far,
		Some(Projection::Orthographic(p)) => p.far,
		_ => 4000.0,
	};

	let ids: Vec<_> = cache.tiles().map(|tile| tile.id).collect();
	for id in ids {
		let Some(tile) = cache.get(id) else {
			continue;
		};
		if !tile_visible(tile.bounds, camera_pos, forward, far) {
			continue;
		}
		let level = select_band(tile.center, tile.radius, camera_pos);
		let batches = tile.batches(level).to_vec();
		if batches.is_empty() {
			continue;
		}
		let upload = tile.gpu_dirty;
		let instances = batches.iter().map(|batch| batch.instances.len()).sum::<usize>();
		selection.draws += batches.len();
		selection.instances += instances;
		selection.tiles.push(SelectedTile { id, level, batches, upload });
		cache.touch(id, frame_n);
		if upload {
			cache.mark_uploaded(id);
		}
	}
}

pub fn evict_packed_tiles(mut cache: ResMut<PackedGroveCache>) {
	if PackMode::current().packs_orchard() {
		cache.evict_to_budget();
	}
}

pub fn sync_packed_tiles(
	hosts: Query<(&PackedGroveCell, &ChicoGroveHost)>,
	mut cache: ResMut<PackedGroveCache>,
	frame: Option<Res<bevy::diagnostic::FrameCount>>,
) {
	if !PackMode::current().packs_orchard() {
		return;
	}
	let frame_n = frame.map(|count| count.0).unwrap_or(0);
	let live: std::collections::HashSet<_> = hosts.iter().map(|(cell, _)| cell.id).collect();
	for (cell, host) in &hosts {
		let Some(orchard) = host.tile.as_orchard() else {
			continue;
		};
		cache.ensure(cell.id, cell.version, orchard, host.layer, frame_n);
	}
	let stale: Vec<_> = cache.tiles().map(|tile| tile.id).filter(|id| !live.contains(id)).collect();
	for id in stale {
		cache.remove(id);
	}
}

fn tile_visible(bounds: Aabb3d, camera: Vec3, forward: Dir3, far: f32) -> bool {
	let center = (Vec3::from(bounds.min) + Vec3::from(bounds.max)) * 0.5;
	let radius = (Vec3::from(bounds.max) - Vec3::from(bounds.min)).length() * 0.5;
	if camera.distance(center) - radius > far {
		return false;
	}
	(center - camera).dot(*forward) + radius > 0.0
}

/// Visual primitives for the current selection (tests / metrics).
pub fn selected_primitives(selection: &PackedGroveSelection) -> Vec<VisualLodPrimitive> {
	selection
		.tiles
		.iter()
		.flat_map(|tile| tile.batches.iter().map(|batch| batch.visual_primitive(tile.level)))
		.collect()
}
