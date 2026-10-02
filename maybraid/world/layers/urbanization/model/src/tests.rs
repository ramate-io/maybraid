use bevy::app::{App, Plugin};
use bevy::ecs::system::{Res, SystemParamItem, SystemState};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{Resource, World};
use layer_stack::{GenerationMode, RequireLayer};
use lod::gen::Id;
use lod::lod_ref::LodRef;
use terrain_layer_model::{
	terrain_streaming, BaseTerrainGenerationCore, HeightField, TerrainCell, TerrainContract,
	TerrainExtent, TerrainGeneration, TerrainModel, TerrainStreaming, TerrainView,
};

use crate::{
	PadOps, UrbanSnapshot, Urbanization, UrbanizationGeneration, UrbanizationGenerationCore,
	UrbanizationGenerationPlugin, UrbanizationModel, UrbanizationScheme,
};

struct TestMode;

impl GenerationMode for TestMode {}

#[derive(Resource, Default)]
struct GroundStore {
	fallback: f32,
	cell: Option<StubRaw>,
}

#[derive(Clone, Copy)]
struct StubRaw {
	bounds: Aabb3d,
}

impl TerrainCell for StubRaw {
	type Mesh = ();
	fn bounds(&self) -> Aabb3d {
		self.bounds
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

#[derive(Clone, Copy)]
struct StubPadded {
	bounds: Aabb3d,
}

impl TerrainCell for StubPadded {
	type Mesh = ();
	fn bounds(&self) -> Aabb3d {
		self.bounds
	}
	fn mesh_builder(&self) {}
	fn chunk_pose(&self) -> bevy::prelude::Transform {
		bevy::prelude::Transform::IDENTITY
	}
	fn seeds_collision(&self) -> bool {
		true
	}
	fn res_2(&self) -> u8 {
		3
	}
}

#[derive(Clone)]
struct GroundField(f32);

impl HeightField for GroundField {
	fn height_at(&self, _xz: Vec2) -> Option<f32> {
		Some(self.0)
	}
	fn fallback_height_at(&self, _xz: Vec2) -> f32 {
		self.0
	}
}

struct StubGround;

impl TerrainModel for StubGround {
	type Base = Self;
	type Cell = StubRaw;
	type Read = Res<'static, GroundStore>;
	type Snapshot = GroundField;
	type Prepare = ();

	fn prepare(
		_prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
	}

	fn height_at(read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> Option<f32> {
		read.cell.map(|_| read.fallback)
	}

	fn fallback_height_at(read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> f32 {
		read.fallback
	}

	fn cell_ids_overlapping(
		_read: &SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<Id> {
		Vec::new()
	}

	fn cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		_id: Id,
	) -> Option<&'a StubRaw> {
		read.cell.as_ref()
	}

	fn overlay_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		_bounds: Aabb3d,
		_target_size: f32,
		_overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = ()>> {
		read.cell.as_ref().map(|cell| cell as &dyn TerrainCell<Mesh = ()>)
	}

	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> GroundField {
		GroundField(read.fallback)
	}

	fn require_generation(app: &App) {
		app.require_layer::<BaseTerrainGenerationCore<Self>, Self>();
	}
}

impl TerrainGeneration for StubGround {
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
}

#[derive(Clone, Copy)]
struct StubPads {
	delta: f32,
}

impl PadOps for StubPads {
	fn modify_elevation(&self, height: f32, _x: f32, _z: f32) -> f32 {
		height + self.delta
	}
}

#[derive(Resource, Default)]
struct UrbanStore {
	pads: StubPads,
	padded: Option<StubPadded>,
}

impl Default for StubPads {
	fn default() -> Self {
		Self { delta: 0.0 }
	}
}

struct StubLeaf {
	bounds: Aabb3d,
}

struct StubDev {
	bounds: Aabb3d,
}

struct StubUrban;

impl UrbanizationModel for StubUrban {
	type Ground = StubGround;
	type Leaf = StubLeaf;
	type Cell = StubDev;
	type Built = ();
	type Pads = StubPads;
	type Kind = ();
	type Selection = ();
	type Surface = StubPadded;
	type Read = Res<'static, UrbanStore>;
	type Select = ();
	type Prepare = ();

	fn pads(read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> StubPads {
		read.pads
	}

	fn pads_at(read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> StubPads {
		read.pads
	}

	fn surface<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		_id: Id,
	) -> Option<&'a StubPadded> {
		read.padded.as_ref()
	}

	fn surface_ids(read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> Vec<Id> {
		if read.padded.is_some() {
			vec![Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE))]
		} else {
			Vec::new()
		}
	}

	fn overlay_surface<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		_bounds: Aabb3d,
	) -> Option<&'a StubPadded> {
		read.padded.as_ref()
	}

