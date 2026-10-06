use bevy::app::{App, Plugin};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::ecs::system::{ResMut, RunSystemOnce, SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{
	AssetPlugin, Commands, Component, MinimalPlugins, NextState, PostUpdate, Query, Resource,
	Update, With, World,
};
use bevy::state::app::StatesPlugin;
use layer_stack::{
	subscribe_mode, ActiveGenerationMode, Generate, GenerationMode, GenerationModePlugin,
	LayerGenerationCore, LayerPresentationCore, ModeSubscribers, ModeSubscription, Present,
	RequireLayer, Scheme,
};
use lod::gen::{Id, LodGenerated, Version};
use lod::lod_ref::LodRef;
use mob_intelligence::MemberOf;
use mob_layer_model::{MobCellPresented, MobGeneration, MobModel, Mobs};
use terrain_layer_model::{HeightField, OnTerrain, TerrainCell, TerrainGeneration, TerrainModel};

use crate::present::MobPresenterState;

struct TestMode;

impl GenerationMode for TestMode {}

struct OtherMode;

impl GenerationMode for OtherMode {}

struct SilentGround;

#[derive(Clone)]
struct SilentField;

impl HeightField for SilentField {
	fn height_at(&self, _xz: Vec2) -> Option<f32> {
		None
	}
	fn fallback_height_at(&self, _xz: Vec2) -> f32 {
		0.0
	}
}

struct SilentCell;

impl TerrainCell for SilentCell {
	type Mesh = ();
	fn bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE)
	}
	fn mesh_builder(&self) {}
	fn chunk_pose(&self) -> bevy::prelude::Transform {
		bevy::prelude::Transform::IDENTITY
	}
	fn seeds_collision(&self) -> bool {
		false
	}
	fn res_2(&self) -> u8 {
		0
	}
}

impl TerrainModel for SilentGround {
	type Base = Self;
	type Cell = SilentCell;
	type Read = ();
	type Snapshot = SilentField;
	type Prepare = ();

	fn prepare(
		_prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
	}

	fn height_at(_read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> Option<f32> {
		None
	}

	fn fallback_height_at(_read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> f32 {
		0.0
	}

	fn overlay_cell<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_bounds: Aabb3d,
		_target_size: f32,
		_overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = ()>> {
		None
	}

	fn snapshot(_read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> SilentField {
		SilentField
	}

	fn require_generation(app: &App) {
		app.require_layer::<LayerGenerationCore<OnTerrain<Self>>, Self>();
	}
}

impl TerrainGeneration for SilentGround {
	const LABEL: &'static str = "silent-ground";
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
	fn install_presentation(_app: &mut App) {}
}

#[derive(Clone, Copy, Debug)]
struct StubCell {
	id: Id,
}

#[derive(Resource, Default)]
struct StubStore {
	cells: Vec<Id>,
}

#[derive(SystemParam)]
struct StubWrites<'w> {
	store: ResMut<'w, StubStore>,
	generated: MessageWriter<'w, LodGenerated<StubCell>>,
}

impl StubWrites<'_> {
	fn insert(&mut self, cell: StubCell) {
		self.store.cells.push(cell.id);
		self.generated.write(LodGenerated::new(cell.id));
	}
}

struct StubMob;

impl MobModel for StubMob {
	type Ground = SilentGround;
	type Cell = StubCell;
	type Writes = StubWrites<'static>;

	fn require_generation(app: &App) {
		app.require_layer::<LayerGenerationCore<Mobs<Self>>, Mobs<Self>>();
	}
}

impl MobGeneration for StubMob {
	const LABEL: &'static str = "stub";
	type Config = ();

	fn install_generation(app: &mut App) {
		app.init_resource::<StubStore>();
		app.add_message::<LodGenerated<StubCell>>();
	}

	fn apply_generation(_world: &mut World, _config: &()) {}

	fn clear_generation(world: &mut World) {
		world.resource_mut::<StubStore>().cells.clear();
	}

