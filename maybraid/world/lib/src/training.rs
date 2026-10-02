//! Training Ground is a seeded FinePatch of the Maybraid world, not Discovery's
//! moving rings.
//!
//! While [`TrainingGround`](maybraid_game_mode_training_ground::TrainingGround) is
//! active, hopscotch stays off and a Training pose is not written. Terrain
//! layout lives on each mode's [`terrain_layer_model::BaseTerrainScheme`].
//! Training's urbanization scheme stamps one seeded Richmond development;
//! [`crate::training_plaza`] raises the wall and seats the player once padded
//! colliders exist. The training crate writes the roster as one mob cell.
//!
//! A Training respawn ends the life with [`TrainingLifeEnded`], and the shell
//! advances [`TrainingRound`]. A new round moves the patch, tears the plaza
//! down, and stamps the next one. A new life on the same map keeps the plaza
//! and only rolls the next trainee.
//!
//! Each session has one terrain collider owner: the padded urbanization
//! presenter. Training does not present raw [`durham::DurhamCells`].

use bevy::prelude::*;
use combat_hud::{CombatScore, LiveEnemies};
use crozon_character_items::{random_starter_loadout, Inventory, ItemRng};
use crozon_characters::species::{
	braidman::BraidmanConfig, lero::LeroConfig, mygr::MygrConfig, tuberwaber::TuberwaberConfig,
	wumbus::WumbusConfig,
};
use crozon_characters::CharacterAppearance;
use damage::{Downed, Health};
use durham::TerrainColliderSystems;
use maybraid_game_mode_training_ground::{TrainingBrawler, TrainingGround, TrainingRound};
use mob_intelligence::MemberOf;
use layer_stack::ActiveGenerationMode;

use crate::WorldPlayerLoadout;
use crate::control::{WorldSurfaceSet, update_world_surface_ready};
use crate::training_markers::{TrainingEnemyMarkersEnabled, sync_training_enemy_markers};
use crate::training_plaza::{
	clear_training_plaza, mount_training_plaza, park_on_training_site, promote_training_plaza,
	reseat_training_life,
};

/// Training Ground session: patch retarget and the walled plaza.
pub(crate) struct TrainingGroundPlugin;

impl Plugin for TrainingGroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<TrainingRound>()
			.init_resource::<TrainingEnemyMarkersEnabled>()
			.add_message::<TrainingLifeEnded>();
		register_generation_mode_transitions(app);
		app.add_systems(
			Update,
			(
				park_on_training_site,
				mount_training_plaza.in_set(WorldSurfaceSet).after(update_world_surface_ready),
				promote_training_plaza.after(TerrainColliderSystems::QueueMeshes),
				reseat_training_life,
				count_training_enemies,
				sync_training_enemy_markers,
			),
		)
		// Wall fixtures and the player anchor tear down in Last so combat
		// commands queued through PostUpdate still find their targets.
		.add_systems(Last, clear_training_plaza);
	}
}

/// Mode transitions. Plaza systems stay on the plugin so a headless transition
/// test can register this without the rest of the stack.
fn register_generation_mode_transitions(app: &mut App) {
	app.add_systems(
		OnEnter(ActiveGenerationMode::of::<TrainingGround>()),
		open_training_score,
	)
	.add_systems(OnExit(ActiveGenerationMode::of::<TrainingGround>()), close_training_score);
}

/// A Training body respawned. The shell advances [`TrainingRound`] and loads
/// the next life in behind the loading screen.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingLifeEnded;

