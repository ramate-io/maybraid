//! App tests for [`super::GenerationPlugin`] and [`super::PresentationPlugin`],
//! with the real worker.

use std::time::Duration;

use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::*;
use bevy::scene::ScenePlugin;

use crate::gen::tests::test_utils::{cell, Terrain, Vegetation};
use crate::gen::{Id, Version};
use crate::lod_ref::LodNodePose;
use crate::scene::host::LodLevelSpawnRequest;
use crate::scene::refresh::LodSceneRefreshChunkPlugin;
use crate::scene::{LodHostBounds, LodSceneHost, LodViewer};

use super::{GenerationPlugin, HcsgBounds, HcsgDemand, HcsgNode, HcsgStorage, PresentationPlugin};

const IDLE: Duration = Duration::from_secs(10);

#[derive(Resource, Default)]
struct Window {
	inner: Option<Aabb3d>,
	outer: Option<Aabb3d>,
}

struct WindowBounds;

impl HcsgBounds for WindowBounds {
	type Param = Res<'static, Window>;

	fn inner(window: &SystemParamItem<Self::Param>) -> Option<Aabb3d> {
		window.inner
	}

	fn outer(window: &SystemParamItem<Self::Param>) -> Option<Aabb3d> {
		window.outer
	}
}

fn span(x: f32, width: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + width, 1.0, 1.0))
}

fn ids(xs: &[f32]) -> Vec<Id> {
	let mut ids: Vec<Id> = xs.iter().map(|&x| Id::from_cell(cell(x))).collect();
	ids.sort();
	ids
}

fn app(plugin: impl Plugin, inner: Aabb3d, outer: Aabb3d) -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.add_plugins((AssetPlugin::default(), ScenePlugin))
		.insert_resource(Window { inner: Some(inner), outer: Some(outer) })
		.add_plugins(plugin);
	app
}

fn present_terrain(inner: Aabb3d, outer: Aabb3d) -> App {
	app(PresentationPlugin::<WindowBounds, Terrain>::default(), inner, outer)
}

fn set_window(app: &mut App, inner: Option<Aabb3d>, outer: Option<Aabb3d>) {
	*app.world_mut().resource_mut::<Window>() = Window { inner, outer };
}

/// Subscribe (or notice a dropped subscription and resubscribe), let the
/// worker finish, then reconcile hosts.
fn settle(app: &mut App) -> anyhow::Result<()> {
	for _ in 0..2 {
		app.update();
		let demand = app.world().resource::<HcsgDemand>().clone();
		anyhow::ensure!(demand.wait_idle(IDLE), "worker did not go idle");
	}
	app.update();
	Ok(())
}

fn hosts(app: &mut App) -> Vec<(Id, Version, Entity)> {
	let mut hosts: Vec<_> = app
		.world_mut()
		.query::<(Entity, &HcsgNode<Terrain>)>()
		.iter(app.world())
		.map(|(entity, node)| (node.id, node.version, entity))
		.collect();
	hosts.sort();
	hosts
}

fn hosted_ids(app: &mut App) -> Vec<Id> {
	hosts(app).into_iter().map(|(id, _, _)| id).collect()
}

#[test]
fn presentation_spawns_one_host_per_published_value() -> anyhow::Result<()> {
	let mut app = present_terrain(span(0.2, 2.6), span(-10.0, 30.0));
	settle(&mut app)?;
	assert_eq!(hosted_ids(&mut app), ids(&[0.0, 1.0, 2.0]));

	for (id, _, entity) in hosts(&mut app) {
		let bounds = app.world().get::<LodHostBounds>(entity).map(|bounds| bounds.0);
		assert_eq!(bounds, id.origin_cell_bounds(), "hosts carry their stored world bounds");
	}
	Ok(())
}

