//! Training Ground is a seeded FinePatch of the Maybraid world, not Discovery's
//! moving rings.
//!
//! While [`crate::WorldMode::Training`] is active, Durham presents a four-cell
//! window pinned on the [`TrainingRound`] site, hopscotch stays off, the forest
//! shrinks to one grove tile, and a Training pose is not written.
//! [`crate::training_plaza`] stamps one seeded Richmond development onto that
//! patch — pads first, then hosts — once the whole window exists.
//!
//! A Training respawn ends the life with [`TrainingLifeEnded`], and the shell
//! advances [`TrainingRound`]. A new round moves the patch, tears the plaza
//! down, and stamps the next one. A new life on the same map keeps the plaza
//! and only rolls the next trainee.
//!
//! Each session has one terrain collider owner. Discovery's is Richmond's
//! urbanized presenter, which turning urbanization off tears down. Training's
//! is Durham's raw FinePatch, except where Training's padded fills supersede
//! it under the courtyard.

use bevy::prelude::*;
use combat_hud::{CombatScore, LiveEnemies};
use crozon_character_items::{random_starter_loadout, Inventory, ItemRng};
use crozon_characters::species::{
	braidman::BraidmanConfig, lero::LeroConfig, mygr::MygrConfig, tuberwaber::TuberwaberConfig,
	wumbus::WumbusConfig,
};
use crozon_characters::CharacterAppearance;
use damage::{Downed, Health};
use durham_terrain_models::{
	TerrainCellLayout, TerrainColliderSystems, TerrainCoverage, TerrainFillSystems,
	TerrainLayoutPinned, TerrainPresentEnabled, TerrainPresentPending, TerrainPresentationAssets,
	TerrainPresentationDirty, TerrainPresenterState, WORLD_FINE_HALF_EXTENT_CELLS,
	fine_patch_cell_layout, playable_world_cell_layout, retarget_presentation_assets,
};
use mob_intelligence::MemberOf;
use mob_layer_model::MobStreamSuspended;
use urbanization_layer_model::UrbanizationStreamingEnabled;
use vegetation_layer_model::VegetationLayerConfig;

use crate::WorldPlayerLoadout;
use crate::control::{WorldSurfaceSet, update_world_surface_ready};
use crate::training_markers::{TrainingEnemyMarkersEnabled, sync_training_enemy_markers};
use crate::training_plaza::{
	TrainingBrawler, clear_training_plaza, mount_training_plaza, park_on_training_site,
	promote_training_plaza, reseat_training_life, supersede_training_raw_terrain,
};
use crate::world_mode::WorldMode;

/// Training Ground session: patch retarget, the stamped plaza, and the
/// raw-to-padded hand-off under its courtyard.
pub(crate) struct TrainingGroundPlugin;

impl Plugin for TrainingGroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<TrainingRound>()
			.init_resource::<AppliedTrainingMap>()
			.init_resource::<TrainingEnemyMarkersEnabled>()
			.add_message::<TrainingLifeEnded>();
		register_world_mode_transitions(app);
		app.add_systems(
			Update,
			(
				park_on_training_site.after(retarget_training_map),
				clear_training_terrain_present,
				mount_training_plaza.in_set(WorldSurfaceSet).after(update_world_surface_ready),
				(supersede_training_raw_terrain, promote_training_plaza)
					.chain()
					.after(TerrainColliderSystems::QueueMeshes),
				reseat_training_life,
				count_training_enemies,
				sync_training_enemy_markers,
			),
		)
		// Mob, threat, and combat systems queue plain inserts on squad hosts
		// and members all through Update and PostUpdate; tearing them down any
		// earlier in the frame panics those commands.
		.add_systems(Last, clear_training_plaza);
	}
}

/// The map whose fill is currently applied. `None` is Discovery's rings.
#[derive(Resource, Default)]
struct AppliedTrainingMap(Option<TrainingMap>);

/// Mode transitions. Plaza systems stay on the plugin so a headless transition
/// test can register this without the rest of the stack.
fn register_world_mode_transitions(app: &mut App) {
	app.add_systems(
		Update,
		retarget_training_map
			.run_if(in_state(WorldMode::Training))
			.before(TerrainFillSystems::Generate),
	)
	.add_systems(OnEnter(WorldMode::Training), (enter_training, open_training_score))
	.add_systems(
		OnTransition { exited: WorldMode::Training, entered: WorldMode::Discovery },
		return_to_discovery,
	)
	.add_systems(OnExit(WorldMode::Training), close_training_score);
}

