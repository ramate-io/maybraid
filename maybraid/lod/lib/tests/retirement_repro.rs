use bevy::camera::visibility::VisibilityPlugin;
use bevy::ecs::entity_disabling::Disabled;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::scene::{ScenePlugin, WorldSceneExt};
use bevy::world_serialization::{WorldAsset, WorldAssetRoot, WorldSerializationPlugin};
use lod::lod_ref::LodRef;
use lod::scene::LodScene;
use lod::{
	add_lod_refresh_chunk_full_for, lod_host_scene_pending, LodLevelRoot, LodLevelRoots,
	LodRefreshCorePlugin, LodSceneLevel, LodViewer,
};

#[derive(Component, Clone)]
struct RenderableProbe;
#[derive(Component, Clone, Copy, Debug, Default)]
struct BandMesh(LodSceneLevel);
impl LodScene for RenderableProbe {
	fn scene_lod_level(&self, r: &LodRef) -> LodSceneLevel {
		let d = r.current_transform.translation.x.abs();
		if d < 80.0 {
			LodSceneLevel::High
		} else if d < 220.0 {
			LodSceneLevel::Medium
		} else {
			LodSceneLevel::Low
		}
	}
	fn scene_with_level(
		&self,
		_: &LodRef,
		level: LodSceneLevel,
	) -> impl bevy::scene::Scene + 'static {
		use bevy::scene::prelude::bsn;
		bsn! {
			~{BandMesh(level)}
			Mesh3d::default()
			MeshMaterial3d::<StandardMaterial>::default()
			Transform::default()
			Visibility::Inherited
		}
	}
}
fn app() -> anyhow::Result<(App, Entity, Entity)> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		AssetPlugin::default(),
		MeshPlugin,
		ScenePlugin,
		VisibilityPlugin,
		WorldSerializationPlugin,
		LodRefreshCorePlugin,
	));
	add_lod_refresh_chunk_full_for::<RenderableProbe>(&mut app);
	let viewer = app.world_mut().spawn((LodViewer, Transform::IDENTITY)).id();
	let host = app.world_mut().spawn_scene(lod_host_scene_pending(
		LodSceneLevel::High,
		bevy::shape::Aabb3d::from_min_max(Vec3::splat(-5.0), Vec3::splat(5.0)),
	))?;
	let host = host.id();
	app.world_mut().entity_mut(host).insert(RenderableProbe);
	Ok((app, viewer, host))
}
#[test]
fn renderable_walk_cycle_only_current_band_enabled() -> anyhow::Result<()> {
	let (mut app, viewer, host) = app()?;
	for x in [0.0, 120.0, 400.0, 120.0, 0.0, 400.0, 0.0] {
		app.world_mut().entity_mut(viewer).insert(Transform::from_xyz(x, 0.0, 0.0));
		for _ in 0..15 {
			app.update();
		}
		let desired = *app
			.world()
			.get::<LodSceneLevel>(host)
			.ok_or_else(|| anyhow::anyhow!("missing host"))?;
		let mut q = app.world_mut().query::<(&BandMesh, &InheritedVisibility)>();
		let shown: Vec<_> =
			q.iter(app.world()).filter_map(|(band, v)| v.get().then_some(band.0)).collect();
		assert_eq!(shown, vec![desired], "x={x}");
		let world = app.world();
		let host_kids =
			world.get::<Children>(host).ok_or_else(|| anyhow::anyhow!("missing children"))?;
		let bag = host_kids
			.iter()
			.find(|e| world.get::<LodLevelRoots>(*e).is_some())
			.ok_or_else(|| anyhow::anyhow!("missing bag"))?;
		for root in world.get::<Children>(bag).into_iter().flat_map(|c| c.iter()) {
			let Some(band) = world.get::<LodLevelRoot>(root) else { continue };
			assert_eq!(world.get::<Disabled>(root).is_none(), band.0 == desired);
		}
	}
	Ok(())
}

#[test]
fn late_world_asset_cannot_reveal_a_retired_band() -> anyhow::Result<()> {
	use lod::scene::host::hide_lod_tree;
	let (mut app, _, _) = app()?;
	let delayed = app.world_mut().resource_mut::<Assets<WorldAsset>>().reserve_handle();
	let root = app
		.world_mut()
		.spawn((Visibility::Inherited, Transform::default(), LodLevelRoot(LodSceneLevel::Medium)))
		.id();
	let asset_root = app
		.world_mut()
		.spawn((
			WorldAssetRoot(delayed.clone()),
			Visibility::Inherited,
			Transform::default(),
			ChildOf(root),
		))
		.id();
	for _ in 0..3 {
		app.update();
	}
	{
		let mut c = app.world_mut().commands();
		hide_lod_tree(&mut c, root);
	}
	for _ in 0..3 {
		app.update();
	}
	assert!(app.world().get::<Disabled>(asset_root).is_some());
	let mut content = World::new();
	content.spawn((
		Name::new("late-mid-wall"),
		Mesh3d::default(),
		Transform::default(),
		Visibility::Inherited,
	));
	app.world_mut()
		.resource_mut::<Assets<WorldAsset>>()
		.insert(delayed.id(), WorldAsset::new(content))?;
	for _ in 0..12 {
		app.update();
	}
	let mut q = app.world_mut().query::<(Entity, &Name, &InheritedVisibility, Has<Disabled>)>();
	let late: Vec<_> = q
		.iter(app.world())
		.filter(|(_, name, _, _)| name.as_str() == "late-mid-wall")
		.map(|(e, _, v, d)| (e, v.get(), d))
		.collect();
	assert_eq!(late.len(), 1, "WorldAsset really spawned late");
	assert!(!late[0].1 || late[0].2, "late wall under retired Mid root must not render: {late:?}");
	Ok(())
}