#[test]
fn presentation_keeps_hosts_until_they_leave_the_outer_bounds() -> anyhow::Result<()> {
	let mut app = present_terrain(span(0.2, 2.6), span(-10.0, 30.0));
	settle(&mut app)?;

	set_window(&mut app, Some(span(1.2, 2.6)), Some(span(-10.0, 30.0)));
	settle(&mut app)?;
	assert_eq!(hosted_ids(&mut app), ids(&[0.0, 1.0, 2.0, 3.0]), "no host twice; none retired");

	set_window(&mut app, Some(span(1.2, 2.6)), Some(span(0.5, 2.0)));
	settle(&mut app)?;
	assert_eq!(hosted_ids(&mut app), ids(&[0.0, 1.0, 2.0]));

	let storage = app.world().resource::<HcsgStorage>().clone();
	assert!(storage.contains::<Terrain>(Id::from_cell(cell(3.0))), "retiring never evicts");
	Ok(())
}

#[test]
fn presentation_without_bounds_unsubscribes_and_retires() -> anyhow::Result<()> {
	let mut app = present_terrain(span(0.2, 2.6), span(-10.0, 30.0));
	settle(&mut app)?;
	set_window(&mut app, None, None);
	settle(&mut app)?;
	assert!(hosts(&mut app).is_empty());
	Ok(())
}

#[test]
fn presentation_replaces_hosts_whose_value_changed_across_an_epoch() -> anyhow::Result<()> {
	let mut app = present_terrain(span(0.2, 2.6), span(-10.0, 30.0));
	settle(&mut app)?;
	let before = hosts(&mut app);

	app.world().resource::<HcsgDemand>().advance_epoch();
	app.world().resource::<HcsgStorage>().clear::<Terrain>();
	settle(&mut app)?;

	let after = hosts(&mut app);
	assert_eq!(after.len(), before.len());
	for ((id, version, entity), (before_id, before_version, before_entity)) in
		after.iter().zip(&before)
	{
		assert_eq!(id, before_id);
		assert!(version > before_version);
		assert_ne!(entity, before_entity);
		assert!(app.world().get_entity(*before_entity).is_err(), "stale host despawned");
	}
	Ok(())
}

#[test]
fn presented_hosts_fulfill_through_the_lod_pipeline() -> anyhow::Result<()> {
	let mut app = present_terrain(span(0.2, 2.6), span(-10.0, 30.0));
	app.add_plugins(LodSceneRefreshChunkPlugin::<HcsgNode<Terrain>>::default());
	let at = Transform::from_translation(Vec3::new(1.0, 0.0, 0.0));
	app.world_mut()
		.spawn((LodViewer, at, LodNodePose { previous: at, current: at }));
	settle(&mut app)?;
	for _ in 0..4 {
		app.update();
	}

	let hosts = hosts(&mut app);
	assert_eq!(hosts.len(), 3);
	for (_, _, host) in hosts {
		assert!(app.world().get::<LodSceneHost>(host).is_some());
		assert!(
			app.world().get::<LodLevelSpawnRequest>(host).is_none(),
			"the forwarded scene was fulfilled"
		);
	}
	Ok(())
}

#[test]
fn generation_keeps_values_warm_without_hosts() -> anyhow::Result<()> {
	let mut app = app(
		GenerationPlugin::<WindowBounds, Vegetation>::default(),
		span(0.2, 2.6),
		span(-10.0, 30.0),
	);
	settle(&mut app)?;
	let storage = app.world().resource::<HcsgStorage>().clone();
	for id in ids(&[0.0, 1.0, 2.0]) {
		assert!(storage.contains::<Vegetation>(id) && storage.contains::<Terrain>(id));
	}
	assert!(app
		.world_mut()
		.query::<&HcsgNode<Vegetation>>()
		.iter(app.world())
		.next()
		.is_none());

	app.world().resource::<HcsgDemand>().advance_epoch();
	storage.clear::<Vegetation>();
	settle(&mut app)?;
	assert!(
		storage.contains::<Vegetation>(Id::from_cell(cell(1.0))),
		"resubscribed after the epoch"
	);
	Ok(())
}
