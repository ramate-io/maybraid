//! Shared [`App`] wiring for refresh pipeline tests (scan index, no Avian).

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use bevy::scene::prelude::Scene;

use crate::lod_ref::LodRef;
use crate::scene::host::{LodLevelRoot, LodLevelRoots, LodSceneHost};
use crate::scene::level::LodSceneLevel;
use crate::scene::region_index::LodSceneRegionIndex;
use crate::scene::LodScene;

use super::super::{
	Bullseye, LodCullRegionCursor, LodHostBounds, LodRefreshCorePlugin, LodRefreshSystems,
	LodSceneCullRegion, LodSceneCullRegionPlugin, LodSceneRefreshEntitiesPlugin,
	LodSceneRefreshLevelsPlugin, LodSceneRefreshRegion, LodSceneRefreshRegionPlugin, LodViewer,
	OpenLattice, Spotlight,
};

/// Channel marker for spotlight region / level tests.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpotChan;

/// Channel marker for bullseye region / level tests.
#[derive(Debug, Clone, Copy, Default)]
pub struct BullChan;

/// Channel marker for OpenLattice cull-region tests.
#[derive(Debug, Clone, Copy, Default)]
pub struct CullChan;

/// Test host: bands on viewer distance from the origin.
#[derive(Component, Clone, Default)]
pub struct Probe;

impl LodScene for Probe {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		let d = lod_ref.current_transform.translation.length();
		if d < 25.0 {
			LodSceneLevel::High
		} else if d < 80.0 {
			LodSceneLevel::Medium
		} else {
			LodSceneLevel::Low
		}
	}

	fn scene_with_level(&self, _: &LodRef, _: LodSceneLevel) -> impl Scene + 'static {}
}

/// Brute-force [`LodSceneRegionIndex`]: world AABB from translation + local bounds.
#[derive(SystemParam)]
pub struct ScanIndex<'w, 's, T: Component + LodScene + 'static> {
	hosts: Query<
		'w,
		's,
		(Entity, &'static T, &'static Transform, &'static LodHostBounds),
		With<LodSceneHost>,
	>,
}

impl<T: Component + LodScene + 'static> LodSceneRegionIndex<T> for ScanIndex<'_, '_, T> {
	fn hosts_in_region<'a>(
		&'a mut self,
		region: Aabb3d,
	) -> impl Iterator<Item = (Entity, &'a T)> + 'a {
		self.hosts.iter().filter_map(move |(entity, scene, transform, bounds)| {
			let t = transform.translation;
			let world =
				Aabb3d::from_min_max(Vec3::from(bounds.0.min) + t, Vec3::from(bounds.0.max) + t);
			world.intersects(&region).then_some((entity, scene))
		})
	}
}

/// This-frame [`LodSceneRefreshRegion<M>`] AABBs (cleared each produce tick).
#[derive(Resource)]
pub struct NewRegions<M: Send + Sync + 'static> {
	pub regions: Vec<Aabb3d>,
	_marker: std::marker::PhantomData<M>,
}

impl<M: Send + Sync + 'static> Default for NewRegions<M> {
	fn default() -> Self {
		Self { regions: Vec::new(), _marker: std::marker::PhantomData }
	}
}

/// This-frame [`LodSceneCullRegion<M>`] AABBs.
#[derive(Resource)]
pub struct NewCullRegions<M: Send + Sync + 'static> {
	pub regions: Vec<Aabb3d>,
	_marker: std::marker::PhantomData<M>,
}

impl<M: Send + Sync + 'static> Default for NewCullRegions<M> {
	fn default() -> Self {
		Self { regions: Vec::new(), _marker: std::marker::PhantomData }
	}
}

pub fn capture_regions<M: Send + Sync + 'static>(
	mut reader: MessageReader<LodSceneRefreshRegion<M>>,
	mut log: ResMut<NewRegions<M>>,
) {
	log.regions.clear();
	log.regions.extend(reader.read().map(|m| m.region));
}

pub fn capture_cull_regions<M: Send + Sync + 'static>(
	mut reader: MessageReader<LodSceneCullRegion<M>>,
	mut log: ResMut<NewCullRegions<M>>,
) {
	log.regions.clear();
	log.regions.extend(reader.read().map(|m| m.region));
}

pub fn unit_bounds() -> LodHostBounds {
	LodHostBounds(Aabb3d::from_min_max(Vec3::splat(-1.0), Vec3::splat(1.0)))
}

pub fn spawn_viewer(world: &mut World, at: Vec3) -> Entity {
	world.spawn((LodViewer, Transform::from_translation(at))).id()
}

pub fn spawn_host(world: &mut World, at: Vec3, level: LodSceneLevel) -> Entity {
	world
		.spawn((LodSceneHost, Probe, level, unit_bounds(), Transform::from_translation(at)))
		.id()
}

pub fn move_viewer(app: &mut App, viewer: Entity, at: Vec3) {
	app.world_mut().entity_mut(viewer).insert(Transform::from_translation(at));
}