/// Forest stream radius used by [`VegetationLayerConfig::world_defaults`].
const WORLD_FOREST_STREAM_RADIUS: u32 = 1;
/// Half-extent of the training patch layout.
const TRAINING_FINE_HALF_EXTENT_CELLS: i32 = 2;
/// Sites land within this many 160 m cells of the world origin on each axis.
const TRAINING_SITE_RANGE_CELLS: i32 = 48;
/// Sites tried per round before Training gives up on stamping a development.
const TRAINING_SITE_ATTEMPTS: u32 = 8;
const TRAINING_SITE_SALT: u64 = 0x51_7E5A_17E5;
const TRAINING_DEVELOPMENT_SALT: u64 = 0xDE7E_10A5;
const TRAINING_MOB_SALT: u64 = 0x0B_5EED;
const TRAINING_TRAINEE_SALT: u64 = 0x7EA1_4EE5;
const TRAINING_NEXT_SALT: u64 = 0x9E37_79B9_7F4A_7C15;

/// One Training life. The shell rolls the first from entropy. A respawn
/// advances to [`Self::next`] (a fresh site, development, roster, and
/// trainee) or [`Self::next_life`] (the next trainee on the same map).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingRound {
	pub seed: u64,
	/// Sites already tried for this seed. A site that fits no development rerolls.
	site_attempt: u32,
	/// Lives played on this map.
	life: u32,
}

/// What a round stamps. Every life on the same map shares it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingMap {
	seed: u64,
	site_attempt: u32,
}

impl Default for TrainingRound {
	fn default() -> Self {
		Self::new(42)
	}
}

impl TrainingRound {
	pub const fn new(seed: u64) -> Self {
		Self { seed, site_attempt: 0, life: 0 }
	}

	pub fn from_entropy() -> Self {
		let nanos = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|elapsed| elapsed.as_nanos() as u64)
			.unwrap_or(0);
		Self::new(mix(nanos))
	}

	/// A new map after this life.
	pub fn next(self) -> Self {
		Self::new(self.lane(TRAINING_NEXT_SALT))
	}

	/// The next life on this map.
	pub fn next_life(self) -> Self {
		Self { life: self.life.wrapping_add(1), ..self }
	}

	pub fn map(self) -> TrainingMap {
		TrainingMap { seed: self.seed, site_attempt: self.site_attempt }
	}

	pub(crate) fn reroll_site(self) -> Self {
		Self { site_attempt: self.site_attempt + 1, ..self }
	}

	pub(crate) fn site_exhausted(self) -> bool {
		self.site_attempt + 1 >= TRAINING_SITE_ATTEMPTS
	}

	/// Cell corner the patch centers on.
	pub fn site(self) -> IVec2 {
		let lane = self.lane(TRAINING_SITE_SALT ^ u64::from(self.site_attempt).rotate_left(17));
		let span = (2 * TRAINING_SITE_RANGE_CELLS + 1) as u64;
		let axis = |bits: u64| (bits % span) as i32 - TRAINING_SITE_RANGE_CELLS;
		IVec2::new(axis(lane), axis(lane >> 32))
	}

	pub fn layout(self) -> TerrainCellLayout {
		let half = TRAINING_FINE_HALF_EXTENT_CELLS;
		fine_patch_cell_layout(half, self.site() - IVec2::splat(half))
	}

	pub fn development_seed(self) -> u32 {
		self.lane(TRAINING_DEVELOPMENT_SALT) as u32
	}

	/// Mob number for the first squad. 24 bits, so every squad offset stays exact.
	pub fn mob_seed(self) -> f32 {
		(self.lane(TRAINING_MOB_SALT) >> 40) as f32
	}

	/// A player-scale playable species in a rolled starter loadout. Never saved.
	pub fn trainee(self) -> WorldPlayerLoadout {
		let life = u64::from(self.life).rotate_left(23);
		let mut rng = ItemRng::from_seed(self.lane(TRAINING_TRAINEE_SALT ^ life));
		let appearance = match rng.gen_index(5) {
			0 => CharacterAppearance::Braidman(BraidmanConfig::default_preview()),
			1 => CharacterAppearance::Lero(LeroConfig::default_preview()),
			2 => CharacterAppearance::Mygr(MygrConfig::default_preview()),
			3 => CharacterAppearance::Tuberwaber(TuberwaberConfig::default_preview()),
			_ => CharacterAppearance::Wumbus(WumbusConfig::default_preview()),
		};
		let inventory = Inventory::with_starter_outfit(random_starter_loadout(&mut rng));
		let key = format!("trainee-{:016x}-{}", self.seed, self.life);
		WorldPlayerLoadout::new(key, appearance, inventory).with_name("Trainee")
	}

	fn lane(self, salt: u64) -> u64 {
		mix(self.seed ^ salt)
	}
}