#[test]
fn modified_world_asset_stays_hidden_and_reveals_with_its_original_band() -> anyhow::Result<()> {
	use lod::scene::host::{hide_lod_tree, show_lod_tree};
	let (mut app, _, _) = app()?;
	let mut initial = World::new();
	initial.spawn((
		Name::new("initial-wall"),
		Mesh3d::default(),
		Transform::from_xyz(3.0, 0.0, 0.0),
		Visibility::Inherited,
	));
	let handle = app
		.world_mut()
		.resource_mut::<Assets<WorldAsset>>()
		.add(WorldAsset::new(initial));
	let root = app
		.world_mut()
		.spawn((
			Visibility::Inherited,
			Transform::from_xyz(2000.0, 0.0, 0.0),
			LodLevelRoot(LodSceneLevel::Medium),
		))
		.id();
	app.world_mut().spawn((
		WorldAssetRoot(handle.clone()),
		Visibility::Inherited,
		Transform::default(),
		ChildOf(root),
	));
	for _ in 0..5 {
		app.update();
	}
	let mut names = app.world_mut().query::<&Name>();
	assert_eq!(names.iter(app.world()).filter(|name| name.as_str() == "initial-wall").count(), 1);
	{
		let mut commands = app.world_mut().commands();
		hide_lod_tree(&mut commands, root);
	}
	for _ in 0..3 {
		app.update();
	}
	app.world_mut().entity_mut(root).insert(Transform::from_xyz(2010.0, 0.0, 0.0));
	let mut replacement = World::new();
	replacement.spawn((
		Name::new("replacement-wall"),
		Mesh3d::default(),
		Transform::from_xyz(3.0, 0.0, 0.0),
		Visibility::Inherited,
	));
	app.world_mut()
		.resource_mut::<Assets<WorldAsset>>()
		.insert(handle.id(), WorldAsset::new(replacement))?;
	for _ in 0..12 {
		app.update();
	}
	let mut names = app.world_mut().query::<(Entity, &Name, Has<Disabled>)>();
	let replacement: Vec<_> = names
		.iter(app.world())
		.filter(|(_, name, _)| name.as_str() == "replacement-wall")
		.collect();
	assert_eq!(replacement.len(), 1, "modified WorldAsset must respawn its instance");
	assert!(replacement[0].2, "modified instance under retired band must be disabled");
	{
		let mut commands = app.world_mut().commands();
		show_lod_tree(&mut commands, root);
	}
	for _ in 0..4 {
		app.update();
	}
	let mut q = app
		.world_mut()
		.query::<(&Name, &InheritedVisibility, Has<Disabled>, &GlobalTransform)>();
	let shown: Vec<_> = q
		.iter(app.world())
		.filter(|(name, _, _, _)| name.as_str() == "replacement-wall")
		.collect();
	assert_eq!(shown.len(), 1);
	assert!(!shown[0].2, "revealing the original band left the respawned mesh disabled");
	assert!(shown[0].1.get(), "revealing the original band left inherited visibility false");
	assert!(
		(shown[0].3.translation().x - 2013.0).abs() < 1e-3,
		"revealing the original band must refresh the late child's world transform"
	);
	Ok(())
}

