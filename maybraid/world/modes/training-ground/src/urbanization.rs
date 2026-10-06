//! Training's urbanization scheme: one authored, walled development.

use bevy::ecs::system::ParamSet;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use durham::{Durham, HcsgStorage, TerrainCellLayout, TerrainStorage};
use layer_stack::{ActiveGenerationMode, GenerationModeSystems};
use layer_stack::{LayerSystems, Scheme};
use lod::gen::Id;
use richmond::{
	AuthoredCourtyard, AuthoredDevelopment, AuthoredDevelopments, DevelopmentConfig,
	DevelopmentKind, Richmond, RichmondDevelopment, DEVELOPMENT_CELL_SIZE,
};
use terrain_layer_model::{OnTerrain, TerrainView};
use urbanization_layer_model::{Urbanization, UrbanizationLayerRegion};

use crate::{TrainingGround, TrainingMap, TrainingRound};

/// Courtyard band between the building footprint and the wall.
pub const TRAINING_ARENA_MARGIN_M: f32 = 16.0;
/// Keeps the courtyard plus its ease inside the 320 m FinePatch.
pub const TRAINING_ARENA_MAX_HALF_M: f32 = 128.0;
/// Flatten runs under the wall so its base never meets the ease slope.
pub const TRAINING_COURTYARD_OVERHANG_M: f32 = 3.0;
pub const TRAINING_COURTYARD_EASE_M: f32 = 24.0;

/// Pads are composed; the world reads this and never a type from the plaza.
#[derive(Resource, Debug)]
pub struct TrainingPlazaStamped {
	round: TrainingRound,
	cell_id: Id,
	terrain_ids: Vec<Id>,
	center: Vec2,
	half: Vec2,
	footprint: Vec2,
	plaza_y: f32,
}

impl TrainingPlazaStamped {
	pub fn new(
		round: TrainingRound,
		cell_id: Id,
		terrain_ids: Vec<Id>,
		center: Vec2,
		half: Vec2,
		footprint: Vec2,
		plaza_y: f32,
	) -> Self {
		Self { round, cell_id, terrain_ids, center, half, footprint, plaza_y }
	}

	pub fn round(&self) -> TrainingRound {
		self.round
	}

	pub fn cell_id(&self) -> Id {
		self.cell_id
	}

	pub fn terrain_ids(&self) -> &[Id] {
		&self.terrain_ids
	}

	pub fn center(&self) -> Vec2 {
		self.center
	}

	pub fn half(&self) -> Vec2 {
		self.half
	}

	pub fn footprint(&self) -> Vec2 {
		self.footprint
	}

	pub fn plaza_y(&self) -> f32 {
		self.plaza_y
	}

	pub fn fills_ready(&self, cooked: usize) -> bool {
		self.terrain_ids.is_empty() || cooked >= self.terrain_ids.len()
	}
}

/// Set when this map's site fitted no development.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingStampSettled(pub TrainingMap);

/// Training's development over the FinePatch ground.
pub type TrainingDevelopment = RichmondDevelopment<OnTerrain<Durham>>;

/// One flat walled courtyard around the building footprint.
pub(crate) const TRAINING_COURTYARD: AuthoredCourtyard = AuthoredCourtyard {
	margin: TRAINING_ARENA_MARGIN_M,
	max_half: TRAINING_ARENA_MAX_HALF_M,
	overhang: TRAINING_COURTYARD_OVERHANG_M,
	ease: TRAINING_COURTYARD_EASE_M,
};

impl Scheme<Urbanization<Richmond<OnTerrain<Durham>>>> for TrainingGround {
	fn install(app: &mut App, _config: &richmond::RichmondConfig) {
		app.add_systems(
			Update,
			stamp_training_urbanization
				.in_set(GenerationModeSystems::<TrainingGround>::default())
				.in_set(LayerSystems::<Urbanization<Richmond<OnTerrain<Durham>>>>::default()),
		);
		app.add_systems(OnExit(ActiveGenerationMode::of::<TrainingGround>()), clear_training_stamp);
	}
}

fn clear_training_stamp(mut authored: ResMut<AuthoredDevelopments>, mut commands: Commands) {
	authored.0.clear();
	commands.remove_resource::<TrainingPlazaStamped>();
	commands.remove_resource::<TrainingStampSettled>();
}

