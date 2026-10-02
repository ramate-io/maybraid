use bevy::app::{App, Plugin};
use bevy::ecs::system::{Res, SystemParamItem, SystemState};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{Resource, World};
use layer_stack::{GenerationMode, RequireLayer};
use lod::lod_ref::LodRef;
use terrain_layer_model::{
	terrain_streaming, BaseTerrainGenerationCore, HeightField, TerrainCell, TerrainContract,
	TerrainExtent, TerrainGeneration, TerrainModel, TerrainStreaming, TerrainView,
};

use crate::{
	Vegetation, VegetationGeneration, VegetationGenerationCore, VegetationGenerationPlugin,
	VegetationModel, VegetationScheme,
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

struct StubVeg;

impl VegetationModel for StubVeg {
	type Ground = StubGround;

	fn require_generation(app: &App) {
		app.require_layer::<VegetationGenerationCore<Self>, Vegetation<Self>>();
	}
}

impl VegetationGeneration for StubVeg {
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
	fn clear_generation(_world: &mut World) {}
}

impl VegetationScheme<StubVeg> for TestMode {
	fn install(_app: &mut App, _config: &()) {}
}

type Stacked = Vegetation<StubVeg>;

fn empty_world() -> World {
	let mut world = World::new();
	world.init_resource::<GroundStore>();
	world
}

#[test]
fn wrapper_heights_match_the_ground() -> anyhow::Result<()> {
	let mut world = empty_world();
	world.resource_mut::<GroundStore>().fallback = 7.0;
	world.resource_mut::<GroundStore>().cell = Some(StubRaw {
		bounds: Aabb3d::from_min_max(Vec3::ZERO, Vec3::splat(40.0)),
	});
	let xz = Vec2::new(2.0, 3.0);
	let stacked_height = {
		let mut stacked = SystemState::<TerrainView<Stacked>>::new(&mut world);
		let view = stacked.get(&world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		(
			Stacked::height_at(&view.read, xz),
			Stacked::fallback_height_at(&view.read, xz),
		)
	};
	let ground_height = {
		let mut ground = SystemState::<TerrainView<StubGround>>::new(&mut world);
		let view = ground.get(&world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		(
			StubGround::height_at(&view.read, xz),
			StubGround::fallback_height_at(&view.read, xz),
		)
	};
	anyhow::ensure!(stacked_height == ground_height);
	Ok(())
}

#[test]
fn overlays_pass_through_the_ground_cell() -> anyhow::Result<()> {
	let mut world = empty_world();
	world.resource_mut::<GroundStore>().cell = Some(StubRaw {
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
	.ok_or_else(|| anyhow::anyhow!("ground overlay"))?;
	anyhow::ensure!(cell.res_2() == 0, "ground cell is unchanged");
	Ok(())
}

#[test]
#[should_panic(expected = "requires terrain_layer_model::generation::BaseTerrainGenerationCore")]
fn requirements_recurse_to_the_ground() {
	Stacked::require_generation(&App::new());
}

#[test]
#[should_panic(expected = "requires vegetation_layer_model::generation::VegetationGenerationCore")]
fn requirements_name_the_vegetation_core() {
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
fn vegetation_generation_without_ground_names_the_missing_plugin() {
	let result = std::panic::catch_unwind(|| {
		VegetationGenerationPlugin::<TestMode, StubVeg>::default().finish(&mut App::new());
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