#[test]
fn nested_hidden_root_owns_late_instance_until_its_own_reveal() -> anyhow::Result<()> {
	use lod::scene::host::{hide_lod_tree, show_lod_tree};
	let (mut app, _, _) = app()?;
	let outer = app.world_mut().spawn((Visibility::Inherited, Transform::default())).id();
	let nested = app
		.world_mut()
		.spawn((Visibility::Inherited, Transform::default(), ChildOf(outer)))
		.id();
	{
		let mut commands = app.world_mut().commands();
		hide_lod_tree(&mut commands, nested);
		hide_lod_tree(&mut commands, outer);
	}
	for _ in 0..3 {
		app.update();
	}
	let delayed = app.world_mut().resource_mut::<Assets<WorldAsset>>().reserve_handle();
	let asset_root = app
		.world_mut()
		.spawn((
			WorldAssetRoot(delayed.clone()),
			Visibility::Inherited,
			Transform::default(),
			ChildOf(nested),
		))
		.id();
	for _ in 0..3 {
		app.update();
	}
	let mut content = World::new();
	content.spawn((
		Name::new("nested-late-wall"),
		Mesh3d::default(),
		Transform::default(),
		Visibility::Inherited,
	));
	app.world_mut()
		.resource_mut::<Assets<WorldAsset>>()
		.insert(delayed.id(), WorldAsset::new(content))?;
	for _ in 0..12 {
		app.update();
	}
	assert!(app.world().get::<Disabled>(asset_root).is_some());
	{
		let mut commands = app.world_mut().commands();
		show_lod_tree(&mut commands, outer);
	}
	for _ in 0..4 {
		app.update();
	}
	let mut q = app.world_mut().query::<(&Name, Has<Disabled>)>();
	let hidden: Vec<_> = q
		.iter(app.world())
		.filter(|(name, _)| name.as_str() == "nested-late-wall")
		.collect();
	assert_eq!(hidden.len(), 1);
	assert!(hidden[0].1, "revealing the outer owner must preserve the independent nested hide");
	{
		let mut commands = app.world_mut().commands();
		show_lod_tree(&mut commands, nested);
	}
	for _ in 0..4 {
		app.update();
	}
	let mut q = app.world_mut().query::<(&Name, &InheritedVisibility, Has<Disabled>)>();
	let shown: Vec<_> = q
		.iter(app.world())
		.filter(|(name, _, _)| name.as_str() == "nested-late-wall")
		.collect();
	assert_eq!(shown.len(), 1);
	assert!(!shown[0].2, "the nested owner must reveal its late instance");
	assert!(shown[0].1.get(), "the nested owner must refresh inherited visibility");
	Ok(())
}

#[test]
fn late_instance_does_not_drop_pending_hide_work_and_hide_show_hide_converges() -> anyhow::Result<()>
{
	use lod::scene::host::{hide_lod_tree, show_lod_tree, LodTreeVisBudget};
	let (mut app, _, _) = app()?;
	app.world_mut().resource_mut::<LodTreeVisBudget>().entities_per_frame = 1;
	let root = app.world_mut().spawn((Visibility::Inherited, Transform::default())).id();
	let mut original = Vec::new();
	for _ in 0..64 {
		original.push(
			app.world_mut()
				.spawn((Visibility::Inherited, Transform::default(), ChildOf(root)))
				.id(),
		);
	}
	{
		let mut commands = app.world_mut().commands();
		hide_lod_tree(&mut commands, root);
	}
	app.update();
	let delayed = app.world_mut().resource_mut::<Assets<WorldAsset>>().reserve_handle();
	app.world_mut().spawn((
		WorldAssetRoot(delayed.clone()),
		Visibility::Inherited,
		Transform::default(),
		ChildOf(root),
	));
	for _ in 0..3 {
		app.update();
	}
	let mut content = World::new();
	content.spawn((
		Name::new("queued-late-wall"),
		Mesh3d::default(),
		Transform::default(),
		Visibility::Inherited,
	));
	app.world_mut()
		.resource_mut::<Assets<WorldAsset>>()
		.insert(delayed.id(), WorldAsset::new(content))?;
	for _ in 0..12 {
		app.update();
	}
	let mut q = app.world_mut().query::<(&Name, Has<Disabled>)>();
	let hidden: Vec<_> = q
		.iter(app.world())
		.filter(|(name, _)| name.as_str() == "queued-late-wall")
		.collect();
	assert_eq!(hidden.len(), 1);
	assert!(hidden[0].1, "late instance must be disabled while earlier hide work is pending");
	for _ in 0..80 {
		app.update();
	}
	assert!(
		original.iter().all(|entity| app.world().get::<Disabled>(*entity).is_some()),
		"original queued hide work must finish"
	);
	{
		let mut commands = app.world_mut().commands();
		show_lod_tree(&mut commands, root);
	}
	for _ in 0..80 {
		app.update();
	}
	assert!(
		original.iter().all(|entity| app.world().get::<Disabled>(*entity).is_none()),
		"show must re-enable the original descendants"
	);
	let mut q = app.world_mut().query::<(&Name, &InheritedVisibility, Has<Disabled>)>();
	let shown: Vec<_> = q
		.iter(app.world())
		.filter(|(name, _, _)| name.as_str() == "queued-late-wall")
		.collect();
	assert_eq!(shown.len(), 1);
	assert!(!shown[0].2, "show left late instance descendants disabled");
	assert!(shown[0].1.get(), "show left late instance inherited visibility false");
	{
		let mut commands = app.world_mut().commands();
		hide_lod_tree(&mut commands, root);
	}
	for _ in 0..80 {
		app.update();
	}
	assert!(
		original.iter().all(|entity| app.world().get::<Disabled>(*entity).is_some()),
		"second hide must converge"
	);
	let mut q = app.world_mut().query::<(&Name, Has<Disabled>)>();
	let hidden: Vec<_> = q
		.iter(app.world())
		.filter(|(name, _)| name.as_str() == "queued-late-wall")
		.collect();
	assert_eq!(hidden.len(), 1);
	assert!(hidden[0].1, "second hide must disable late instance descendants");
	Ok(())
}