/// A player-scale playable species in a rolled starter loadout. Never saved.
pub fn training_trainee(round: TrainingRound) -> WorldPlayerLoadout {
	let mut rng = ItemRng::from_seed(round.trainee_seed());
	let appearance = match rng.gen_index(5) {
		0 => CharacterAppearance::Braidman(BraidmanConfig::default_preview()),
		1 => CharacterAppearance::Lero(LeroConfig::default_preview()),
		2 => CharacterAppearance::Mygr(MygrConfig::default_preview()),
		3 => CharacterAppearance::Tuberwaber(TuberwaberConfig::default_preview()),
		_ => CharacterAppearance::Wumbus(WumbusConfig::default_preview()),
	};
	let inventory = Inventory::with_starter_outfit(random_starter_loadout(&mut rng));
	let key = format!("trainee-{:016x}-{}", round.seed, round.life());
	WorldPlayerLoadout::new(key, appearance, inventory).with_name("Trainee")
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
	mode: Res<State<ActiveGenerationMode>>,
	live: Option<ResMut<LiveEnemies>>,
	squads: Query<(), With<TrainingBrawler>>,
	fighters: Query<(&MemberOf, &Health), Without<Downed>>,
	mut commands: Commands,
) {
	if !mode.get().is::<TrainingGround>() {
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

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::state::app::StatesPlugin;
	use durham::{
		fine_patch_cell_layout, playable_world_cell_layout, BaseTerrainNoise, Durham,
		DurhamTerrainConfig, TerrainCellLayout, TerrainConfig, TerrainCoverage,
		TerrainLayoutPinned, TerrainPresentPending, TerrainPresentationAssets,
		TerrainPresentationDirty, WorldBaseTerrain, WORLD_FINE_HALF_EXTENT_CELLS,
	};
	use maybraid_game_mode_training_ground::TRAINING_FINE_HALF_EXTENT_CELLS;
	use richmond::DevelopmentEntryStore;

	use crate::PlayerSpawnXz;
	use crate::training_plaza::TrainingPlazaMounted;
	use maybraid_game_mode_discover::Discovery;
	use terrain_layer_model::{BaseTerrainModeConfig, BaseTerrainScheme};
use layer_stack::GenerationModePlugin;
	use super::*;

	#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
	enum ShellFlow {
		#[default]
		Home,
		Loading,
	}

	#[derive(Resource, Clone, Copy)]
	struct ShellHop {
		flow: ShellFlow,
		mode: ActiveGenerationMode,
	}

	fn route_shell(
		hop: Res<ShellHop>,
		mut flow: ResMut<NextState<ShellFlow>>,
		mut mode: ResMut<NextState<ActiveGenerationMode>>,
	) {
		flow.set(hop.flow);
		NextState::set_if_neq(&mut mode, hop.mode);
	}

	fn hop(app: &mut App, flow: ShellFlow, mode: ActiveGenerationMode) -> anyhow::Result<()> {
		app.insert_resource(ShellHop { flow, mode });
		app.world_mut()
			.run_system_once(route_shell)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		app.update();
		Ok(())
	}

	#[derive(Resource, Default)]
	struct ModeSeenOnLoading(Option<ActiveGenerationMode>);

	fn note_mode_on_loading(
		mode: Res<State<ActiveGenerationMode>>,
		mut seen: ResMut<ModeSeenOnLoading>,
	) {
		seen.0 = Some(*mode.get());
	}

	#[derive(Resource, Default)]
	struct TrainingExits(u32);

	fn count_training_exit(mut exits: ResMut<TrainingExits>) {
		exits.0 += 1;
	}

	#[test]
	fn a_shell_enter_retargets_on_that_update_and_startup_does_not() -> anyhow::Result<()> {
		let round = TrainingRound::new(7);
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Discovery>::initial(),
			GenerationModePlugin::<TrainingGround>::default(),
		));
		// Durham `install_generation` needs a render world (`Messages`).
		let playable = DurhamTerrainConfig::playable_world();
		let training = DurhamTerrainConfig::fine_patch(TRAINING_FINE_HALF_EXTENT_CELLS);
		Discovery::install(&mut app, &playable);
		TrainingGround::install(&mut app, &training);
		app.insert_resource(BaseTerrainModeConfig::<Discovery, Durham>::new(playable));
		app.insert_resource(BaseTerrainModeConfig::<TrainingGround, Durham>::new(training));
		app.init_state::<ShellFlow>();
		app.insert_resource(round);
		app.insert_resource(playable_world_cell_layout());
		app.insert_resource(TerrainCoverage::PlayableWorld);
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
		register_generation_mode_transitions(&mut app);
		app.init_resource::<ModeSeenOnLoading>();
		app.init_resource::<TrainingExits>();
		app.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
			&TerrainConfig::new(42),
		)));
		app.insert_resource(DevelopmentEntryStore::default());
		app.insert_resource(PlayerSpawnXz(None));
		app.add_systems(OnEnter(ShellFlow::Loading), note_mode_on_loading);
		app.add_systems(OnExit(ActiveGenerationMode::of::<TrainingGround>()), count_training_exit);
		app.add_systems(Last, clear_training_plaza);
		app.update();
		assert!(
			!app.world().resource::<TerrainPresentationDirty>().0,
			"the initial Discovery state does not retarget"
		);
		assert_eq!(*app.world().resource::<TerrainCoverage>(), TerrainCoverage::PlayableWorld);
		assert!(app.world().resource::<ModeSeenOnLoading>().0.is_none());
		assert_eq!(app.world().resource::<TrainingExits>().0, 0);

		hop(&mut app, ShellFlow::Loading, ActiveGenerationMode::of::<TrainingGround>())?;
		assert_eq!(
			app.world().resource::<ModeSeenOnLoading>().0,
			Some(ActiveGenerationMode::of::<TrainingGround>())
		);
		assert!(app.world().resource::<State<ActiveGenerationMode>>().get().is::<TrainingGround>());
		assert_eq!(*app.world().resource::<TerrainCellLayout>(), round.layout());
		assert_eq!(*app.world().resource::<TerrainCoverage>(), TerrainCoverage::FinePatch);
		assert!(app.world().resource::<TerrainPresentationDirty>().0);
		assert!(app.world().get_resource::<CombatScore>().is_some());
		assert_eq!(app.world().resource::<TrainingExits>().0, 0);

		app.world_mut().resource_mut::<TerrainPresentationDirty>().0 = false;
		app.insert_resource(round.next_life());
		hop(&mut app, ShellFlow::Loading, ActiveGenerationMode::of::<TrainingGround>())?;
		assert!(
			!app.world().resource::<TerrainPresentationDirty>().0,
			"a new life on the same map does not move the patch"
		);
		assert_eq!(app.world().resource::<TrainingExits>().0, 0);
		assert!(app.world().get_resource::<CombatScore>().is_some());

		app.world_mut().resource_mut::<TerrainPresentationDirty>().0 = false;
		app.insert_resource(round.next());
		app.insert_resource(TrainingPlazaMounted(round));
		let wall = app.world_mut().spawn(crate::training_plaza::TrainingPlaza).id();
		hop(&mut app, ShellFlow::Loading, ActiveGenerationMode::of::<TrainingGround>())?;
		assert_eq!(*app.world().resource::<TerrainCellLayout>(), round.next().layout());
		assert!(app.world().resource::<TerrainPresentationDirty>().0);
		assert_eq!(app.world().resource::<TrainingExits>().0, 0);
		assert!(app.world().get_entity(wall).is_err(), "a new map tears the wall down in Last");

		let current = *app.world().resource::<TrainingRound>();
		app.insert_resource(TrainingPlazaMounted(current));
		let wall = app.world_mut().spawn(crate::training_plaza::TrainingPlaza).id();
		hop(&mut app, ShellFlow::Home, ActiveGenerationMode::of::<Discovery>())?;
		assert!(app.world().resource::<State<ActiveGenerationMode>>().get().is::<Discovery>());
		assert_eq!(*app.world().resource::<TerrainCellLayout>(), playable_world_cell_layout());
		assert_eq!(app.world().resource::<TrainingExits>().0, 1);
		assert!(app.world().get_resource::<CombatScore>().is_none());
		assert!(app.world().get_entity(wall).is_err(), "leaving despawns the wall in Last");
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
		world.insert_resource(State::new(ActiveGenerationMode::of::<TrainingGround>()));
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

		world.insert_resource(State::new(ActiveGenerationMode::of::<Discovery>()));
		assert_eq!(run(&mut world)?, None, "leaving hides the count");
		Ok(())
	}

	#[test]
	fn plaza_waits_for_the_whole_patch() {
		let store = durham::TerrainEntryStore::default();
		let origin = IVec2::splat(-TRAINING_FINE_HALF_EXTENT_CELLS);
		let patch = fine_patch_cell_layout(TRAINING_FINE_HALF_EXTENT_CELLS, origin);
		assert!(!store.fills_layout(&patch));
	}

	#[test]
	fn trainees_are_player_scale_and_never_a_saved_character() {
		let species: std::collections::HashSet<&str> = (0..40)
			.map(|seed| training_trainee(TrainingRound::new(seed)))
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
		assert_eq!(
			training_trainee(TrainingRound::new(5)),
			training_trainee(TrainingRound::new(5))
		);
	}

	#[test]
	fn a_new_life_keeps_the_map_and_rolls_a_new_trainee() {
		let round = TrainingRound::new(77).reroll_site();
		let life = round.next_life();
		assert_eq!(life.map(), round.map());
		assert_eq!(life.site(), round.site());
		assert_eq!(life.development_seed(), round.development_seed());
		assert_eq!(life.mob_seed(), round.mob_seed());
		assert_ne!(training_trainee(life).key, training_trainee(round).key);
		let lives: std::collections::HashSet<_> =
			std::iter::successors(Some(round), |round| Some(round.next_life()))
				.take(12)
				.map(|life| format!("{:?}", training_trainee(life).appearance))
				.collect();
		assert!(lives.len() > 2, "lives on one map should vary the trainee");
		assert_ne!(round.next().map(), round.map());
	}
}
