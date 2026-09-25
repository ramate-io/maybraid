//! Training Ground is a seeded FinePatch of the Maybraid world, not Discovery's
//! moving rings.
//!
//! While [`TrainingGrounds`] is set, Durham presents a four-cell window pinned
//! on the [`TrainingRound`] site, hopscotch stays off, the forest shrinks to one
//! grove tile, and a Training pose is not written. [`crate::training_plaza`]
//! stamps one seeded Richmond development onto that patch — pads first, then
//! hosts — once the whole window exists.
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
use chico_vegetation_on_terrain_playground::PlaygroundConfig;
use combat_hud::CombatScore;
use crozon_character_items::{random_starter_loadout, Inventory, ItemRng};
use crozon_characters::species::{
	braidman::BraidmanConfig, lero::LeroConfig, mygr::MygrConfig, tuberwaber::TuberwaberConfig,
	wumbus::WumbusConfig,
};
use crozon_characters::CharacterAppearance;
use durham_terrain_models::{
	TerrainCellLayout, TerrainColliderSystems, TerrainCoverage, TerrainFillSystems,
	TerrainLayoutPinned, TerrainPresentEnabled, TerrainPresentPending, TerrainPresentationAssets,
	TerrainPresentationDirty, TerrainPresenterState, WORLD_FINE_HALF_EXTENT_CELLS,
	playable_world_cell_layout, retarget_presentation_assets, training_grounds_cell_layout_at,
};
use richmond_developments_on_terrain_playground::UrbanizationStreamingEnabled;

use crate::WorldPlayerLoadout;
use crate::control::{WorldSurfaceSet, update_world_surface_ready};
use crate::training_plaza::{
	clear_training_plaza, mount_training_plaza, park_on_training_site, promote_training_plaza,
	reseat_training_life, supersede_training_raw_terrain,
};

/// Training Ground session: patch retarget, the stamped plaza, and the
/// raw-to-padded hand-off under its courtyard.
pub(crate) struct TrainingGroundPlugin;

impl Plugin for TrainingGroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<TrainingGrounds>()
			.init_resource::<TrainingRound>()
			.add_message::<TrainingLifeEnded>()
			.add_systems(
				Update,
				(
					apply_training_grounds.before(TerrainFillSystems::Generate),
					park_on_training_site.after(apply_training_grounds),
					clear_training_terrain_present,
					mount_training_plaza.in_set(WorldSurfaceSet).after(update_world_surface_ready),
					(supersede_training_raw_terrain, promote_training_plaza)
						.chain()
						.after(TerrainColliderSystems::QueueMeshes),
					reseat_training_life.before(clear_training_plaza),
					clear_training_plaza,
					keep_training_score,
				),
			);
	}
}

/// Set by the game shell while Training Ground is the live session.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingGrounds(pub bool);

/// Forest stream radius used by [`PlaygroundConfig::world_defaults`].
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
		training_grounds_cell_layout_at(self.site())
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

/// Shrink Durham and the forest to the round's patch, and keep hopscotch off.
/// A new map (or site reroll) moves the patch. Restores the playable rings
/// when the shell clears [`TrainingGrounds`].
///
/// Runs in [`Update`] before generate so [`OnEnter`] shell look (after
/// [`PreUpdate`]) retargets the fill on the same frame.
pub(crate) fn apply_training_grounds(
	grounds: Res<TrainingGrounds>,
	round: Res<TrainingRound>,
	mut applied: Local<Option<TrainingMap>>,
	mut layout: ResMut<TerrainCellLayout>,
	mut coverage: ResMut<TerrainCoverage>,
	mut present: ResMut<TerrainPresentEnabled>,
	mut pinned: ResMut<TerrainLayoutPinned>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut assets: ResMut<TerrainPresentationAssets>,
	mut forest: Option<ResMut<PlaygroundConfig>>,
	mut urban: Option<ResMut<UrbanizationStreamingEnabled>>,
) {
	let target = grounds.0.then_some(*round);
	if *applied == target.map(TrainingRound::map) {
		return;
	}
	*applied = target.map(TrainingRound::map);
	let fill = TrainingFill::for_session(target);
	*layout = fill.layout.clone();
	*coverage = fill.coverage;
	present.0 = fill.present;
	pinned.0 = fill.pin_layout;
	dirty.0 = true;
	pending.0 = true;
	retarget_presentation_assets(&mut assets, fill.coverage, fill.terrain_radius);
	if let Some(urban) = urban.as_deref_mut() {
		urban.0 = fill.urbanization;
	}
	if let Some(config) = forest.as_deref_mut() {
		if let Some(spec) = config.forest.as_mut() {
			spec.stream_radius = fill.forest_stream_radius;
		}
		config.coverage = fill.coverage;
		config.terrain_radius = fill.terrain_radius;
	}
}

