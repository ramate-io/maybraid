//! Presenter announce, indexed refresh, and the mode-hop despawn timing.

use bevy::app::{App, Plugin};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::{AssetPlugin, ChildOf, MinimalPlugins, NextState, PostUpdate, With};
use bevy::state::app::StatesPlugin;
use chico::Chico;
use durham::Durham;
use layer_stack::{
	subscribe_mode, ActiveGenerationMode, GenerationMode, GenerationModePlugin, ModeSubscribers,
};
use lod::gen::{Id, Version};
use mob_intelligence::MemberOf;
use mob_layer_model::Mobs;
use mob_layer_presentation::{
	MobPresent, MobPresentationCore, MobPresentationPlugin, MobPresenterState,
};
use mob_scenes::{MobLodRefreshMode, DEFAULT_MOB_HIGH_RADIUS};
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;
use vegetation_layer_model::Vegetation;

use crate::present::{MobCellRoot, MobHighLodRegion, MOB_HIGH_LOD_REFRESH_RADIUS};
use crate::stream::MobLodChan;
use crate::{Barking, MobCell, MobIndex};

type Urbanized = Urbanization<richmond::Richmond<OnTerrain<Durham>>>;
type Vegetated = Vegetation<Chico<Urbanized>>;
type Inhabited = Mobs<Barking<Vegetated>>;

struct TestMode;

impl GenerationMode for TestMode {}

struct OtherMode;

impl GenerationMode for OtherMode {}

#[test]
fn high_lod_index_region_follows_the_viewer_in_three_dimensions() -> anyhow::Result<()> {
	let center = Vec3::new(10.0, 120.0, -20.0);
	let region = MobHighLodRegion::region_at(center);
	anyhow::ensure!(
		Vec3::from(region.min) == center - Vec3::splat(MOB_HIGH_LOD_REFRESH_RADIUS),
		"region min"
	);
	anyhow::ensure!(
		Vec3::from(region.max) == center + Vec3::splat(MOB_HIGH_LOD_REFRESH_RADIUS),
		"region max"
	);
	Ok(())
}

#[test]
fn high_lod_refresh_keeps_margin_around_the_high_band() -> anyhow::Result<()> {
	anyhow::ensure!(MOB_HIGH_LOD_REFRESH_RADIUS == 250.0, "refresh cube");
	anyhow::ensure!(
		MOB_HIGH_LOD_REFRESH_RADIUS > DEFAULT_MOB_HIGH_RADIUS,
		"margin past the high sphere"
	);
	Ok(())
}

#[test]
fn retire_despawns_presented_roots_and_pending_while_unsubscribed() -> anyhow::Result<()> {
	let mut app = present_app();
	subscribe_mode::<(Inhabited, MobPresent), TestMode>(&mut app);
	let root = app.world_mut().spawn(MobCellRoot).id();
	let pending = app.world_mut().spawn_empty().id();
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	app.world_mut()
		.resource_mut::<MobPresenterState>()
		.remember(id, Version(1), vec![root]);
	app.world_mut().resource_mut::<MobPresenterState>().queue(vec![pending]);
	app.update();
	anyhow::ensure!(app.world().get_entity(root).is_ok(), "subscribed mode keeps presented roots");

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	anyhow::ensure!(app.world().get_entity(root).is_err(), "Last drain despawns the root");
	anyhow::ensure!(app.world().get_entity(pending).is_err(), "Last drain despawns pending");
	Ok(())
}

#[test]
fn presentation_inserts_indexed_refresh_mode() -> anyhow::Result<()> {
	use bevy::ecs::system::IntoSystem;
	use bevy::prelude::{System, Update};
	use lod::{cull_lod_level_roots, update_lod_host_levels, LodViewer};
	use mob_scenes::MobScene;

	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()));
	MobPresentationPlugin::<TestMode, Barking<Vegetated>>::default().build(&mut app);
	let mut ids = Vec::new();
	let mut inspect_error = None;
	app.world_mut().schedule_scope(Update, |world, schedule| {
		if let Err(error) = schedule.initialize(world) {
			inspect_error = Some(anyhow::anyhow!("{error:?}"));
			return;
		}
		match schedule.systems() {
			Ok(systems) => ids.extend(systems.map(|(_, system)| system.system_type())),
			Err(error) => inspect_error = Some(anyhow::anyhow!("{error:?}")),
		}
	});
	if let Some(error) = inspect_error {
		return Err(error);
	}
	let cull_id = IntoSystem::into_system(cull_lod_level_roots::<MobScene, (), With<LodViewer>>)
		.system_type();
	let update_id =
		IntoSystem::into_system(update_lod_host_levels::<MobScene, (), With<LodViewer>>)
			.system_type();
	let culls = ids.iter().filter(|id| **id == cull_id).count();
	let host_levels = ids.iter().filter(|id| **id == update_id).count();
	anyhow::ensure!(
		culls <= 1,
		"MobScenes Indexed must not add a second cull_lod_level_roots, got {culls}"
	);
	anyhow::ensure!(
		host_levels == 1,
		"indexed refresh must register one update_lod_host_levels, got {host_levels}"
	);
	anyhow::ensure!(
		*app.world().resource::<MobLodRefreshMode>() == MobLodRefreshMode::Indexed,
		"presentation installs indexed refresh"
	);
	Ok(())
}