/// A Training body respawned. The shell advances [`TrainingRound`] and loads
/// the next life in behind the loading screen.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingLifeEnded;

fn mix(value: u64) -> u64 {
	let mut value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
	value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
	value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
	value ^ (value >> 31)
}

/// What the world fill should be for a Training / Discovery session.
#[derive(Clone, Debug, PartialEq)]
pub struct TrainingFill {
	pub layout: TerrainCellLayout,
	pub coverage: TerrainCoverage,
	pub terrain_radius: i32,
	pub present: bool,
	pub pin_layout: bool,
	pub urbanization: bool,
	pub forest_stream_radius: u32,
}

impl TrainingFill {
	/// Training's patch for `round`, or Discovery's playable rings for `None`.
	pub fn for_session(round: Option<TrainingRound>) -> Self {
		match round {
			Some(round) => Self {
				layout: round.layout(),
				coverage: TerrainCoverage::FinePatch,
				terrain_radius: TRAINING_FINE_HALF_EXTENT_CELLS,
				present: true,
				pin_layout: true,
				urbanization: false,
				forest_stream_radius: 0,
			},
			None => Self {
				layout: playable_world_cell_layout(),
				coverage: TerrainCoverage::PlayableWorld,
				terrain_radius: WORLD_FINE_HALF_EXTENT_CELLS,
				present: false,
				pin_layout: false,
				urbanization: true,
				forest_stream_radius: WORLD_FOREST_STREAM_RADIUS,
			},
		}
	}
}

/// Entering Training pins the round's patch and suspends the mob stream.
///
/// `OnEnter` runs in `StateTransition`, before [`TerrainFillSystems::Generate`].
/// Startup stays in Discovery, so this does not retarget the first frame.
fn enter_training(
	round: Res<TrainingRound>,
	mut applied: ResMut<AppliedTrainingMap>,
	mut suspended: ResMut<MobStreamSuspended>,
	mut layout: ResMut<TerrainCellLayout>,
	mut coverage: ResMut<TerrainCoverage>,
	mut present: ResMut<TerrainPresentEnabled>,
	mut pinned: ResMut<TerrainLayoutPinned>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut assets: ResMut<TerrainPresentationAssets>,
	mut vegetation: Option<ResMut<VegetationLayerConfig>>,
	mut urban: Option<ResMut<UrbanizationStreamingEnabled>>,
) {
	applied.0 = Some(round.map());
	suspended.0 = true;
	write_session_fill(
		Some(*round),
		&mut layout,
		&mut coverage,
		&mut present,
		&mut pinned,
		&mut dirty,
		&mut pending,
		&mut assets,
		vegetation.as_deref_mut(),
		urban.as_deref_mut(),
	);
}

/// Discovery's rings, after Training. `OnTransition` does not run for the
/// initial enter of Discovery, so startup leaves Durham's own dirty flag alone.
fn return_to_discovery(
	mut applied: ResMut<AppliedTrainingMap>,
	mut suspended: ResMut<MobStreamSuspended>,
	mut layout: ResMut<TerrainCellLayout>,
	mut coverage: ResMut<TerrainCoverage>,
	mut present: ResMut<TerrainPresentEnabled>,
	mut pinned: ResMut<TerrainLayoutPinned>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut assets: ResMut<TerrainPresentationAssets>,
	mut vegetation: Option<ResMut<VegetationLayerConfig>>,
	mut urban: Option<ResMut<UrbanizationStreamingEnabled>>,
) {
	applied.0 = None;
	suspended.0 = false;
	write_session_fill(
		None,
		&mut layout,
		&mut coverage,
		&mut present,
		&mut pinned,
		&mut dirty,
		&mut pending,
		&mut assets,
		vegetation.as_deref_mut(),
		urban.as_deref_mut(),
	);
}

