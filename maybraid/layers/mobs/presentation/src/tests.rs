use bevy::app::{App, Plugin};
use bevy::prelude::{AssetPlugin, MinimalPlugins, NextState};
use bevy::state::app::StatesPlugin;
use durham_terrain_models::Durham;
use lod::gen::Id;
use maybraid_mobs::{DEFAULT_MOB_HIGH_RADIUS, MobLodRefreshMode};
use terrain_layer_model::{
	subscribe_mode, ActiveGenerationMode, GenerationMode, GenerationModePlugin, ModeSubscribers,
	OnTerrain,
};
use urbanization_layer_model::Urbanization;

use crate::present::{
	drain_retired_mob_cells, retire_mob_presenters, MobCellRoot, MobHighLodRegion,
	MobPresenterState, PresentedMobCell, MOB_HIGH_LOD_REFRESH_RADIUS,
};
use crate::{MobPresent, MobPresentationPlugin};

type Urbanized = Urbanization<OnTerrain<Durham>>;

#[test]
fn high_lod_index_region_follows_the_viewer_in_three_dimensions() {
	let center = bevy::math::Vec3::new(10.0, 120.0, -20.0);
	let region = MobHighLodRegion::region_at(center);
	assert_eq!(
		bevy::math::Vec3::from(region.min),
		center - bevy::math::Vec3::splat(MOB_HIGH_LOD_REFRESH_RADIUS)
	);
	assert_eq!(
		bevy::math::Vec3::from(region.max),
		center + bevy::math::Vec3::splat(MOB_HIGH_LOD_REFRESH_RADIUS)
	);
}

#[test]
fn high_lod_refresh_keeps_margin_around_the_high_band() {
	assert_eq!(MOB_HIGH_LOD_REFRESH_RADIUS, 250.0);
	assert!(MOB_HIGH_LOD_REFRESH_RADIUS > DEFAULT_MOB_HIGH_RADIUS);
}

struct TestMode;

impl GenerationMode for TestMode {}

struct OtherMode;

impl GenerationMode for OtherMode {}

#[test]
fn retire_despawns_presented_roots_and_pending_while_unsubscribed() -> anyhow::Result<()> {
	use bevy::ecs::system::RunSystemOnce;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	subscribe_mode::<(Urbanized, MobPresent), TestMode>(&mut app);
	app.init_resource::<MobPresenterState>();
	let root = app.world_mut().spawn(MobCellRoot).id();
	let pending = app.world_mut().spawn_empty().id();
	let id = Id::from_cell(bevy::math::bounding::Aabb3d::from_min_max(
		bevy::math::Vec3::ZERO,
		bevy::math::Vec3::ONE,
	));
	app.world_mut().resource_mut::<MobPresenterState>().insert_presented(id, vec![root]);
	app.world_mut().resource_mut::<MobPresenterState>().push_pending(vec![pending]);
	app.update();
	app.world_mut()
		.run_system_once(retire_mob_presenters::<Urbanized>)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(
		app.world().get_entity(root).is_ok(),
		"subscribed mode keeps presented roots"
	);

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	app.world_mut()
		.run_system_once(retire_mob_presenters::<Urbanized>)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(
		app.world().get_entity(root).is_ok(),
		"retire queues; the root lives through Update"
	);
	anyhow::ensure!(
		app.world().get_entity(pending).is_ok(),
		"pending stays until Last"
	);
	app.world_mut()
		.run_system_once(drain_retired_mob_cells)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(app.world().get_entity(root).is_err(), "Last drain despawns the root");
	anyhow::ensure!(app.world().get_entity(pending).is_err(), "Last drain despawns pending");
	Ok(())
}