#[test]
fn two_modes_share_one_presentation_core() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin));
	app.add_plugins((
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		MobPresentationPlugin::<TestMode, Barking<Vegetated>>::default(),
		MobPresentationPlugin::<OtherMode, Barking<Vegetated>>::default(),
	));
	anyhow::ensure!(
		app.is_plugin_added::<MobPresentationCore<Barking<Vegetated>>>(),
		"core is installed"
	);
	let subscribers = app.world().resource::<ModeSubscribers<(Inhabited, MobPresent)>>();
	anyhow::ensure!(subscribers.contains::<TestMode>());
	anyhow::ensure!(subscribers.contains::<OtherMode>());
	Ok(())
}

fn teardown_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	mob_layer_presentation::install_mob_cell_teardown(&mut app);
	app
}

fn present_app() -> App {
	use bevy::prelude::{OnExit, With};
	use lod::{LodPresentCullPlugin, LodPresentPlugin, LodViewer};

	use crate::present::BarkingPresenter;
	use crate::tests::insert_urbanized_resources;

	let mut app = teardown_app();
	layer_stack::install_lod_present_gate::<(Inhabited, MobPresent), MobLodChan>(&mut app);
	app.add_plugins(
		(
			AssetPlugin::default(),
			LodPresentPlugin::<
				MobCell,
				MobIndex,
				BarkingPresenter<'_, '_, Urbanized>,
				MobLodChan,
				With<LodViewer>,
			>::default(),
			LodPresentCullPlugin::<
				MobCell,
				MobIndex,
				BarkingPresenter<'_, '_, Urbanized>,
				MobLodChan,
			>::default(),
		),
	);
	app.add_systems(OnExit(ActiveGenerationMode::of::<TestMode>()), clear_index);
	app.add_systems(OnExit(ActiveGenerationMode::of::<OtherMode>()), clear_index);
	insert_urbanized_resources(app.world_mut());
	app.init_resource::<MobIndex>();
	app.add_message::<mob_layer_model::MobCellPresented>();
	app
}

fn clear_index(mut index: bevy::prelude::ResMut<MobIndex>) {
	index.clear();
}

fn cover_origin_keep(app: &mut App) {
	use lod::presentation::LodPresentKeepRegion;

	app.insert_resource({
		let mut keep = LodPresentKeepRegion::<MobLodChan>::default();
		keep.region = Some(Aabb3d::from_min_max(
			Vec3::new(-2_000.0, -1_000.0, -2_000.0),
			Vec3::new(2_000.0, 1_000.0, 2_000.0),
		));
		keep
	});
}

fn spawn_viewer(app: &mut App) {
	use bevy::prelude::Transform;
	use lod::lod_ref::{LodNode, LodNodePose};
	use lod::LodViewer;

	app.world_mut()
		.spawn((LodViewer, LodNode, LodNodePose::default(), Transform::IDENTITY));
}

#[derive(bevy::prelude::Resource, Default)]
struct SquadSeenInPostUpdate(bool);

fn note_squad_before_last(
	roots: bevy::prelude::Query<(), With<MobCellRoot>>,
	members: bevy::prelude::Query<(), With<MemberOf>>,
	mut seen: bevy::prelude::ResMut<SquadSeenInPostUpdate>,
) {
	seen.0 = !roots.is_empty() && !members.is_empty();
}

struct Squad {
	root: bevy::prelude::Entity,
	host: bevy::prelude::Entity,
	member: bevy::prelude::Entity,
	respawned: bevy::prelude::Entity,
}

fn spawn_presented_squad(app: &mut App) -> Squad {
	use mob_layer_presentation::PresentedMobCell;

	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let root = app.world_mut().spawn(MobCellRoot).id();
	let host = app.world_mut().spawn((PresentedMobCell(id), ChildOf(root))).id();
	let member = app.world_mut().spawn(MemberOf { mob: host, slot: 0 }).id();
	let respawned = app.world_mut().spawn(MemberOf { mob: host, slot: 1 }).id();
	app.world_mut()
		.resource_mut::<MobPresenterState>()
		.remember(id, Version(1), vec![root, host]);
	Squad { root, host, member, respawned }
}

#[test]
fn retired_hosts_and_members_survive_post_update_then_leave_in_last() -> anyhow::Result<()> {
	let mut app = present_app();
	subscribe_mode::<(Inhabited, MobPresent), TestMode>(&mut app);
	cover_origin_keep(&mut app);
	app.init_resource::<SquadSeenInPostUpdate>();
	app.add_systems(PostUpdate, note_squad_before_last);
	app.update();

	let squad = spawn_presented_squad(&mut app);
	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();

	anyhow::ensure!(
		app.world().resource::<SquadSeenInPostUpdate>().0,
		"hosts and members survive PostUpdate on the exit frame"
	);
	anyhow::ensure!(
		app.world().get_entity(squad.root).is_err(),
		"Last drain despawns the cell root"
	);
	anyhow::ensure!(app.world().get_entity(squad.host).is_err(), "Last drain despawns the host");
	anyhow::ensure!(
		app.world().get_entity(squad.member).is_err(),
		"Last drain despawns the member"
	);
	anyhow::ensure!(
		app.world().get_entity(squad.respawned).is_err(),
		"Last drain despawns a respawned member"
	);
	Ok(())
}

