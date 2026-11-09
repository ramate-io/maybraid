//! Per-view CPU cell culling and LOD selection; draw lists contain no instances.

use crate::packed::gpu::PackedBatchInstance;
use bevy::camera::primitives::{Frustum, Sphere};
use bevy::pbr::ShadowView;
use bevy::prelude::*;
use bevy::render::view::{ExtractedView, RetainedViewEntity};
use lod::LodSceneLevel;
use std::collections::HashMap;
use vegetation_components::StructuralLod;

#[derive(Resource, Default)]
pub struct PackedViewSelection {
	pub levels: HashMap<(RetainedViewEntity, Entity), LodSceneLevel>,
	pub visible_cells: usize,
	pub culled_cells: usize,
}

pub fn select_band(band: StructuralLod, viewer: Vec3) -> LodSceneLevel {
	band.level_for(&Transform::from_translation(viewer))
}

pub fn select_packed_views(
	views: Query<(&ExtractedView, Option<&Frustum>, Has<ShadowView>)>,
	instances: Query<&PackedBatchInstance>,
	mut selection: ResMut<PackedViewSelection>,
	mut cells: Local<HashMap<Entity, std::sync::Arc<crate::packed::cache::TilePayload>>>,
) {
	let _span =
		bevy::log::tracing::trace_span!(target: "maybraid.packed.profile", "packed_cull_select")
			.entered();
	cells.clear();
	for instance in &instances {
		cells
			.entry(instance.cell)
			.or_insert_with(|| std::sync::Arc::clone(&instance.payload));
	}
	selection.levels.clear();
	selection.visible_cells = 0;
	selection.culled_cells = 0;
	for (view, frustum, shadow) in &views {
		for (&cell, payload) in cells.iter() {
			let center = (payload.bounds.min + payload.bounds.max) * 0.5;
			let sphere = Sphere { center, radius: (payload.bounds.max - center).length() };
			if frustum.is_some_and(|frustum| !frustum.intersects_sphere(&sphere, true)) {
				selection.culled_cells += 1;
				continue;
			}
			let viewer = if shadow {
				let associated = view.retained_view_entity.auxiliary_entity;
				views
					.iter()
					.filter(|(_, _, is_shadow)| !*is_shadow)
					.filter(|(camera, _, _)| {
						associated
							.is_none_or(|main| camera.retained_view_entity.main_entity == main)
					})
					.min_by(|(a, _, _), (b, _, _)| {
						a.world_from_view.translation().distance_squared(payload.center).total_cmp(
							&b.world_from_view.translation().distance_squared(payload.center),
						)
					})
					.map(|(camera, _, _)| camera.world_from_view.translation())
					.unwrap_or_else(|| view.world_from_view.translation())
			} else {
				view.world_from_view.translation()
			};
			selection.levels.insert(
				(view.retained_view_entity, cell),
				select_band(payload.structural_lod, viewer),
			);
			selection.visible_cells += 1;
		}
	}
}