#[test]
fn presentation_inserts_indexed_refresh_mode() -> anyhow::Result<()> {
	use bevy::ecs::system::IntoSystem;
	use bevy::prelude::{System, Update, With};
	use lod::{cull_lod_level_roots, update_lod_host_levels, LodViewer};
	use maybraid_mobs::MobScene;

	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()));
	MobPresentationPlugin::<TestMode, Urbanized>::default().build(&mut app);
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
	let cull_id = IntoSystem::into_system(
		cull_lod_level_roots::<MobScene, (), With<LodViewer>>,
	)
	.system_type();
	let update_id = IntoSystem::into_system(
		update_lod_host_levels::<MobScene, (), With<LodViewer>>,
	)
	.system_type();
	let culls = ids.iter().filter(|id| **id == cull_id).count();
	let host_levels = ids.iter().filter(|id| **id == update_id).count();
	// Gimme's default refresh registers one full-scan cull. MobScenes FullScan
	// would add a second cull plus a second `update_lod_host_levels`.
	anyhow::ensure!(
		culls <= 1,
		"MobScenes Indexed must not add a second cull_lod_level_roots, got {culls}"
	);
	anyhow::ensure!(
		host_levels == 1,
		"indexed refresh must register one update_lod_host_levels, got {host_levels}"
	);
	assert_eq!(*app.world().resource::<MobLodRefreshMode>(), MobLodRefreshMode::Indexed);
	Ok(())
}

#[test]
#[should_panic(expected = "MobGenerationCore")]
fn presentation_without_generation_names_the_missing_plugin() {
	MobPresentationPlugin::<TestMode, Urbanized>::default().finish(&mut App::new());
}

#[test]
fn two_modes_install_the_core_once() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin));
	app.add_plugins((
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		MobPresentationPlugin::<TestMode, Urbanized>::default(),
		MobPresentationPlugin::<OtherMode, Urbanized>::default(),
	));
	anyhow::ensure!(
		app.is_plugin_added::<crate::MobPresentationCore<Urbanized>>(),
		"core is installed"
	);
	let subscribers = app.world().resource::<ModeSubscribers<(Urbanized, MobPresent)>>();
	anyhow::ensure!(subscribers.contains::<TestMode>());
	anyhow::ensure!(subscribers.contains::<OtherMode>());
	Ok(())
}

#[derive(bevy::prelude::Resource, Default)]
struct SquadSeenInPostUpdate(bool);

fn note_squad_before_last(
	roots: bevy::prelude::Query<(), bevy::prelude::With<MobCellRoot>>,
	members: bevy::prelude::Query<(), bevy::prelude::With<mob_intelligence::MemberOf>>,
	mut seen: bevy::prelude::ResMut<SquadSeenInPostUpdate>,
) {
	seen.0 = !roots.is_empty() && !members.is_empty();
}

#[test]
fn retired_hosts_and_members_survive_post_update_then_leave_in_last() -> anyhow::Result<()> {
	use bevy::prelude::{ChildOf, Last, PostUpdate, Update};
	use lod::gen::Id;
	use mob_intelligence::MemberOf;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	subscribe_mode::<(Urbanized, MobPresent), TestMode>(&mut app);
	app.init_resource::<MobPresenterState>();
	app.init_resource::<SquadSeenInPostUpdate>();
	let id = Id::from_cell(bevy::math::bounding::Aabb3d::from_min_max(
		bevy::math::Vec3::ZERO,
		bevy::math::Vec3::ONE,
	));
	let root = app.world_mut().spawn(MobCellRoot).id();
	let host = app.world_mut().spawn((PresentedMobCell(id), ChildOf(root))).id();
	let member = app.world_mut().spawn(MemberOf { mob: host, slot: 0 }).id();
	let respawned = app.world_mut().spawn(MemberOf { mob: host, slot: 1 }).id();
	app.world_mut()
		.resource_mut::<MobPresenterState>()
		.insert_presented(id, vec![root, host]);
	app.add_systems(Update, retire_mob_presenters::<Urbanized>);
	app.add_systems(PostUpdate, note_squad_before_last);
	app.add_systems(Last, drain_retired_mob_cells);

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();

	anyhow::ensure!(
		app.world().resource::<SquadSeenInPostUpdate>().0,
		"hosts and members survive PostUpdate on the exit frame"
	);
	anyhow::ensure!(app.world().get_entity(root).is_err(), "Last drain despawns the cell root");
	anyhow::ensure!(app.world().get_entity(host).is_err(), "Last drain despawns the host");
	anyhow::ensure!(app.world().get_entity(member).is_err(), "Last drain despawns the member");
	anyhow::ensure!(
		app.world().get_entity(respawned).is_err(),
		"Last drain despawns a respawned member"
	);
	Ok(())
}