	fn install_presentation(app: &mut App) {
		app.init_resource::<MobPresenterState>();
		app.add_message::<MobCellPresented>();
		app.add_systems(Update, present_announced);
	}
}

impl Scheme<Mobs<StubMob>> for TestMode {
	fn install(_app: &mut App, _config: &()) {}
}

impl Scheme<Mobs<StubMob>> for OtherMode {
	fn install(_app: &mut App, _config: &()) {}
}

fn present_announced(
	mut announced: MessageReader<LodGenerated<StubCell>>,
	mut commands: Commands,
	mut state: ResMut<MobPresenterState>,
	mut presented: MessageWriter<MobCellPresented>,
) {
	for message in announced.read() {
		let host = commands.spawn_empty().id();
		state.remember(message.id, Version(1), vec![host]);
		presented.write(MobCellPresented { id: message.id, hosts: vec![host] });
	}
}

type Stacked = Mobs<StubMob>;

fn panic_message(result: Result<(), Box<dyn std::any::Any + Send>>) -> String {
	match result {
		Ok(()) => "plugin finish returned".to_string(),
		Err(payload) => payload
			.downcast_ref::<String>()
			.cloned()
			.or_else(|| payload.downcast_ref::<&str>().map(|text| (*text).to_string()))
			.unwrap_or_else(|| "non-string panic".to_string()),
	}
}

#[test]
fn presentation_without_generation_names_the_missing_ground() {
	let message = panic_message(std::panic::catch_unwind(|| {
		Present::<TestMode, Mobs<StubMob>>::default().finish(&mut App::new());
	}));
	assert!(
		message.contains("LayerGenerationCore"),
		"finish names the missing ground core, got {message}"
	);
}

#[test]
fn announced_writes_present_hosts() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		LayerGenerationCore::<OnTerrain<SilentGround>>::default(),
		Generate::<TestMode, Mobs<StubMob>>::default(),
		Present::<TestMode, Mobs<StubMob>>::default(),
	));
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	app.world_mut()
		.run_system_once(move |mut cells: StubWrites| cells.insert(StubCell { id }))
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	app.update();
	anyhow::ensure!(
		app.world().resource::<MobPresenterState>().presents(id),
		"the announced cell is presented"
	);
	let hosts = app
		.world_mut()
		.run_system_once(|mut reader: MessageReader<MobCellPresented>| {
			reader
				.read()
				.map(|message| (message.id, message.hosts.clone()))
				.collect::<Vec<_>>()
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(hosts.len() == 1 && hosts[0].0 == id, "presented-hosts hook names the cell");
	anyhow::ensure!(hosts[0].1.len() == 1, "the hook lists the spawned host");
	Ok(())
}

#[derive(Resource, Default)]
struct SquadSeenInPostUpdate(bool);

#[derive(Component)]
struct HostMark;

fn note_squad_before_last(
	hosts: Query<(), With<HostMark>>,
	members: Query<(), With<MemberOf>>,
	mut seen: ResMut<SquadSeenInPostUpdate>,
) {
	seen.0 = !hosts.is_empty() && !members.is_empty();
}

#[test]
fn teardown_retires_hosts_and_members_after_post_update() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, StatesPlugin));
	crate::install_mob_cell_teardown(&mut app);
	app.init_resource::<SquadSeenInPostUpdate>();
	app.add_systems(PostUpdate, note_squad_before_last);
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let host = app.world_mut().spawn(HostMark).id();
	let member = app.world_mut().spawn(MemberOf { mob: host, slot: 0 }).id();
	app.world_mut()
		.resource_mut::<MobPresenterState>()
		.remember(id, Version(1), vec![host]);
	app.update();
	anyhow::ensure!(
		app.world().get_entity(host).is_ok(),
		"a remembered host stays until it is queued"
	);

	app.world_mut().resource_mut::<MobPresenterState>().queue_remove(id);
	app.update();
	anyhow::ensure!(
		app.world().resource::<SquadSeenInPostUpdate>().0,
		"hosts and members survive PostUpdate on the retire frame"
	);
	anyhow::ensure!(app.world().get_entity(host).is_err(), "Last drain despawns the host");
	anyhow::ensure!(app.world().get_entity(member).is_err(), "Last drain despawns the member");
	Ok(())
}

#[test]
fn two_modes_install_the_core_once() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin));
	app.add_plugins((
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		Present::<TestMode, Mobs<StubMob>>::default(),
		Present::<OtherMode, Mobs<StubMob>>::default(),
	));
	anyhow::ensure!(app.is_plugin_added::<LayerPresentationCore<Mobs<StubMob>>>(), "core is installed");
	let subscribers = app.world().resource::<ModeSubscribers<Stacked>>();
	anyhow::ensure!(subscribers.contains::<TestMode>());
	anyhow::ensure!(subscribers.contains::<OtherMode>());
	Ok(())
}

#[test]
fn losing_subscription_is_inactive_and_returning_is_active() -> anyhow::Result<()> {
	use bevy::ecs::system::SystemState;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	subscribe_mode::<Stacked, TestMode>(&mut app);
	app.update();
	{
		let mut state =
			SystemState::<ModeSubscription<Stacked>>::new(app.world_mut());
		anyhow::ensure!(
			state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"subscribed mode presents"
		);
	}
	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	{
		let mut state =
			SystemState::<ModeSubscription<Stacked>>::new(app.world_mut());
		anyhow::ensure!(
			!state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"unsubscribed mode retires"
		);
	}
	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<TestMode>());
	app.update();
	{
		let mut state =
			SystemState::<ModeSubscription<Stacked>>::new(app.world_mut());
		anyhow::ensure!(
			state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"return presents again"
		);
	}
	Ok(())
}
