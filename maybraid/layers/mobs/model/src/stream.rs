//! Mob generate / present-keep bullseyes and the camera stream.

use bevy::ecs::system::{ParamSet, SystemParam};
use bevy::prelude::*;
use chico_forests::ForestIndex;
use lod::gen::{LodGenerateKeepRegion, LodGenerateRegion};
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion};
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};
use lod::lod_ref::LodRef;
use mob_groups::MobPlantHost;
use richmond_development_models::DiscoverablePlace;
use terrain_layer_model::TerrainView;
use urbanization_layer_model::{UrbanModel, UrbanSetting};

use crate::index::{urban_leaf_arrival_radius, xz_radius_aabb, MobCellExtent, MobIndex};

/// Present / generate rings the world stream used (1 km / 3 km).
pub const MOB_GENERATE_RADIUS: f32 = 3_000.0;
pub const MOB_PRESENT_RADIUS: f32 = 1_000.0;

/// Training owns its roster, so the world stream steps aside.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MobStreamSuspended(pub bool);

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct MobGenerateBullseye {
	pub radius_m: f32,
	pub enabled: bool,
}

impl Default for MobGenerateBullseye {
	fn default() -> Self {
		Self { radius_m: MOB_GENERATE_RADIUS, enabled: true }
	}
}

impl LodRefreshRegions for MobGenerateBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		refresh_status(self.enabled, self.radius_m, lod_ref)
	}
}

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct MobPresentBullseye {
	pub radius_m: f32,
	pub enabled: bool,
}

impl Default for MobPresentBullseye {
	fn default() -> Self {
		Self { radius_m: MOB_PRESENT_RADIUS, enabled: true }
	}
}

impl LodRefreshRegions for MobPresentBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		refresh_status(self.enabled, self.radius_m, lod_ref)
	}
}

fn refresh_status(enabled: bool, radius: f32, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
	if !enabled {
		return LodRefreshRegionsStatus::Unchanged;
	}
	let previous = MobCellExtent::cell_index_containing(lod_ref.previous_transform.translation);
	let current = MobCellExtent::cell_index_containing(lod_ref.current_transform.translation);
	if previous == current {
		LodRefreshRegionsStatus::Unchanged
	} else {
		LodRefreshRegionsStatus::Changed(xz_radius_aabb(
			lod_ref.current_transform.translation,
			radius,
		))
	}
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MobLodChan;

#[derive(SystemParam)]
pub struct MobStream<'w> {
	pub generate: ResMut<'w, MobGenerateBullseye>,
	pub present: ResMut<'w, MobPresentBullseye>,
	generate_regions: MessageWriter<'w, LodGenerateRegion<MobLodChan>>,
	present_regions: MessageWriter<'w, LodPresentRegion<MobLodChan>>,
	pub generate_keep: ResMut<'w, LodGenerateKeepRegion<MobLodChan>>,
	pub present_keep: ResMut<'w, LodPresentKeepRegion<MobLodChan>>,
}

pub fn sync_mob_models<G: UrbanModel>(
	forest: Res<ForestIndex>,
	view: TerrainView<G>,
	mut mobs: ResMut<MobIndex>,
) {
	let (urbanization_noise, urbanization_kind) = G::urbanization_selection(&view.read);
	if !mobs.models_match(&forest, urbanization_noise, urbanization_kind) {
		mobs.configure_from(&forest, urbanization_noise, urbanization_kind);
	}
}

/// Collect plant hosts from urbanization leaves, development cells, then
/// presentation-spawned [`UrbanSetting`] / [`DiscoverablePlace`] entities.
///
/// Generation reads entities that presentation spawns.
pub fn sync_mob_plant_hosts<G: UrbanModel>(
	generate_keep: Res<LodGenerateKeepRegion<MobLodChan>>,
	mut access: ParamSet<(G::Select, TerrainView<G>)>,
	settings: Query<(&UrbanSetting, &GlobalTransform)>,
	places: Query<(&DiscoverablePlace, &GlobalTransform)>,
	mut mobs: ResMut<MobIndex>,
) {
	if !mobs.models_ready {
		return;
	}
	let region = generate_keep.region.unwrap_or(xz_radius_aabb(Vec3::ZERO, MOB_GENERATE_RADIUS));
	G::ensure_selected(&mut access.p0(), region);
	let view = access.p1();
	let mut hosts = Vec::new();
	for leaf in G::urbanization_leaves(&view.read, region) {
		hosts.push(MobPlantHost {
			xz: Vec2::new(
				(leaf.bounds.min.x + leaf.bounds.max.x) * 0.5,
				(leaf.bounds.min.z + leaf.bounds.max.z) * 0.5,
			),
			arrival_radius: urban_leaf_arrival_radius(leaf.bounds),
		});
	}
	for cell in G::development_cells(&view.read, region) {
		hosts.push(MobPlantHost {
			xz: Vec2::new(
				(cell.cell.min.x + cell.cell.max.x) * 0.5,
				(cell.cell.min.z + cell.cell.max.z) * 0.5,
			),
			arrival_radius: urban_leaf_arrival_radius(cell.cell),
		});
	}
	for (setting, transform) in &settings {
		hosts.push(MobPlantHost {
			xz: transform.translation().xz(),
			arrival_radius: setting.arrival_radius,
		});
	}
	for (place, transform) in &places {
		hosts.push(MobPlantHost {
			xz: transform.translation().xz(),
			arrival_radius: place.arrival_radius,
		});
	}
	mobs.plant_hosts = hosts;
}

/// Training owns its roster, so the world stream steps aside.
///
/// When suspended, generation keeps today's generation half: disable both
/// bullseyes, set `present_keep.region = None`, reset `previous_cell`, and
/// return. Presenter clears live in presentation.
pub fn stream_mobs(
	camera: Query<&Transform, With<Camera3d>>,
	suspended: Res<MobStreamSuspended>,
	mut stream: MobStream,
	mut previous_cell: Local<Option<(i32, i32)>>,
) {
	if suspended.0 {
		stream.generate.enabled = false;
		stream.present.enabled = false;
		stream.present_keep.region = None;
		*previous_cell = None;
		return;
	}
	let Ok(camera) = camera.single() else {
		return;
	};
	stream.generate.enabled = true;
	stream.present.enabled = true;
	let generate = xz_radius_aabb(camera.translation, MOB_GENERATE_RADIUS);
	let present = xz_radius_aabb(camera.translation, MOB_PRESENT_RADIUS);
	stream.generate_keep.region = Some(generate);
	stream.present_keep.region = Some(present);
	let current = MobCellExtent::cell_index_containing(camera.translation);
	if previous_cell.as_ref() == Some(&current) {
		return;
	}
	stream.generate_regions.write(LodGenerateRegion::new(generate));
	stream.present_regions.write(LodPresentRegion::new(present));
	*previous_cell = Some(current);
}