/// A new [`TrainingRound`] map while Training stays active. A new life keeps
/// the map, so the patch does not move. `NextState::set` of the current mode
/// would exit and enter Training; the shell uses `set_if_neq` instead, and this
/// path moves the patch without that exit.
fn retarget_training_map(
	round: Res<TrainingRound>,
	mut applied: ResMut<AppliedTrainingMap>,
	mut layout: ResMut<TerrainCellLayout>,
	mut coverage: ResMut<TerrainCoverage>,
	mut present: ResMut<TerrainPresentEnabled>,
	mut pinned: ResMut<TerrainLayoutPinned>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut assets: ResMut<TerrainPresentationAssets>,
	mut vegetation: Option<ResMut<VegetationLayerConfig>>,
	mut urban: Option<ResMut<UrbanizationStreamingEnabled>>,
) {
	if applied.0 == Some(round.map()) {
		return;
	}
	applied.0 = Some(round.map());
	write_session_fill(
		Some(*round),
		&mut layout,
		&mut coverage,
		&mut present,
		&mut pinned,
		&mut dirty,
		&mut pending,
		&mut assets,
		vegetation.as_deref_mut(),
		urban.as_deref_mut(),
	);
}

fn write_session_fill(
	round: Option<TrainingRound>,
	layout: &mut TerrainCellLayout,
	coverage: &mut TerrainCoverage,
	present: &mut TerrainPresentEnabled,
	pinned: &mut TerrainLayoutPinned,
	dirty: &mut TerrainPresentationDirty,
	pending: &mut TerrainPresentPending,
	assets: &mut TerrainPresentationAssets,
	vegetation: Option<&mut VegetationLayerConfig>,
	urban: Option<&mut UrbanizationStreamingEnabled>,
) {
	let fill = TrainingFill::for_session(round);
	*layout = fill.layout.clone();
	*coverage = fill.coverage;
	present.0 = fill.present;
	pinned.0 = fill.pin_layout;
	dirty.0 = true;
	pending.0 = true;
	retarget_presentation_assets(assets, fill.coverage, fill.terrain_radius);
	if let Some(urban) = urban {
		urban.0 = fill.urbanization;
	}
	if let Some(config) = vegetation {
		if let Some(spec) = config.forest.as_mut() {
			spec.stream_radius = fill.forest_stream_radius;
		}
	}
}

/// Each Training session keeps its own [`CombatScore`] across its rounds and
/// lives; leaving drops it, which hides the score panel.
fn open_training_score(mut commands: Commands) {
	commands.insert_resource(CombatScore::default());
}

fn close_training_score(mut commands: Commands) {
	commands.remove_resource::<CombatScore>();
}

/// Keep [`LiveEnemies`] at the number of training fighters still standing. A
/// downed fighter drops out until its squad respawns it.
pub(crate) fn count_training_enemies(
	mode: Res<State<WorldMode>>,
	live: Option<ResMut<LiveEnemies>>,
	squads: Query<(), With<TrainingBrawler>>,
	fighters: Query<(&MemberOf, &Health), Without<Downed>>,
	mut commands: Commands,
) {
	if !mode.get().is_training() {
		if live.is_some() {
			commands.remove_resource::<LiveEnemies>();
		}
		return;
	}
	let standing = fighters
		.iter()
		.filter(|(member, health)| squads.contains(member.mob) && !health.is_dead())
		.count();
	let count = LiveEnemies(u32::try_from(standing).unwrap_or(u32::MAX));
	match live {
		Some(mut live) => {
			live.set_if_neq(count);
		}
		None => commands.insert_resource(count),
	}
}