/// Author the round's development once the FinePatch is stored, then stamp
/// it once Richmond has generated it from that seed.
///
/// A new map drops the authored development and stamp here. Leaving Training
/// drops them on [`OnExit`], so a later enter on the same map stamps again.
/// A new life keeps the map, so this returns while the stamp resource is still current.
#[allow(clippy::type_complexity)]
fn stamp_training_urbanization(
	mut round: ResMut<TrainingRound>,
	settled: Option<Res<TrainingStampSettled>>,
	stamped: Option<Res<TrainingPlazaStamped>>,
	mut authored: ResMut<AuthoredDevelopments>,
	mut layer: ResMut<UrbanizationLayerRegion>,
	mut access: ParamSet<(
		(ResMut<HcsgStorage>, Res<TerrainCellLayout>),
		TerrainView<OnTerrain<Durham>>,
	)>,
	mut commands: Commands,
) {
	if let Some(stamped) = stamped.as_deref() {
		if stamped.round().map() != round.map() {
			authored.0.clear();
			layer.region = None;
			commands.remove_resource::<TrainingPlazaStamped>();
			commands.remove_resource::<TrainingStampSettled>();
		}
		return;
	}
	if settled.is_some_and(|settled| settled.0 == round.map()) {
		return;
	}
	let center = {
		let (storage, layout) = access.p0();
		if *layout != round.layout() || !storage.fills_layout(&layout) {
			return;
		}
		layout.region_center_xz().xz()
	};
	let plaza_y = access.p1().height_or_fallback(center);
	let wanted =
		training_development(training_development_cell(center), round.development_seed(), plaza_y);
	let id = wanted.id();
	if authored.0.as_slice() != std::slice::from_ref(&wanted) {
		authored.0 = vec![wanted];
		return;
	}
	let (mut storage, layout) = access.p0();
	if storage.get::<AuthoredDevelopments>(Id::Universal) != Some(&*authored) {
		return;
	}
	storage.get_one_or_generate::<TrainingDevelopment>(id);
	let fitted = storage
		.get::<TrainingDevelopment>(id)
		.filter(|development| development.is_filled())
		.and_then(|development| {
			let footprint = development.footprint_half_extents()?;
			Some((development.kind(), footprint, terrain_ids_under_pads(&storage, development)))
		});
	let Some((kind, footprint, terrain_ids)) = fitted else {
		authored.0.clear();
		let site = round.site();
		if round.site_exhausted() {
			warn!(target: "world.training", "no Richmond development fitted at site {site}");
			commands.insert_resource(TrainingStampSettled(round.map()));
		} else {
			info!(target: "world.training", "no development fitted at site {site}; rerolling");
			*round = round.reroll_site();
		}
		return;
	};
	info!(
		target: "world.training",
		"stamped {kind:?} at site {} (seed {:016x})",
		round.site(),
		round.seed,
	);
	if terrain_ids.is_empty() {
		warn!(target: "world.training", "no FinePatch cells overlapped the development pads");
	}
	layer.region = Some(layout.presentation_region());
	let courtyard = TrainingCourtyard::around(center, footprint, plaza_y);
	commands.insert_resource(TrainingPlazaStamped::new(
		*round,
		id,
		terrain_ids,
		courtyard.center,
		courtyard.half,
		courtyard.footprint,
		courtyard.plaza_y,
	));
	commands.insert_resource(TrainingStampSettled(round.map()));
}

/// Walled courtyard geometry the stamp publishes; [`TRAINING_COURTYARD`]
/// flattens the same half extents.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TrainingCourtyard {
	center: Vec2,
	half: Vec2,
	footprint: Vec2,
	plaza_y: f32,
}

impl TrainingCourtyard {
	fn around(center: Vec2, footprint: Vec2, plaza_y: f32) -> Self {
		let half = (footprint + Vec2::splat(TRAINING_COURTYARD.margin))
			.min(Vec2::splat(TRAINING_COURTYARD.max_half));
		let footprint = footprint.min(half);
		Self { center, half, footprint, plaza_y }
	}
}