	fn urbanization_leaves<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<&'a StubLeaf> {
		Vec::new()
	}

	fn development_cells<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<&'a StubDev> {
		Vec::new()
	}

	fn built<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<&'a ()> {
		Vec::new()
	}

	fn built_overlapping<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<(Id, lod::gen::Version, &'a ())> {
		Vec::new()
	}

	fn development_cell<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_id: Id,
	) -> Option<&'a StubDev> {
		None
	}

	fn urbanization_selection(
		_read: &SystemParamItem<'_, '_, Self::Read>,
	) -> ((), Option<()>) {
		((), None)
	}

	fn leaf_bounds(leaf: &StubLeaf) -> Aabb3d {
		leaf.bounds
	}

	fn cell_bounds(cell: &StubDev) -> Aabb3d {
		cell.bounds
	}

	fn ensure_selected(
		_select: &mut SystemParamItem<'_, '_, Self::Select>,
		_region: Aabb3d,
	) {
	}

	fn prepare(
		_prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
	}

	fn require_generation(app: &App) {
		app.require_layer::<UrbanizationGenerationCore<Self>, Urbanization<Self>>();
	}
}

impl UrbanizationGeneration for StubUrban {
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
	fn clear_generation(_world: &mut World) {}
}

impl UrbanizationScheme<StubUrban> for TestMode {
	fn install(_app: &mut App, _config: &()) {}
}

type Stacked = Urbanization<StubUrban>;

fn empty_world() -> World {
	let mut world = World::new();
	world.init_resource::<GroundStore>();
	world.init_resource::<UrbanStore>();
	world
}

#[test]
fn stub_pads_apply_to_heights() -> anyhow::Result<()> {
	let snapshot = UrbanSnapshot::new(GroundField(3.0), StubPads { delta: 9.0 });
	let terrace = snapshot
		.height_at(Vec2::ZERO)
		.ok_or_else(|| anyhow::anyhow!("terrace"))?;
	anyhow::ensure!((terrace - 12.0).abs() < 1e-5);
	Ok(())
}