/// Each Training session keeps its own [`CombatScore`] across its rounds and
/// lives; leaving drops it, which hides the score panel.
pub(crate) fn keep_training_score(
	grounds: Res<TrainingGrounds>,
	mut kept: Local<bool>,
	mut commands: Commands,
) {
	if grounds.0 == *kept {
		return;
	}
	*kept = grounds.0;
	if grounds.0 {
		commands.insert_resource(CombatScore::default());
	} else {
		commands.remove_resource::<CombatScore>();
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
	use durham_terrain_models::{TerrainConfig, training_grounds_cell_layout};

	use super::*;

	fn training_world(round: TrainingRound) -> World {
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(true));
		world.insert_resource(round);
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

	#[test]
	fn apply_training_grounds_pins_the_round_site() -> anyhow::Result<()> {
		let round = TrainingRound::new(7);
		let mut world = training_world(round);
		world
			.run_system_once(apply_training_grounds)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<TerrainCellLayout>(), round.layout());
		assert_eq!(*world.resource::<TerrainCoverage>(), TerrainCoverage::FinePatch);
		assert!(world.resource::<TerrainPresentEnabled>().0);
		assert!(world.resource::<TerrainLayoutPinned>().0);
		assert!(world.resource::<TerrainPresentationDirty>().0);
		Ok(())
	}

	#[test]
	fn a_new_round_moves_the_patch() -> anyhow::Result<()> {
		let round = TrainingRound::new(7);
		let mut world = training_world(round);
		let mut system = IntoSystem::into_system(apply_training_grounds);
		system.initialize(&mut world);
		system.run((), &mut world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.resource_mut::<TerrainPresentationDirty>().0 = false;
		system.run((), &mut world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<TerrainPresentationDirty>().0, "same round stays put");

		world.insert_resource(round.next_life());
		system.run((), &mut world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<TerrainPresentationDirty>().0, "a new life keeps the map");

		world.insert_resource(round.next());
		system.run((), &mut world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<TerrainCellLayout>(), round.next().layout());
		assert!(world.resource::<TerrainPresentationDirty>().0);
		Ok(())
	}

	#[test]
	fn each_training_session_keeps_a_fresh_score() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(true));
		let mut system = IntoSystem::into_system(keep_training_score);
		system.initialize(&mut world);
		let mut run = |world: &mut World| -> anyhow::Result<()> {
			system.run((), world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
			world.flush();
			Ok(())
		};
		run(&mut world)?;
		world.resource_mut::<CombatScore>().record_down();
		run(&mut world)?;
		assert_eq!(world.resource::<CombatScore>().downs, 1, "a session keeps its tally");

		world.insert_resource(TrainingGrounds(false));
		run(&mut world)?;
		assert!(world.get_resource::<CombatScore>().is_none(), "leaving drops the score");

		world.insert_resource(TrainingGrounds(true));
		run(&mut world)?;
		assert_eq!(*world.resource::<CombatScore>(), CombatScore::default());
		Ok(())
	}

	#[test]
	fn training_fill_is_a_pinned_raw_fine_patch() {
		let round = TrainingRound::new(3);
		let fill = TrainingFill::for_session(Some(round));
		assert_eq!(fill.layout, training_grounds_cell_layout_at(round.site()));
		assert_eq!(fill.coverage, TerrainCoverage::FinePatch);
		assert!(fill.present);
		assert!(fill.pin_layout);
		assert!(!fill.urbanization);
		assert_eq!(fill.forest_stream_radius, 0);
	}

	#[test]
	fn plaza_waits_for_the_whole_patch() {
		let store = durham_terrain_models::TerrainEntryStore::default();
		assert!(!store.fills_layout(&training_grounds_cell_layout()));
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
