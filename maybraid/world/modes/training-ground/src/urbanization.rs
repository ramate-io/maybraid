//! Training's urbanization scheme: one walled development in the layer store.

use bevy::ecs::system::ParamSet;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use durham::{Durham, TerrainCellLayout, TerrainEntryStore};
use lod::gen::Id;
use richmond::{
	DEVELOPMENT_CELL_SIZE, DevelopmentCell, DevelopmentConfig, DevelopmentEntryStore,
	DevelopmentKind, PadParams, RichmondGroundView,
};
use terrain_layer_model::{OnTerrain, TerrainView};
use layer_stack::{ActiveGenerationMode, GenerationModeSystems};
use richmond::Richmond;
use urbanization_layer_model::{
	UrbanizationLayerRegion, UrbanizationScheme, UrbanizationStoreSystems,
};

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

impl UrbanizationScheme<Richmond<OnTerrain<Durham>>> for TrainingGround {
	fn install(app: &mut App, _config: &richmond::RichmondConfig) {
		app.add_systems(
			Update,
			stamp_training_urbanization
				.in_set(GenerationModeSystems::<TrainingGround>::default())
				.in_set(UrbanizationStoreSystems),
		);
		app.add_systems(
			OnExit(ActiveGenerationMode::of::<TrainingGround>()),
			clear_training_stamp,
		);
	}
}

fn clear_training_stamp(mut commands: Commands) {
	commands.remove_resource::<TrainingPlazaStamped>();
	commands.remove_resource::<TrainingStampSettled>();
}

/// Fit the round's development once the FinePatch is stored.
///
/// A new map drops the previous cell and stamp here. Leaving Training drops
/// the stamp on [`OnExit`], so a later enter on the same map stamps again.
/// A new life keeps the map, so this returns while the stamp resource is still current.
#[allow(clippy::type_complexity)]
fn stamp_training_urbanization(
	mut round: ResMut<TrainingRound>,
	settled: Option<Res<TrainingStampSettled>>,
	stamped: Option<Res<TrainingPlazaStamped>>,
	mut access: ParamSet<(
		(
			Res<TerrainEntryStore>,
			Res<TerrainCellLayout>,
			ResMut<DevelopmentEntryStore>,
			ResMut<UrbanizationLayerRegion>,
			RichmondGroundView,
		),
		TerrainView<OnTerrain<Durham>>,
	)>,
	mut commands: Commands,
) {
	let current = stamped.as_deref().map(|stamped| (stamped.cell_id(), stamped.round().map()));
	if let Some((id, map)) = current {
		if map != round.map() {
			let (_, _, mut developments, mut layer, _) = access.p0();
			developments.remove_cell(id);
			let _ = developments.invalidate_dirty_padded();
			layer.region = None;
			commands.remove_resource::<TrainingPlazaStamped>();
			commands.remove_resource::<TrainingStampSettled>();
		}
		return;
	}
	if settled.is_some_and(|settled| settled.0 == round.map()) {
		return;
	}
	let waiting = {
		let (store, layout, _, _, _) = access.p0();
		*layout != round.layout() || !store.fills_layout(&layout)
	};
	if waiting {
		return;
	}
	let center = {
		let (_, layout, _, _, _) = access.p0();
		layout.region_center_xz().xz()
	};
	let cell = training_development_cell(center);
	let plaza_y = access.p1().height_or_fallback(center);
	let config = DevelopmentConfig::from_world_seed(round.development_seed());
	let fitted = {
		let (_, _, _, _, ground) = access.p0();
		stamp_training_development(&ground, cell, &config, plaza_y)
	};
	let Some((kind, filled, built, courtyard)) = fitted else {
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
	let cell_id = Id::from_cell(filled.cell);
	let terrain_ids = {
		let (store, layout, mut developments, mut layer, _) = access.p0();
		let terrain_ids = terrain_ids_under_pads(&store, &filled);
		developments.insert_cell(cell_id, filled.clone());
		developments.insert_built(cell_id, built, filled.cell);
		layer.region = Some(layout.presentation_region());
		terrain_ids
	};
	if terrain_ids.is_empty() {
		warn!(target: "world.training", "no FinePatch cells overlapped the development pads");
	}
	commands.insert_resource(TrainingPlazaStamped::new(
		*round,
		cell_id,
		terrain_ids,
		courtyard.center,
		courtyard.half,
		courtyard.footprint,
		courtyard.plaza_y,
	));
	commands.insert_resource(TrainingStampSettled(round.map()));
}

/// Walled courtyard geometry the stamp publishes.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TrainingCourtyard {
	center: Vec2,
	half: Vec2,
	footprint: Vec2,
	plaza_y: f32,
}

impl TrainingCourtyard {
	fn around(center: Vec2, footprint: Vec2, plaza_y: f32) -> Self {
		let half = (footprint + Vec2::splat(TRAINING_ARENA_MARGIN_M))
			.min(Vec2::splat(TRAINING_ARENA_MAX_HALF_M));
		let footprint = footprint.min(half);
		Self { center, half, footprint, plaza_y }
	}