/// 300 m cell centered on the FinePatch.
pub fn training_development_cell(center: Vec2) -> Aabb3d {
	let half = DEVELOPMENT_CELL_SIZE * 0.5;
	Aabb3d::from_min_max(
		Vec3::new(center.x - half, 0.0, center.y - half),
		Vec3::new(center.x + half, 1.0, center.y + half),
	)
}

/// The round's walled development: kinds from the seeded pick onward, as one
/// flat courtyard at the plaza height. Multi-terrace kinds cannot share a
/// level arena, so Richmond skips them.
pub fn training_development(cell: Aabb3d, seed: u32, plaza_y: f32) -> AuthoredDevelopment {
	let config = DevelopmentConfig::from_world_seed(seed);
	AuthoredDevelopment {
		cell,
		kinds: DevelopmentKind::filled_from_pick(cell, &config),
		height: plaza_y,
		config,
		courtyard: Some(TRAINING_COURTYARD),
	}
}

pub fn terrain_ids_under_pads(store: &HcsgStorage, development: &TrainingDevelopment) -> Vec<Id> {
	let Some(region) = pad_influence_region(development) else {
		return Vec::new();
	};
	store
		.terrain_ids_overlapping(region)
		.into_iter()
		.filter(|id| store.terrain(*id).is_some())
		.collect()
}

pub fn pad_influence_region(development: &TrainingDevelopment) -> Option<Aabb3d> {
	let mut min = Vec2::splat(f32::INFINITY);
	let mut max = Vec2::splat(f32::NEG_INFINITY);
	let mut any = false;
	for pad in development.pad_complexes() {
		min = min.min(pad.bounds.min);
		max = max.max(pad.bounds.max);
		any = true;
	}
	any.then(|| {
		Aabb3d::from_min_max(Vec3::new(min.x, -10_000.0, min.y), Vec3::new(max.x, 10_000.0, max.y))
	})
}

/// Seeds `authored` beside any earlier test authoring and generates its
/// development and hosts, as Richmond's seeds and windows would.
#[cfg(test)]
pub(crate) fn author_for_test(
	storage: &mut HcsgStorage,
	authored: AuthoredDevelopment,
) -> anyhow::Result<Id> {
	use lod::hcsg::universal_bounds;
	use richmond::{register_richmond_nodes, Built, RichmondConfig};

	if storage.get::<DevelopmentConfig>(Id::Universal).is_none() {
		register_richmond_nodes::<OnTerrain<Durham>>(storage);
		storage.seed(RichmondConfig::shared_world().development_config(), universal_bounds());
	}
	let mut all = storage.get::<AuthoredDevelopments>(Id::Universal).cloned().unwrap_or_default();
	let id = authored.id();
	all.0.push(authored);
	storage.seed(all, universal_bounds());
	storage
		.get_or_generate::<Built<OnTerrain<Durham>>>(id)
		.ok_or_else(|| anyhow::anyhow!("authored development was not built"))?;
	Ok(id)
}

