//! Mob generate / present-keep bullseyes and the camera stream.

use bevy::ecs::system::ParamSet;
use bevy::prelude::*;
use chico_forests::ForestIndex;
use lod::gen::{LodGenerateKeepRegion, LodGenerateRegion};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion};
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};
use mob_groups::MobPlantHost;
use richmond_development_models::DiscoverablePlace;
use terrain_layer_model::TerrainView;
use urbanization_layer_model::{UrbanModel, UrbanSetting};

use crate::index::{urban_leaf_arrival_radius, xz_radius_aabb, MobCellExtent, MobIndex};

/// Present / generate rings the world stream used (1 km / 3 km).
pub const MOB_GENERATE_RADIUS: f32 = 3_000.0;
pub const MOB_PRESENT_RADIUS: f32 = 1_000.0;

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct MobGenerateBullseye {
	pub(crate) radius_m: f32,
	pub(crate) enabled: bool,
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
	pub(crate) radius_m: f32,
	pub(crate) enabled: bool,
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

/// Present keep follows the viewer in every mode so a written cell inside
/// the present radius presents.
pub fn stream_mob_present(
	camera: Query<&Transform, With<Camera3d>>,
	mut present: ResMut<MobPresentBullseye>,
	mut present_regions: MessageWriter<LodPresentRegion<MobLodChan>>,
	mut present_keep: ResMut<LodPresentKeepRegion<MobLodChan>>,
	mut previous_cell: Local<Option<(i32, i32)>>,
) {
	let Ok(camera) = camera.single() else {
		return;
	};
	present.enabled = true;
	let present_aabb = xz_radius_aabb(camera.translation, MOB_PRESENT_RADIUS);
	present_keep.region = Some(present_aabb);
	let current = MobCellExtent::cell_index_containing(camera.translation);
	if previous_cell.as_ref() == Some(&current) {
		return;
	}
	present_regions.write(LodPresentRegion::new(present_aabb));
	*previous_cell = Some(current);
}

/// Generate keep and grid-cell re-arm. A grid scheme installs this.
pub fn stream_mob_generate(
	camera: Query<&Transform, With<Camera3d>>,
	mut generate: ResMut<MobGenerateBullseye>,
	mut generate_regions: MessageWriter<LodGenerateRegion<MobLodChan>>,
	mut generate_keep: ResMut<LodGenerateKeepRegion<MobLodChan>>,
	mut previous_cell: Local<Option<(i32, i32)>>,
) {
	let Ok(camera) = camera.single() else {
		return;
	};
	generate.enabled = true;
	let generate_aabb = xz_radius_aabb(camera.translation, MOB_GENERATE_RADIUS);
	generate_keep.region = Some(generate_aabb);
	let current = MobCellExtent::cell_index_containing(camera.translation);
	if previous_cell.as_ref() == Some(&current) {
		return;
	}
	generate_regions.write(LodGenerateRegion::new(generate_aabb));
	*previous_cell = Some(current);
}

fn clear_mob_generate(
	mut generate: ResMut<MobGenerateBullseye>,
	mut generate_keep: ResMut<LodGenerateKeepRegion<MobLodChan>>,
) {
	generate.enabled = false;
	generate_keep.region = None;
}

/// Grid stream for a mode that owns hopscotch cells.
pub fn install_mob_grid_stream<Mode: terrain_layer_model::GenerationMode>(app: &mut App) {
	use lod::LodGenerateSystems;
	use lod::LodPresentSystems;
	use terrain_layer_model::{ActiveGenerationMode, GenerationModeSystems};

	use crate::generation::MobGenerationSystems;

	app.add_systems(
		Update,
		stream_mob_generate
			.in_set(GenerationModeSystems::<Mode>::default())
			.in_set(MobGenerationSystems)
			.before(LodGenerateSystems::Produce)
			.before(LodPresentSystems::Produce),
	);
	app.add_systems(
		OnExit(ActiveGenerationMode::of::<Mode>()),
		clear_mob_generate,
	);
}