/// Drop raw training terrain meshes once present is turned back off.
pub(crate) fn clear_training_terrain_present(
	present: Res<TerrainPresentEnabled>,
	mut was_present: Local<bool>,
	mut commands: Commands,
	mut state: ResMut<TerrainPresenterState>,
) {
	if *was_present && !present.0 {
		state.clear(&mut commands);
	}
	*was_present = present.0;
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::state::app::StatesPlugin;
	use durham_terrain_models::{BaseTerrainNoise, TerrainConfig, WorldBaseTerrain};
	use richmond_development_models::DevelopmentEntryStore;
	use urbanization_layer_model::UrbanizationStreamingEnabled;
	use vegetation_layer_model::VegetationLayerConfig;

	use crate::PlayerSpawnXz;
	use crate::training_plaza::TrainingPlazaMounted;
	use crate::world_mode::WorldModePlugin;
	use super::*;

	fn training_world(round: TrainingRound) -> World {
		let mut world = World::new();
		world.insert_resource(round);
		world.init_resource::<AppliedTrainingMap>();
		world.insert_resource(MobStreamSuspended(false));
		world.insert_resource(VegetationLayerConfig::world_defaults());
		world.insert_resource(UrbanizationStreamingEnabled(true));
		world.insert_resource(playable_world_cell_layout());
		world.insert_resource(TerrainCoverage::PlayableWorld);
		world.insert_resource(TerrainPresentEnabled(false));
		world.insert_resource(TerrainLayoutPinned(false));
		world.insert_resource(TerrainPresentationDirty(false));
		world.insert_resource(TerrainPresentPending(false));
		world.insert_resource(TerrainPresentationAssets {
			config: TerrainConfig::new(42),
			material: Handle::default(),
			lod_bands: Vec::new(),
			outer_add_walls: true,
			fine_grid_max_radius: Some(WORLD_FINE_HALF_EXTENT_CELLS),
			macro_seam_half_extents: Vec::new(),
			macro_cell_min_size: None,
			macro_res_2: None,
		});
		world
	}

	fn forest_radius(world: &World) -> Option<u32> {
		world.resource::<VegetationLayerConfig>().forest.map(|spec| spec.stream_radius)
	}

	#[test]
	fn entering_training_pins_the_round_site() -> anyhow::Result<()> {
		let round = TrainingRound::new(7);
		let mut world = training_world(round);
		world.run_system_once(enter_training).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<TerrainCellLayout>(), round.layout());
		assert_eq!(*world.resource::<TerrainCoverage>(), TerrainCoverage::FinePatch);
		assert!(world.resource::<TerrainPresentEnabled>().0);
		assert!(world.resource::<TerrainLayoutPinned>().0);
		assert!(world.resource::<TerrainPresentationDirty>().0);
		assert!(world.resource::<TerrainPresentPending>().0);
		assert!(world.resource::<MobStreamSuspended>().0);
		assert!(!world.resource::<UrbanizationStreamingEnabled>().0);
		assert_eq!(forest_radius(&world), Some(0));
		Ok(())
	}

	#[test]
	fn returning_to_discovery_restores_the_playable_rings() -> anyhow::Result<()> {
		let round = TrainingRound::new(7);
		let mut world = training_world(round);
		world.run_system_once(enter_training).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.resource_mut::<TerrainPresentationDirty>().0 = false;
		world
			.run_system_once(return_to_discovery)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<TerrainCellLayout>(), playable_world_cell_layout());
		assert_eq!(*world.resource::<TerrainCoverage>(), TerrainCoverage::PlayableWorld);
		assert!(!world.resource::<TerrainPresentEnabled>().0);
		assert!(!world.resource::<TerrainLayoutPinned>().0);
		assert!(world.resource::<TerrainPresentationDirty>().0);
		assert!(!world.resource::<MobStreamSuspended>().0);
		assert!(world.resource::<UrbanizationStreamingEnabled>().0);
		assert_eq!(forest_radius(&world), Some(WORLD_FOREST_STREAM_RADIUS));
		Ok(())
	}

	#[test]
	fn a_new_round_moves_the_patch() -> anyhow::Result<()> {
		let round = TrainingRound::new(7);
		let mut world = training_world(round);
		world.run_system_once(enter_training).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.resource_mut::<TerrainPresentationDirty>().0 = false;
		world
			.run_system_once(retarget_training_map)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<TerrainPresentationDirty>().0, "same round stays put");

		world.insert_resource(round.next_life());
		world
			.run_system_once(retarget_training_map)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<TerrainPresentationDirty>().0, "a new life keeps the map");
		assert_eq!(*world.resource::<TerrainCellLayout>(), round.layout());

		world.insert_resource(round.next());
		world
			.run_system_once(retarget_training_map)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<TerrainCellLayout>(), round.next().layout());
		assert!(world.resource::<TerrainPresentationDirty>().0);
		Ok(())
	}

	#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
	enum ShellFlow {
		#[default]
		Home,
		Loading,
	}

	#[derive(Resource, Clone, Copy)]
	struct ShellHop {
		flow: ShellFlow,
		mode: WorldMode,
	}

	fn route_shell(
		hop: Res<ShellHop>,
		mut flow: ResMut<NextState<ShellFlow>>,
		mut mode: ResMut<NextState<WorldMode>>,
	) {
		flow.set(hop.flow);
		NextState::set_if_neq(&mut mode, hop.mode);
	}

	fn hop(app: &mut App, flow: ShellFlow, mode: WorldMode) -> anyhow::Result<()> {
		app.insert_resource(ShellHop { flow, mode });
		app.world_mut()
			.run_system_once(route_shell)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		app.update();
		Ok(())
	}

	#[derive(Resource, Default)]
	struct ModeSeenOnLoading(Option<WorldMode>);

	fn note_mode_on_loading(mode: Res<State<WorldMode>>, mut seen: ResMut<ModeSeenOnLoading>) {
		seen.0 = Some(*mode.get());
	}

	#[derive(Resource, Default)]
	struct TrainingExits(u32);

	fn count_training_exit(mut exits: ResMut<TrainingExits>) {
		exits.0 += 1;
	}

	#[derive(Resource, Default)]
	struct SquadSeenInPostUpdate(bool);

	fn note_squad_before_last(
		squads: Query<(), With<TrainingBrawler>>,
		mut seen: ResMut<SquadSeenInPostUpdate>,
	) {
		seen.0 = !squads.is_empty();
	}

	#[test]
	fn a_shell_enter_retargets_on_that_update_and_startup_does_not() -> anyhow::Result<()> {
		let round = TrainingRound::new(7);
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin, WorldModePlugin));
		app.init_state::<ShellFlow>();
		app.insert_resource(round);
		app.init_resource::<AppliedTrainingMap>();
		app.insert_resource(MobStreamSuspended(false));
		app.insert_resource(VegetationLayerConfig::world_defaults());
		app.insert_resource(UrbanizationStreamingEnabled(true));
		app.insert_resource(playable_world_cell_layout());
		app.insert_resource(TerrainCoverage::PlayableWorld);
		app.insert_resource(TerrainPresentEnabled(false));
		app.insert_resource(TerrainLayoutPinned(false));
		app.insert_resource(TerrainPresentationDirty(false));
		app.insert_resource(TerrainPresentPending(false));
		app.insert_resource(TerrainPresentationAssets {
			config: TerrainConfig::new(42),
			material: Handle::default(),
			lod_bands: Vec::new(),
			outer_add_walls: true,
			fine_grid_max_radius: Some(WORLD_FINE_HALF_EXTENT_CELLS),
			macro_seam_half_extents: Vec::new(),
			macro_cell_min_size: None,
			macro_res_2: None,
		});
		register_world_mode_transitions(&mut app);
		app.init_resource::<ModeSeenOnLoading>();
		app.init_resource::<TrainingExits>();
		app.init_resource::<SquadSeenInPostUpdate>();
		app.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
			&TerrainConfig::new(42),
		)));
		app.insert_resource(DevelopmentEntryStore::default());
		app.insert_resource(PlayerSpawnXz(None));
		app.add_systems(OnEnter(ShellFlow::Loading), note_mode_on_loading);
		app.add_systems(OnExit(WorldMode::Training), count_training_exit);
		app.add_systems(PostUpdate, note_squad_before_last);
		app.add_systems(Last, clear_training_plaza);
		app.update();
		assert!(
			!app.world().resource::<TerrainPresentationDirty>().0,
			"the initial Discovery state does not retarget"
		);
		assert_eq!(*app.world().resource::<TerrainCoverage>(), TerrainCoverage::PlayableWorld);
		assert_eq!(forest_radius(app.world()), Some(WORLD_FOREST_STREAM_RADIUS));
		assert!(app.world().resource::<ModeSeenOnLoading>().0.is_none());
		assert_eq!(app.world().resource::<TrainingExits>().0, 0);

		hop(&mut app, ShellFlow::Loading, WorldMode::Training)?;
		assert_eq!(app.world().resource::<ModeSeenOnLoading>().0, Some(WorldMode::Training));
		assert_eq!(*app.world().resource::<State<WorldMode>>().get(), WorldMode::Training);
		assert_eq!(*app.world().resource::<TerrainCellLayout>(), round.layout());
		assert_eq!(*app.world().resource::<TerrainCoverage>(), TerrainCoverage::FinePatch);
		assert_eq!(forest_radius(app.world()), Some(0));
		assert!(app.world().resource::<TerrainPresentationDirty>().0);
		assert!(app.world().get_resource::<CombatScore>().is_some());
		assert_eq!(app.world().resource::<TrainingExits>().0, 0);

		app.world_mut().resource_mut::<TerrainPresentationDirty>().0 = false;
		app.insert_resource(round.next_life());
		hop(&mut app, ShellFlow::Loading, WorldMode::Training)?;
		assert!(
			!app.world().resource::<TerrainPresentationDirty>().0,
			"a new life on the same map does not move the patch"
		);
		assert_eq!(app.world().resource::<TrainingExits>().0, 0);
		assert!(app.world().get_resource::<CombatScore>().is_some());

		app.world_mut().resource_mut::<TerrainPresentationDirty>().0 = false;
		app.insert_resource(round.next());
		app.insert_resource(TrainingPlazaMounted(round));
		let squad = app.world_mut().spawn(TrainingBrawler).id();
		hop(&mut app, ShellFlow::Loading, WorldMode::Training)?;
		assert_eq!(*app.world().resource::<TerrainCellLayout>(), round.next().layout());
		assert!(app.world().resource::<TerrainPresentationDirty>().0);
		assert_eq!(app.world().resource::<TrainingExits>().0, 0);
		assert!(app.world().resource::<SquadSeenInPostUpdate>().0);
		assert!(app.world().get_entity(squad).is_err(), "a new map tears the squad down in Last");

		let current = *app.world().resource::<TrainingRound>();
		app.insert_resource(TrainingPlazaMounted(current));
		let squad = app.world_mut().spawn(TrainingBrawler).id();
		hop(&mut app, ShellFlow::Home, WorldMode::Discovery)?;
		assert_eq!(*app.world().resource::<State<WorldMode>>().get(), WorldMode::Discovery);
		assert_eq!(*app.world().resource::<TerrainCellLayout>(), playable_world_cell_layout());
		assert_eq!(forest_radius(app.world()), Some(WORLD_FOREST_STREAM_RADIUS));
		assert_eq!(app.world().resource::<TrainingExits>().0, 1);
		assert!(app.world().get_resource::<CombatScore>().is_none());
		assert!(app.world().resource::<SquadSeenInPostUpdate>().0);
		assert!(app.world().get_entity(squad).is_err(), "leaving despawns the squad in Last");
		Ok(())
	}

	#[test]
	fn each_training_session_keeps_a_fresh_score() -> anyhow::Result<()> {
		let mut world = World::new();
		let run = |world: &mut World, system: fn(Commands)| -> anyhow::Result<()> {
			world.run_system_once(system).map_err(|error| anyhow::anyhow!("{error:?}"))?;
			world.flush();
			Ok(())
		};
		run(&mut world, open_training_score)?;
		world.resource_mut::<CombatScore>().record_down();
		assert_eq!(world.resource::<CombatScore>().downs, 1, "a session keeps its tally");

		run(&mut world, close_training_score)?;
		assert!(world.get_resource::<CombatScore>().is_none(), "leaving drops the score");

		run(&mut world, open_training_score)?;
		assert_eq!(*world.resource::<CombatScore>(), CombatScore::default());
		Ok(())
	}

	#[test]
	fn the_enemy_count_follows_standing_training_fighters() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(State::new(WorldMode::Training));
		let squad = world.spawn(TrainingBrawler).id();
		let stranger = world.spawn_empty().id();
		let fighter = |mob| (MemberOf { mob, slot: 0 }, Health::from_max(10.0));
		let first = world.spawn(fighter(squad)).id();
		world.spawn(fighter(squad));
		world.spawn(fighter(stranger));
		let mut system = IntoSystem::into_system(count_training_enemies);
		system.initialize(&mut world);
		let mut run = |world: &mut World| -> anyhow::Result<Option<u32>> {
			system.run((), world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
			world.flush();
			Ok(world.get_resource::<LiveEnemies>().map(|live| live.0))
		};
		assert_eq!(run(&mut world)?, Some(2), "only training squads count");

		let mut first = world.entity_mut(first);
		let mut health =
			first.get_mut::<Health>().ok_or_else(|| anyhow::anyhow!("fighter lost health"))?;
		health.apply_damage(10.0);
		assert_eq!(run(&mut world)?, Some(1), "a dead fighter drops out");

		world.insert_resource(State::new(WorldMode::Discovery));
		assert_eq!(run(&mut world)?, None, "leaving hides the count");
		Ok(())
	}

	#[test]
	fn training_fill_is_a_pinned_raw_fine_patch() {
		let round = TrainingRound::new(3);
		let fill = TrainingFill::for_session(Some(round));
		let layout = &fill.layout;
		let site = round.site();
		let center = layout.region_center_xz();
		let cell = layout.cell_size;
		assert!((center.x - site.x as f32 * cell).abs() < 1e-3);
		assert!((center.z - site.y as f32 * cell).abs() < 1e-3);
		let side = (2 * TRAINING_FINE_HALF_EXTENT_CELLS) as u32;
		assert_eq!(layout.extents, UVec2::new(side, side));
		assert_eq!(fill.coverage, TerrainCoverage::FinePatch);
		assert!(fill.present);
		assert!(fill.pin_layout);
		assert!(!fill.urbanization);
		assert_eq!(fill.forest_stream_radius, 0);
	}

	#[test]
	fn plaza_waits_for_the_whole_patch() {
		let store = durham_terrain_models::TerrainEntryStore::default();
		let origin = IVec2::splat(-TRAINING_FINE_HALF_EXTENT_CELLS);
		let patch = fine_patch_cell_layout(TRAINING_FINE_HALF_EXTENT_CELLS, origin);
		assert!(!store.fills_layout(&patch));
	}

	#[test]
	fn discovery_fill_restores_the_playable_rings() {
		let fill = TrainingFill::for_session(None);
		assert_eq!(fill.layout, playable_world_cell_layout());
		assert_eq!(fill.coverage, TerrainCoverage::PlayableWorld);
		assert!(!fill.present);
		assert!(!fill.pin_layout);
		assert!(fill.urbanization);
		assert_eq!(fill.forest_stream_radius, WORLD_FOREST_STREAM_RADIUS);
	}

	#[test]
	fn rounds_are_reproducible_and_move_on() {
		let round = TrainingRound::new(1234);
		assert_eq!(round.next(), TrainingRound::new(1234).next());
		assert_ne!(round.next().seed, round.seed);
		let sites: std::collections::HashSet<IVec2> =
			std::iter::successors(Some(round), |round| Some(round.next()))
				.take(16)
				.map(TrainingRound::site)
				.collect();
		assert!(sites.len() > 8, "sixteen lives should not share a handful of sites");
		for site in sites {
			assert!(site.abs().max_element() <= TRAINING_SITE_RANGE_CELLS);
		}
	}

	#[test]
	fn site_rerolls_keep_the_seed_and_stop() {
		let mut round = TrainingRound::new(99);
		let first = round.site();
		round = round.reroll_site();
		assert_eq!(round.seed, 99);
		assert_ne!(round.site(), first);
		let tries = std::iter::successors(Some(TrainingRound::new(99)), |round| {
			(!round.site_exhausted()).then(|| round.reroll_site())
		})
		.count();
		assert_eq!(tries as u32, TRAINING_SITE_ATTEMPTS);
	}

	#[test]
	fn trainees_are_player_scale_and_never_a_saved_character() {
		let species: std::collections::HashSet<&str> = (0..40)
			.map(|seed| TrainingRound::new(seed).trainee())
			.inspect(|trainee| {
				assert!(trainee.key.starts_with("trainee-"));
				assert!(crozon_character_persist::CharacterId::from_hex(&trainee.key).is_none());
				assert!(trainee.inventory.primary_weapon().is_some());
			})
			.map(|trainee| trainee.appearance.species_id())
			.collect();
		let player_scale = ["braidman", "lero", "mygr", "tuberwaber", "wumbus"];
		assert!(species.iter().all(|id| player_scale.contains(id)), "{species:?}");
		assert!(species.len() > 2, "trainees should vary across rounds: {species:?}");
		assert_eq!(TrainingRound::new(5).trainee(), TrainingRound::new(5).trainee());
	}

	#[test]
	fn a_new_life_keeps_the_map_and_rolls_a_new_trainee() {
		let round = TrainingRound::new(77).reroll_site();
		let life = round.next_life();
		assert_eq!(life.map(), round.map());
		assert_eq!(life.site(), round.site());
		assert_eq!(life.development_seed(), round.development_seed());
		assert_eq!(life.mob_seed(), round.mob_seed());
		assert_ne!(life.trainee().key, round.trainee().key);
		let lives: std::collections::HashSet<_> =
			std::iter::successors(Some(round), |round| Some(round.next_life()))
				.take(12)
				.map(|life| format!("{:?}", life.trainee().appearance))
				.collect();
		assert!(lives.len() > 2, "lives on one map should vary the trainee");
		assert_ne!(round.next().map(), round.map());
	}
}