/// A bare Les Halles at `height`, with no courtyard.
#[cfg(test)]
pub(crate) fn les_halles_for_test(cell: Aabb3d, height: f32, seed: u32) -> AuthoredDevelopment {
	AuthoredDevelopment {
		cell,
		kinds: vec![DevelopmentKind::LesHalles],
		height,
		config: DevelopmentConfig::from_world_seed(seed),
		courtyard: None,
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::math::Vec2;
	use bevy::prelude::{App, MinimalPlugins, NextState, World};
	use bevy::state::app::StatesPlugin;
	use durham::{
		BaseTerrainNoise, Durham, HcsgStorage, TerrainCellLayout, TerrainConfig, TerrainStorage,
		WorldBaseTerrain,
	};
	use layer_stack::Scheme;
	use layer_stack::{ActiveGenerationMode, GenerationMode, GenerationModePlugin};
	use lod::gen::{Id, OriginalId};
	use lod::hcsg::universal_bounds;
	use richmond::{
		column_bounds, register_richmond_nodes, AuthoredDevelopments, DevelopmentSite,
		RichmondConfig, RichmondNodes, DEVELOPMENT_CELL_SIZE,
	};
	use terrain_layer_model::OnTerrain;
	use urbanization_layer_model::{Urbanization, UrbanizationLayerRegion};

	use super::{
		pad_influence_region, stamp_training_urbanization, training_development_cell,
		TrainingDevelopment, TrainingPlazaStamped, TrainingStampSettled,
	};
	use crate::{TrainingGround, TrainingRound};

	struct OtherMode;

	impl GenerationMode for OtherMode {}

	fn run_stamp(world: &mut World) -> anyhow::Result<()> {
		world
			.run_system_once(stamp_training_urbanization)
			.map_err(|error| anyhow::anyhow!("{error:?}"))
	}

	/// What Richmond's `Seed<AuthoredDevelopments>` does between frames.
	fn land_seeds(world: &mut World) {
		let authored = world.resource::<AuthoredDevelopments>().clone();
		let mut storage = world.resource_mut::<HcsgStorage>();
		if storage.get::<AuthoredDevelopments>(Id::Universal) != Some(&authored) {
			storage.seed(authored, universal_bounds());
			storage.clear_group::<RichmondNodes>();
		}
	}

	fn fill_round(world: &mut World) -> anyhow::Result<()> {
		let round = *world.resource::<TrainingRound>();
		let layout = round.layout();
		let noise = BaseTerrainNoise::from_config(&TerrainConfig::new(round.development_seed()));
		world.insert_resource(WorldBaseTerrain(noise.clone()));
		world.insert_resource(layout.clone());
		{
			let mut store = world.resource_mut::<HcsgStorage>();
			for x in 0..layout.extents.x {
				for z in 0..layout.extents.y {
					store.insert_base_terrain_for_test(
						&layout,
						layout.origin.x + x as i32,
						layout.origin.y + z as i32,
						noise.clone(),
					);
				}
			}
		}
		let stored = world.resource::<HcsgStorage>().fills_layout(&layout);
		anyhow::ensure!(stored, "fine patch at {:?} was not stored", layout.origin);
		Ok(())
	}

	fn stamp_until_fitted(world: &mut World) -> anyhow::Result<Id> {
		for _ in 0..8 {
			fill_round(world)?;
			run_stamp(world)?;
			land_seeds(world);
			run_stamp(world)?;
			if let Some(stamped) = world.get_resource::<TrainingPlazaStamped>() {
				return Ok(stamped.cell_id());
			}
		}
		Err(anyhow::anyhow!("training scheme did not stamp a development"))
	}

	fn training_storage() -> HcsgStorage {
		let mut storage = HcsgStorage::default();
		register_richmond_nodes::<OnTerrain<Durham>>(&mut storage);
		storage.seed(RichmondConfig::shared_world().development_config(), universal_bounds());
		storage.seed(AuthoredDevelopments::default(), universal_bounds());
		storage
	}

	fn insert_training_resources(world: &mut World) {
		world.insert_resource(TrainingRound::new(42));
		world.insert_resource(training_storage());
		world.insert_resource(TerrainCellLayout::default());
		world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
			&TerrainConfig::new(42),
		)));
		world.init_resource::<AuthoredDevelopments>();
		world.insert_resource(UrbanizationLayerRegion::default());
	}

	fn development_at(world: &World, id: Id) -> bool {
		world
			.resource::<HcsgStorage>()
			.get::<TrainingDevelopment>(id)
			.is_some_and(TrainingDevelopment::is_filled)
	}

	#[test]
	fn training_authors_one_development_and_a_new_map_drops_it() -> anyhow::Result<()> {
		let mut world = World::new();
		insert_training_resources(&mut world);

		let cell_id = stamp_until_fitted(&mut world)?;
		anyhow::ensure!(development_at(&world, cell_id), "Richmond generated the development");
		anyhow::ensure!(world.resource::<UrbanizationLayerRegion>().region.is_some());
		let region = column_bounds(world.resource::<TerrainCellLayout>().presentation_region());
		let sites: Vec<Id> = world
			.resource_mut::<HcsgStorage>()
			.original_ids_for::<DevelopmentSite>(region)
			.into_iter()
			.map(|OriginalId(id)| id)
			.collect();
		anyhow::ensure!(sites == [cell_id], "hopscotch-off training holds one site, got {sites:?}");
		let authored = world.resource::<AuthoredDevelopments>().clone();

		let life = world.resource::<TrainingRound>().next_life();
		world.insert_resource(life);
		run_stamp(&mut world)?;
		anyhow::ensure!(
			world.resource::<TrainingPlazaStamped>().cell_id() == cell_id,
			"a new life keeps the stamped cell"
		);
		anyhow::ensure!(
			*world.resource::<AuthoredDevelopments>() == authored,
			"a new life does not re-author"
		);

		let map = world.resource::<TrainingRound>().next();
		world.insert_resource(map);
		fill_round(&mut world)?;
		run_stamp(&mut world)?;
		anyhow::ensure!(world.resource::<AuthoredDevelopments>().0.is_empty());
		anyhow::ensure!(
			world.get_resource::<TrainingPlazaStamped>().is_none(),
			"a new map drops the stamp so the next pass can restamp"
		);
		anyhow::ensure!(
			world.get_resource::<TrainingStampSettled>().is_none(),
			"a new map drops the settled marker"
		);
		anyhow::ensure!(world.resource::<UrbanizationLayerRegion>().region.is_none());
		land_seeds(&mut world);
		anyhow::ensure!(!development_at(&world, cell_id), "a new map removes the previous cell");

		let new_id = stamp_until_fitted(&mut world)?;
		anyhow::ensure!(new_id != cell_id, "the new map stamps a different cell");
		let restamped = world.resource::<AuthoredDevelopments>().clone();
		run_stamp(&mut world)?;
		let stamped_again =
			world.get_resource::<TrainingPlazaStamped>().map(TrainingPlazaStamped::cell_id);
		anyhow::ensure!(
			stamped_again == Some(new_id) && *world.resource::<AuthoredDevelopments>() == restamped,
			"the new map stamps once"
		);
		Ok(())
	}

	#[test]
	fn leaving_and_reentering_the_same_map_restamps() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<TrainingGround>::initial(),
			GenerationModePlugin::<OtherMode>::default(),
		));
		<TrainingGround as Scheme<Urbanization<richmond::Richmond<OnTerrain<Durham>>>>>::install(
			&mut app,
			&richmond::RichmondConfig::default(),
		);
		insert_training_resources(app.world_mut());

		let cell_id = stamp_until_fitted(app.world_mut())?;
		anyhow::ensure!(development_at(app.world(), cell_id), "the first enter stamps a courtyard");

		app.world_mut()
			.resource_mut::<NextState<ActiveGenerationMode>>()
			.set(ActiveGenerationMode::of::<OtherMode>());
		app.update();
		anyhow::ensure!(
			app.world().get_resource::<TrainingPlazaStamped>().is_none(),
			"leaving Training drops the stamp"
		);
		anyhow::ensure!(
			app.world().get_resource::<TrainingStampSettled>().is_none(),
			"leaving Training drops the settled marker"
		);
		anyhow::ensure!(
			app.world().resource::<AuthoredDevelopments>().0.is_empty(),
			"other modes see no training development"
		);
		land_seeds(app.world_mut());

		app.world_mut()
			.resource_mut::<NextState<ActiveGenerationMode>>()
			.set(ActiveGenerationMode::of::<TrainingGround>());
		app.update();
		let restamped = stamp_until_fitted(app.world_mut())?;
		anyhow::ensure!(
			development_at(app.world(), restamped),
			"the courtyard development is generated again"
		);
		Ok(())
	}

	#[test]
	fn training_cell_is_centered_on_the_site() -> anyhow::Result<()> {
		for center in [Vec2::ZERO, Vec2::new(-4_800.0, 1_120.0)] {
			let cell = training_development_cell(center);
			anyhow::ensure!(((cell.min.x + cell.max.x) * 0.5 - center.x).abs() < 1e-3);
			anyhow::ensure!(((cell.min.z + cell.max.z) * 0.5 - center.y).abs() < 1e-3);
			anyhow::ensure!((cell.max.x - cell.min.x - DEVELOPMENT_CELL_SIZE).abs() < 1e-3);
		}
		Ok(())
	}

	#[test]
	fn empty_cell_has_no_pad_influence() -> anyhow::Result<()> {
		let empty = TrainingDevelopment::Empty(training_development_cell(Vec2::ZERO));
		anyhow::ensure!(pad_influence_region(&empty).is_none());
		Ok(())
	}
}