	fn courtyard_half(self) -> Vec2 {
		self.half + Vec2::splat(TRAINING_COURTYARD_OVERHANG_M)
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

/// First single-terrace kind from the seeded pick onward, re-padded as one
/// flat walled courtyard. Multi-terrace kinds cannot share a level arena.
fn stamp_training_development(
	ground: &RichmondGroundView,
	cell: Aabb3d,
	config: &DevelopmentConfig,
	height: f32,
) -> Option<(DevelopmentKind, DevelopmentCell, richmond::BuiltDevelopment, TrainingCourtyard)>
{
	let preferred = DevelopmentKind::pick_filled(cell, config);
	let start = DevelopmentKind::FILLED.iter().position(|kind| *kind == preferred).unwrap_or(0);
	let count = DevelopmentKind::FILLED.len();
	for kind in (0..count).map(|i| DevelopmentKind::FILLED[(start + i) % count]) {
		let Some(filled) =
			DevelopmentCell::fill::<OnTerrain<Durham>>(ground, cell, kind, config, height)
		else {
			continue;
		};
		let Some(footprint) = filled.footprint_half_extents() else {
			continue;
		};
		let center = Vec2::new((cell.min.x + cell.max.x) * 0.5, (cell.min.z + cell.max.z) * 0.5);
		let courtyard = TrainingCourtyard::around(center, footprint, height);
		let params = PadParams { berm: 0.0, ease: TRAINING_COURTYARD_EASE_M, round: 0.0 };
		let Some(walled) = filled.with_courtyard(courtyard.courtyard_half(), params) else {
			continue;
		};
		let Some(built) = walled.built(config.seed as i32) else {
			continue;
		};
		return Some((kind, walled, built, courtyard));
	}
	None
}

pub fn terrain_ids_under_pads(store: &TerrainEntryStore, filled: &DevelopmentCell) -> Vec<Id> {
	let Some(region) = pad_influence_region(filled) else {
		return Vec::new();
	};
	store
		.terrain_ids_overlapping(region)
		.into_iter()
		.filter(|id| store.terrain(*id).is_some())
		.collect()
}

pub fn pad_influence_region(filled: &DevelopmentCell) -> Option<Aabb3d> {
	let mut min = Vec2::splat(f32::INFINITY);
	let mut max = Vec2::splat(f32::NEG_INFINITY);
	let mut any = false;
	for pad in filled.pad_complexes() {
		min = min.min(pad.bounds.min);
		max = max.max(pad.bounds.max);
		any = true;
	}
	any.then(|| {
		Aabb3d::from_min_max(Vec3::new(min.x, -10_000.0, min.y), Vec3::new(max.x, 10_000.0, max.y))
	})
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::math::bounding::Aabb3d;
	use bevy::math::{Vec2, Vec3};
	use bevy::prelude::{App, MinimalPlugins, NextState, World};
	use bevy::state::app::StatesPlugin;
	use durham::{
		BaseTerrainNoise, Durham, TerrainCellLayout, TerrainConfig, TerrainEntryStore,
		WorldBaseTerrain,
	};
	use lod::gen::Id;
	use richmond::{
		DEVELOPMENT_CELL_SIZE, DevelopmentCell, DevelopmentEntryStore,
	};
	use terrain_layer_model::{OnTerrain};
use layer_stack::{ActiveGenerationMode, GenerationMode, GenerationModePlugin};
	use urbanization_layer_model::{UrbanizationLayerRegion, UrbanizationScheme};

	use super::{
		pad_influence_region, stamp_training_urbanization, training_development_cell,
		TrainingPlazaStamped, TrainingStampSettled,
	};
	use crate::{TrainingGround, TrainingRound};

	struct OtherMode;

	impl GenerationMode for OtherMode {}

	fn run_stamp(world: &mut World) -> anyhow::Result<()> {
		world
			.run_system_once(stamp_training_urbanization)
			.map_err(|error| anyhow::anyhow!("{error:?}"))
	}

	fn fill_round(world: &mut World) -> anyhow::Result<()> {
		let round = *world.resource::<TrainingRound>();
		let layout = round.layout();
		let noise = BaseTerrainNoise::from_config(&TerrainConfig::new(round.development_seed()));
		world.insert_resource(WorldBaseTerrain(noise.clone()));
		world.insert_resource(layout.clone());
		{
			let mut store = world.resource_mut::<TerrainEntryStore>();
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
		let stored = world.resource::<TerrainEntryStore>().fills_layout(&layout);
		anyhow::ensure!(stored, "fine patch at {:?} was not stored", layout.origin);
		Ok(())
	}

	fn stamp_until_fitted(world: &mut World) -> anyhow::Result<Id> {
		for _ in 0..8 {
			fill_round(world)?;
			run_stamp(world)?;
			if let Some(stamped) = world.get_resource::<TrainingPlazaStamped>() {
				return Ok(stamped.cell_id());
			}
		}
		Err(anyhow::anyhow!("training scheme did not stamp a development"))
	}

	fn stamp_world() -> World {
		let mut world = World::new();
		world.insert_resource(TrainingRound::new(42));
		world.insert_resource(TerrainEntryStore::default());
		world.insert_resource(TerrainCellLayout::default());
		world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
			&TerrainConfig::new(42),
		)));
		world.insert_resource(DevelopmentEntryStore::default());
		world.insert_resource(UrbanizationLayerRegion::default());
		world
	}