#[test]
fn overlays_prefer_stub_padded_cells() -> anyhow::Result<()> {
	let mut world = empty_world();
	world.resource_mut::<GroundStore>().fallback = 1.0;
	world.resource_mut::<GroundStore>().cell = Some(StubRaw {
		bounds: Aabb3d::from_min_max(Vec3::ZERO, Vec3::splat(40.0)),
	});
	world.resource_mut::<UrbanStore>().padded = Some(StubPadded {
		bounds: Aabb3d::from_min_max(Vec3::ZERO, Vec3::splat(40.0)),
	});
	let mut state = SystemState::<TerrainView<Stacked>>::new(&mut world);
	let view = state.get(&world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
	let cell = Stacked::overlay_cell(
		&view.read,
		Aabb3d::from_min_max(Vec3::ZERO, Vec3::splat(40.0)),
		40.0,
		None,
	)
	.ok_or_else(|| anyhow::anyhow!("padded overlay"))?;
	anyhow::ensure!(cell.res_2() == 3, "padded cell wins");
	Ok(())
}

#[test]
#[should_panic(expected = "requires terrain_layer_model::generation::BaseTerrainGenerationCore")]
fn requirements_recurse_to_the_ground() {
	Stacked::require_generation(&App::new());
}

#[test]
#[should_panic(expected = "requires urbanization_layer_model::generation::UrbanizationGenerationCore")]
fn requirements_name_the_urbanization_core() {
	let mut app = App::new();
	app.add_plugins(BaseTerrainGenerationCore::<StubGround>::default());
	Stacked::require_generation(&app);
}

#[test]
fn wrappers_read_the_ground_contract_the_same_update() -> anyhow::Result<()> {
	use bevy::prelude::{IntoScheduleConfigs, MinimalPlugins, ResMut, Update};

	#[derive(Resource, Default)]
	struct Seen(Option<(bool, Aabb3d)>);

	fn note_contract(contract: TerrainContract<Stacked>, mut seen: ResMut<Seen>) {
		seen.0 = Some((contract.streaming.enabled, contract.extent.presentation_region()));
	}

	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.insert_resource(TerrainStreaming::<StubGround>::new(false));
	let first = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
	app.insert_resource(TerrainExtent::<StubGround>::pinned(first));
	app.init_resource::<Seen>();
	app.add_systems(Update, note_contract.run_if(terrain_streaming::<Stacked>));

	app.update();
	anyhow::ensure!(
		app.world().resource::<Seen>().0.is_none(),
		"streaming off skips the reader"
	);

	app.world_mut().resource_mut::<TerrainStreaming<StubGround>>().enabled = true;
	let next = Aabb3d::from_min_max(Vec3::splat(-4.0), Vec3::splat(4.0));
	*app.world_mut().resource_mut::<TerrainExtent<StubGround>>() = TerrainExtent::streamed(next);
	app.update();
	let seen = app.world().resource::<Seen>().0;
	anyhow::ensure!(seen == Some((true, next)), "wrapper sees the ground write: {seen:?}");
	Ok(())
}

#[test]
fn empty_keep_draws_a_fine_patch_from_the_layout() -> anyhow::Result<()> {
	use crate::{urbanization_host_region, urbanization_visual_region};

	struct Stub;

	let region = Aabb3d::from_min_max(Vec3::new(-320.0, -80.0, -320.0), Vec3::new(320.0, 80.0, 320.0));
	let extent = TerrainExtent::<Stub>::pinned(region);
	let visual = urbanization_visual_region(&extent, None)
		.ok_or_else(|| anyhow::anyhow!("visual region"))?;
	let host = urbanization_host_region(&extent, None)
		.ok_or_else(|| anyhow::anyhow!("host region"))?;
	anyhow::ensure!(visual == region);
	anyhow::ensure!(host == region);

	let ring = Aabb3d::from_min_max(Vec3::new(-1_000.0, -80.0, -1_000.0), Vec3::new(1_000.0, 80.0, 1_000.0));
	let streamed = TerrainExtent::<Stub>::streamed(ring);
	let layer = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
	anyhow::ensure!(
		urbanization_visual_region(&streamed, Some(layer)) == Some(ring),
		"streamed visual is the presentation ring"
	);
	anyhow::ensure!(
		urbanization_host_region(&streamed, None).is_none(),
		"streamed host waits for the scheme write"
	);
	Ok(())
}

#[test]
fn urbanization_generation_without_ground_names_the_missing_plugin() {
	let result = std::panic::catch_unwind(|| {
		UrbanizationGenerationPlugin::<TestMode, StubUrban>::default().finish(&mut App::new());
	});
	let message = match result {
		Ok(()) => "plugin finish returned".to_string(),
		Err(payload) => payload
			.downcast_ref::<String>()
			.cloned()
			.or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
			.unwrap_or_else(|| "non-string panic".to_string()),
	};
	assert!(
		message.contains("BaseTerrainGenerationCore"),
		"finish names the missing ground core, got {message}"
	);
}