pub fn host_level(app: &App, host: Entity) -> LodSceneLevel {
	*app.world().get::<LodSceneLevel>(host).expect("host LodSceneLevel")
}

pub fn pose(app: &App, viewer: Entity) -> (Vec3, Vec3) {
	let pose = app.world().get::<crate::lod_ref::LodNodePose>(viewer).expect("LodNodePose");
	(pose.previous.translation, pose.current.translation)
}

pub fn app_core() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins).add_plugins(LodRefreshCorePlugin);
	app
}

/// Spotlight region production only (no level writes).
pub fn app_spotlight_regions() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(Spotlight::new(50.0))
		.add_plugins(LodSceneRefreshRegionPlugin::<Spotlight, With<LodViewer>, SpotChan>::default())
		.init_resource::<NewRegions<SpotChan>>()
		.add_systems(Update, capture_regions::<SpotChan>.after(LodRefreshSystems::ProduceRegions));
	app
}

/// Bullseye region production only.
pub fn app_bullseye_regions() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(Bullseye::new(50.0, 500.0))
		.add_plugins(LodSceneRefreshRegionPlugin::<Bullseye, With<LodViewer>, BullChan>::default())
		.init_resource::<NewRegions<BullChan>>()
		.add_systems(Update, capture_regions::<BullChan>.after(LodRefreshSystems::ProduceRegions));
	app
}

/// Spotlight → scan index → level write. Extent 200 so origin hosts stay in-region
/// while the viewer walks High / Medium / Low bands.
pub fn app_spotlight_levels() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(Spotlight::new(200.0))
		.add_plugins(LodSceneRefreshRegionPlugin::<Spotlight, With<LodViewer>, SpotChan>::default())
		.add_plugins(LodSceneRefreshLevelsPlugin::<
			ScanIndex<Probe>,
			SpotChan,
			Probe,
			With<LodViewer>,
		>::default())
		.init_resource::<NewRegions<SpotChan>>()
		.add_systems(Update, capture_regions::<SpotChan>.after(LodRefreshSystems::ProduceRegions));
	app
}

/// Bullseye + Spotlight channels, one host type (untyped level bus).
pub fn app_dual_channel_levels() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(Spotlight::new(200.0))
		.insert_resource(Bullseye::new(50.0, 500.0))
		.add_plugins(LodSceneRefreshRegionPlugin::<Spotlight, With<LodViewer>, SpotChan>::default())
		.add_plugins(LodSceneRefreshRegionPlugin::<Bullseye, With<LodViewer>, BullChan>::default())
		.add_plugins(LodSceneRefreshLevelsPlugin::<
			ScanIndex<Probe>,
			SpotChan,
			Probe,
			With<LodViewer>,
		>::default())
		.add_plugins(LodSceneRefreshLevelsPlugin::<
			ScanIndex<Probe>,
			BullChan,
			Probe,
			With<LodViewer>,
		>::default())
		.init_resource::<NewRegions<SpotChan>>()
		.init_resource::<NewRegions<BullChan>>()
		.add_systems(
			Update,
			(capture_regions::<SpotChan>, capture_regions::<BullChan>)
				.after(LodRefreshSystems::ProduceRegions),
		);
	app
}

pub fn app_entities_only() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.add_plugins(LodSceneRefreshEntitiesPlugin::<Probe>::default());
	app
}

pub fn app_open_lattice() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(OpenLattice::new(1000.0, 3000.0, 500.0))
		.insert_resource(LodCullRegionCursor::default().with_regions_per_tick(1))
		.add_plugins(LodSceneCullRegionPlugin::<OpenLattice, With<LodViewer>, CullChan>::default())
		.init_resource::<NewCullRegions<CullChan>>()
		.add_systems(
			Update,
			capture_cull_regions::<CullChan>.after(LodRefreshSystems::ProduceRegions),
		);
	app
}

pub fn spawn_nested_pair(world: &mut World) -> (Entity, Entity, Entity) {
	let parent = spawn_host(world, Vec3::ZERO, LodSceneLevel::High);
	let bag = world
		.spawn((LodLevelRoots, Transform::default(), Visibility::Inherited, ChildOf(parent)))
		.id();
	let shown = world
		.spawn((
			LodLevelRoot(LodSceneLevel::High),
			Transform::default(),
			Visibility::Inherited,
			ChildOf(bag),
		))
		.id();
	let hidden = world
		.spawn((
			LodLevelRoot(LodSceneLevel::Low),
			Transform::default(),
			Visibility::Hidden,
			ChildOf(bag),
		))
		.id();
	let allowed = world
		.spawn((
			LodSceneHost,
			Probe,
			LodSceneLevel::UltraLow,
			unit_bounds(),
			Transform::from_translation(Vec3::ZERO),
			ChildOf(shown),
		))
		.id();
	let blocked = world
		.spawn((
			LodSceneHost,
			Probe,
			LodSceneLevel::UltraLow,
			unit_bounds(),
			Transform::from_translation(Vec3::ZERO),
			ChildOf(hidden),
		))
		.id();
	(parent, allowed, blocked)
}
