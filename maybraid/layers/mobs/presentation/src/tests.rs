use bevy::app::{App, Plugin};
use bevy::prelude::{AssetPlugin, MinimalPlugins, World};
use durham_terrain_models::Durham;
use lod::gen::Id;
use maybraid_mobs::{DEFAULT_MOB_HIGH_RADIUS, MobLodRefreshMode};
use mob_layer_model::MobStreamSuspended;
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;

use crate::present::{
	retire_mob_presenters, MobCellRoot, MobHighLodRegion, MobPresenterState,
	MOB_HIGH_LOD_REFRESH_RADIUS,
};
use crate::MobPresentationPlugin;

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

#[test]
fn retire_despawns_presented_roots_and_pending_while_suspended() -> anyhow::Result<()> {
	use bevy::ecs::system::RunSystemOnce;

	let mut world = World::new();
	world.insert_resource(MobStreamSuspended(true));
	world.init_resource::<MobPresenterState>();
	let root = world.spawn(MobCellRoot).id();
	let pending = world.spawn_empty().id();
	let id = Id::from_cell(bevy::math::bounding::Aabb3d::from_min_max(
		bevy::math::Vec3::ZERO,
		bevy::math::Vec3::ONE,
	));
	world.resource_mut::<MobPresenterState>().insert_presented(id, vec![root]);
	world.resource_mut::<MobPresenterState>().push_pending(vec![pending]);
	world
		.run_system_once(retire_mob_presenters)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(world.get_entity(root).is_err(), "presented root should despawn");
	anyhow::ensure!(world.get_entity(pending).is_err(), "pending entity should despawn");
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
	MobPresentationPlugin::<Urbanized>::default().build(&mut app);
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
#[should_panic(expected = "MobGenerationPlugin")]
fn presentation_without_generation_names_the_missing_plugin() {
	MobPresentationPlugin::<Urbanized>::default().finish(&mut App::new());
}