#[test]
fn announced_cell_presents_after_the_first_keep_scan() -> anyhow::Result<()> {
	use bevy::ecs::system::RunSystemOnce;

	use crate::MobCellExtent;
	use crate::MobCellWrites;

	let mut app = present_app();
	subscribe_mode::<(Inhabited, MobPresent), TestMode>(&mut app);
	cover_origin_keep(&mut app);
	spawn_viewer(&mut app);
	app.update();
	anyhow::ensure!(
		!app.world()
			.resource::<MobPresenterState>()
			.presents(MobCellExtent::from_cell_index(0, 0).id()),
		"the first keep scan finds nothing before the cell exists"
	);

	let extent = MobCellExtent::from_cell_index(0, 0);
	let id = app
		.world_mut()
		.run_system_once(move |mut cells: MobCellWrites| {
			cells.insert(MobCell { extent, groups: Vec::new() })
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	app.update();
	anyhow::ensure!(
		app.world().resource::<MobPresenterState>().presents(id),
		"MobCellWrites::insert announces so the presenter can enqueue the cell"
	);
	let spawned = app
		.world_mut()
		.run_system_once(|roots: bevy::prelude::Query<&MobCellRoot>| !roots.is_empty())
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(spawned, "the real presenter spawned the announced cell");
	Ok(())
}

#[test]
fn presented_cells_leave_when_the_subscribed_mode_changes() -> anyhow::Result<()> {
	let mut app = present_app();
	subscribe_mode::<(Inhabited, MobPresent), TestMode>(&mut app);
	subscribe_mode::<(Inhabited, MobPresent), OtherMode>(&mut app);
	cover_origin_keep(&mut app);
	app.init_resource::<SquadSeenInPostUpdate>();
	app.add_systems(PostUpdate, note_squad_before_last);
	app.update();

	let squad = spawn_presented_squad(&mut app);
	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();

	anyhow::ensure!(
		app.world().resource::<SquadSeenInPostUpdate>().0,
		"hosts and members survive PostUpdate on the exit frame"
	);
	anyhow::ensure!(
		app.world().get_entity(squad.root).is_err(),
		"Last drain despawns the cell root"
	);
	anyhow::ensure!(app.world().get_entity(squad.host).is_err(), "Last drain despawns the host");
	anyhow::ensure!(
		app.world().get_entity(squad.member).is_err(),
		"Last drain despawns the member"
	);
	anyhow::ensure!(
		app.world().get_entity(squad.respawned).is_err(),
		"Last drain despawns a respawned member"
	);
	Ok(())
}

fn write_cell_in_other_mode(mut cells: crate::MobCellWrites) {
	use crate::MobCellExtent;

	cells.insert(MobCell { extent: MobCellExtent::from_cell_index(1, 0), groups: Vec::new() });
}

#[test]
fn a_cell_written_on_the_entering_frame_still_presents() -> anyhow::Result<()> {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::{IntoScheduleConfigs, Update};
	use layer_stack::in_generation_mode;
	use lod::LodPresentSystems;
	use mob_layer_model::MobGenerationSystems;

	use crate::MobCellExtent;
	use crate::MobCellWrites;

	let mut app = present_app();
	subscribe_mode::<(Inhabited, MobPresent), TestMode>(&mut app);
	subscribe_mode::<(Inhabited, MobPresent), OtherMode>(&mut app);
	cover_origin_keep(&mut app);
	spawn_viewer(&mut app);
	app.add_systems(
		Update,
		write_cell_in_other_mode
			.run_if(in_generation_mode::<OtherMode>())
			.in_set(MobGenerationSystems)
			.before(LodPresentSystems::Produce),
	);
	app.update();

	let leaving = app
		.world_mut()
		.run_system_once(|mut cells: MobCellWrites| {
			cells.insert(MobCell {
				extent: MobCellExtent::from_cell_index(0, 0),
				groups: Vec::new(),
			})
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	app.update();
	anyhow::ensure!(
		app.world().resource::<MobPresenterState>().presents(leaving),
		"the leaving mode's cell is presented before the switch"
	);

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();

	let entering = MobCellExtent::from_cell_index(1, 0).id();
	anyhow::ensure!(
		!app.world().resource::<MobPresenterState>().presents(leaving),
		"the leaving cell is retired on the switch"
	);
	anyhow::ensure!(
		app.world().resource::<MobPresenterState>().presents(entering),
		"a cell written on the entering frame presents after the retire"
	);
	Ok(())
}