#[test]
fn announced_cell_presents_after_the_first_keep_scan() -> anyhow::Result<()> {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::math::bounding::Aabb3d;
	use bevy::math::Vec3;
	use bevy::prelude::{AssetPlugin, Transform};
	use durham_terrain_models::{
		BaseTerrainNoise, TerrainCellLayout, TerrainConfig, TerrainEntryStore, WorldBaseTerrain,
	};
	use lod::gen::LodGenerated;
	use lod::lod_ref::{LodNode, LodNodePose};
	use lod::presentation::LodPresentKeepRegion;
	use lod::{LodPresentPlugin, LodViewer};
	use mob_layer_model::{announce_mob_cell, MobCell, MobCellExtent, MobIndex, MobLodChan};
	use richmond_development_models::DevelopmentEntryStore;
	use richmond_urbanization::UrbanizationIndex;

	use crate::present::MobPresenter;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		LodPresentPlugin::<
			mob_layer_model::MobCell,
			MobIndex,
			MobPresenter<'_, '_, Urbanized>,
			MobLodChan,
			bevy::prelude::With<LodViewer>,
		>::default(),
	));
	subscribe_mode::<(Urbanized, MobPresent), TestMode>(&mut app);
	app.init_resource::<MobPresenterState>();
	app.insert_resource(TerrainEntryStore::default());
	app.insert_resource(TerrainCellLayout::default());
	app.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
		&TerrainConfig::new(42),
	)));
	app.insert_resource(DevelopmentEntryStore::default());
	app.insert_resource(UrbanizationIndex::default());
	app.insert_resource(MobIndex::default());
	app.insert_resource({
		let mut keep = LodPresentKeepRegion::<MobLodChan>::default();
		keep.region = Some(Aabb3d::from_min_max(
			Vec3::new(-2_000.0, -1_000.0, -2_000.0),
			Vec3::new(2_000.0, 1_000.0, 2_000.0),
		));
		keep
	});
	app.world_mut().spawn((
		LodViewer,
		LodNode,
		LodNodePose::default(),
		Transform::IDENTITY,
	));
	app.update();
	anyhow::ensure!(
		!app.world().resource::<MobPresenterState>().presents(MobCellExtent::from_cell_index(0, 0).id()),
		"the first keep scan finds nothing before the cell exists"
	);

	let extent = MobCellExtent::from_cell_index(0, 0);
	let id = app
		.world_mut()
		.resource_mut::<MobIndex>()
		.insert_cell(MobCell { extent, groups: Vec::new() });
	app.update();
	anyhow::ensure!(
		!app.world().resource::<MobPresenterState>().presents(id),
		"inserting a cell after the first scan does not present it"
	);

	app.world_mut()
		.run_system_once(move |mut generated: bevy::prelude::MessageWriter<LodGenerated<MobCell>>| {
			announce_mob_cell(id, &mut generated);
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	app.update();
	anyhow::ensure!(
		app.world().resource::<MobPresenterState>().presents(id),
		"LodGenerated is what presents a scheme-written cell"
	);
	let spawned = app
		.world_mut()
		.run_system_once(|roots: bevy::prelude::Query<&MobCellRoot>| !roots.is_empty())
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(spawned, "the real presenter spawned the announced cell");
	Ok(())
}