	#[test]
	fn training_scheme_stamps_one_development_and_leaving_clears_the_store() -> anyhow::Result<()> {
		let mut world = stamp_world();
		let other = Aabb3d::from_min_max(
			Vec3::new(80_000.0, 0.0, 80_000.0),
			Vec3::new(80_040.0, 1.0, 80_040.0),
		);
		let other_id = Id::from_cell(other);
		world
			.resource_mut::<DevelopmentEntryStore>()
			.insert_cell(other_id, DevelopmentCell::empty(other));

		let cell_id = stamp_until_fitted(&mut world)?;
		anyhow::ensure!(cell_id != other_id, "stamp collided with the pre-existing cell");
		{
			let store = world.resource::<DevelopmentEntryStore>();
			anyhow::ensure!(store.cell(cell_id).is_some(), "stamp inserts the development cell");
			anyhow::ensure!(store.built_at(cell_id).is_some(), "stamp inserts the built development");
			anyhow::ensure!(store.cell(other_id).is_some(), "pre-existing cell stays");
		}
		let revision = world.resource::<DevelopmentEntryStore>().membership_revision();

		let life = world.resource::<TrainingRound>().next_life();
		world.insert_resource(life);
		run_stamp(&mut world)?;
		anyhow::ensure!(
			world.resource::<TrainingPlazaStamped>().cell_id() == cell_id,
			"a new life keeps the stamped cell"
		);
		anyhow::ensure!(
			world.resource::<DevelopmentEntryStore>().membership_revision() == revision,
			"a new life does not restamp"
		);

		let map = world.resource::<TrainingRound>().next();
		world.insert_resource(map);
		fill_round(&mut world)?;
		run_stamp(&mut world)?;
		anyhow::ensure!(
			world.resource::<DevelopmentEntryStore>().cell(cell_id).is_none(),
			"a new map removes the previous cell"
		);
		anyhow::ensure!(
			world.get_resource::<TrainingPlazaStamped>().is_none(),
			"a new map drops the stamp so the next pass can restamp"
		);
		anyhow::ensure!(
			world.get_resource::<TrainingStampSettled>().is_none(),
			"a new map drops the settled marker"
		);
		let new_id = stamp_until_fitted(&mut world)?;
		anyhow::ensure!(new_id != cell_id, "the new map stamps a different cell");
		let restamped = world.resource::<DevelopmentEntryStore>().membership_revision();
		run_stamp(&mut world)?;
		let stamped_again = world.get_resource::<TrainingPlazaStamped>().map(TrainingPlazaStamped::cell_id);
		let revision_again = world.resource::<DevelopmentEntryStore>().membership_revision();
		anyhow::ensure!(
			stamped_again == Some(new_id) && revision_again == restamped,
			"the new map stamps once"
		);

		world.resource_mut::<DevelopmentEntryStore>().clear();
		let store = world.resource::<DevelopmentEntryStore>();
		anyhow::ensure!(store.cell(new_id).is_none(), "leaving Training leaves the store empty");
		anyhow::ensure!(store.built_at(new_id).is_none(), "leaving drops the built development");
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
		<TrainingGround as UrbanizationScheme<richmond::Richmond<OnTerrain<Durham>>>>::install(
			&mut app,
			&richmond::RichmondConfig::default(),
		);
		app.insert_resource(TrainingRound::new(42));
		app.insert_resource(TerrainEntryStore::default());
		app.insert_resource(TerrainCellLayout::default());
		app.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
			&TerrainConfig::new(42),
		)));
		app.insert_resource(DevelopmentEntryStore::default());
		app.insert_resource(UrbanizationLayerRegion::default());

		let cell_id = stamp_until_fitted(app.world_mut())?;
		anyhow::ensure!(
			app.world().resource::<DevelopmentEntryStore>().cell(cell_id).is_some(),
			"the first enter stamps a courtyard"
		);

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
		app.world_mut().resource_mut::<DevelopmentEntryStore>().clear();

		app.world_mut()
			.resource_mut::<NextState<ActiveGenerationMode>>()
			.set(ActiveGenerationMode::of::<TrainingGround>());
		app.update();
		let restamped = stamp_until_fitted(app.world_mut())?;
		anyhow::ensure!(
			app.world().get_resource::<TrainingPlazaStamped>().is_some(),
			"re-entering the same map stamps again"
		);
		anyhow::ensure!(
			app.world().resource::<DevelopmentEntryStore>().cell(restamped).is_some(),
			"the courtyard development is stored again"
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
		let empty = DevelopmentCell::empty(training_development_cell(Vec2::ZERO));
		anyhow::ensure!(pad_influence_region(&empty).is_none());
		Ok(())
	}
}
