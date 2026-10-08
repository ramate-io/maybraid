//! Mob generate / present-keep bullseyes and the camera stream.

use bevy::ecs::system::{ParamSet, StaticSystemParam, SystemParam};
use bevy::prelude::*;
use lod::gen::{Id, LodGenerateKeepRegion, LodGenerateQueue, LodGenerateRegion, LodGenerated};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion};
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};
use lod::LodJobCounter;
use mob_layer_model::MobGenerationSystems;
use terrain_layer_model::TerrainView;

use crate::index::{xz_radius_aabb, MobCell, MobCellExtent, MobIndex};
use crate::sample::{
	DiscoverablePlaces, ForestSelection, PlantHosts, SelectUrbanization, UrbanSelection,
};

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

/// Last camera cell the grid generate stream armed. Cleared when that mode exits.
#[derive(Resource, Default)]
pub(crate) struct MobGenerateStreamCell(Option<(i32, i32)>);

/// Scheme-owned writes. [`Self::insert`] stores the cell and announces it;
/// membership alone only drives stale removal.
#[derive(SystemParam)]
pub struct MobCellWrites<'w> {
	index: ResMut<'w, MobIndex>,
	generated: MessageWriter<'w, LodGenerated<MobCell>>,
}

impl MobCellWrites<'_> {
	pub fn insert(&mut self, cell: MobCell) -> Id {
		let id = self.index.insert_cell(cell);
		self.generated.write(LodGenerated::new(id));
		id
	}

	pub fn remove(&mut self, id: Id) -> Option<MobCell> {
		self.index.remove_cell(id)
	}
}

pub fn sync_mob_models<V, G>(
	forest: StaticSystemParam<V::Read>,
	view: TerrainView<G>,
	mut mobs: ResMut<MobIndex>,
) where
	V: ForestSelection,
	G: UrbanSelection,
{
	let (forest_noise, forest_layering) = V::pick(&forest);
	let (urbanization_noise, urbanization_kind) = G::selection(&view.read);
	if !mobs.models_match(forest_noise, forest_layering, urbanization_noise, urbanization_kind) {
		mobs.configure_from(
			forest_noise,
			forest_layering,
			urbanization_noise,
			urbanization_kind,
			V::layers_at,
			G::kind_at,
		);
	}
}

/// Collect plant hosts from urbanization leaves, development cells, then
/// presentation-spawned settings and discoverable places.
///
/// Generation reads entities that presentation spawns.
///
/// `ensure_selected` is the #720 cross-layer write into Richmond's urbanization index.
pub fn sync_mob_plant_hosts<G>(
	generate_keep: Res<LodGenerateKeepRegion<MobLodChan>>,
	mut access: ParamSet<(G::Select, TerrainView<G>)>,
	places: StaticSystemParam<G::Places>,
	mut mobs: ResMut<MobIndex>,
) where
	G: PlantHosts + SelectUrbanization + DiscoverablePlaces,
{
	if !mobs.models_ready {
		return;
	}
	let region = generate_keep.region.unwrap_or(xz_radius_aabb(Vec3::ZERO, MOB_GENERATE_RADIUS));
	G::ensure_selected(&mut access.p0(), region);
	let mut hosts = {
		let view = access.p1();
		G::plant_hosts(&view.read, region)
	};
	hosts.extend(G::places(&places));
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
	mut previous_cell: ResMut<MobGenerateStreamCell>,
) {
	let Ok(camera) = camera.single() else {
		return;
	};
	generate.enabled = true;
	let generate_aabb = xz_radius_aabb(camera.translation, MOB_GENERATE_RADIUS);
	generate_keep.region = Some(generate_aabb);
	let current = MobCellExtent::cell_index_containing(camera.translation);
	if previous_cell.0.as_ref() == Some(&current) {
		return;
	}
	generate_regions.write(LodGenerateRegion::new(generate_aabb));
	previous_cell.0 = Some(current);
}

fn clear_mob_generate(
	mut generate: ResMut<MobGenerateBullseye>,
	mut generate_keep: ResMut<LodGenerateKeepRegion<MobLodChan>>,
	mut previous_cell: ResMut<MobGenerateStreamCell>,
	queue: Option<ResMut<LodGenerateQueue<MobCell>>>,
	jobs: Option<Res<LodJobCounter>>,
) {
	generate.enabled = false;
	generate_keep.region = None;
	previous_cell.0 = None;
	if let Some(mut queue) = queue {
		let cancelled = queue.clear();
		if let Some(jobs) = jobs {
			jobs.end_n(cancelled);
		}
	}
}

/// Grid stream for a mode that owns hopscotch cells.
pub fn install_mob_grid_stream<Mode: layer_stack::GenerationMode>(app: &mut App) {
	use layer_stack::{ActiveGenerationMode, GenerationModeSystems};
	use lod::{LodGenerateSystems, LodPresentSystems};

	app.init_resource::<MobGenerateStreamCell>();
	app.add_systems(
		Update,
		stream_mob_generate
			.in_set(GenerationModeSystems::<Mode>::default())
			.in_set(MobGenerationSystems)
			.before(LodGenerateSystems::Produce)
			.before(LodPresentSystems::Produce),
	);
	app.add_systems(OnExit(ActiveGenerationMode::of::<Mode>()), clear_mob_generate);
}
