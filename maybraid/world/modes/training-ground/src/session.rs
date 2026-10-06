//! Training score, enemy count, trainee roll, and the life-ended signal.

use std::any::TypeId;

use bevy::prelude::*;
use character_items::{random_starter_loadout, Inventory, ItemRng};
use characters::species::{
	braidman::BraidmanConfig, lero::LeroConfig, mygr::MygrConfig, tuberwaber::TuberwaberConfig,
	wumbus::WumbusConfig,
};
use characters::CharacterAppearance;
use combat_hud::{CombatScore, LiveEnemies};
use damage::{Downed, Health};
use mob_intelligence::MemberOf;
use world_player::{ModePlayerPolicies, ModePlayerPolicy, PlayerLifeEnded};

use crate::{TrainingBrawler, TrainingRound};

/// A player-scale playable species in a rolled starter loadout. Never saved.
#[derive(Clone, Debug, PartialEq)]
pub struct TrainingTrainee {
	pub key: String,
	pub name: String,
	pub appearance: CharacterAppearance,
	pub inventory: Inventory,
}

/// A Training body respawned. The shell advances [`TrainingRound`] and loads
/// the next life in behind the loading screen.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingLifeEnded;

/// The shell reads [`TrainingLifeEnded`] after this set, in the same frame the life ends.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrainingSessionSet {
	LifeEnded,
}

pub fn training_trainee(round: TrainingRound) -> TrainingTrainee {
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
	TrainingTrainee { key, name: String::from("Trainee"), appearance, inventory }
}

pub(crate) fn training_player_policy() -> ModePlayerPolicy {
	ModePlayerPolicy {
		home: Vec2::ZERO,
		keep_waypoints: false,
		respawn_ends_life: true,
		pick_first_spawn: false,
	}
}

pub(crate) fn register_training_policy(app: &mut App) {
	let mut policies = app.world_mut().get_resource_or_insert_with(ModePlayerPolicies::default);
	policies.register(TypeId::of::<crate::TrainingGround>(), training_player_policy());
}

/// Each Training session keeps its own [`CombatScore`] across its rounds and
/// lives; leaving drops it, which hides the score panel.
pub(crate) fn open_training_score(mut commands: Commands) {
	commands.insert_resource(CombatScore::default());
}

pub(crate) fn close_training_score(mut commands: Commands) {
	commands.remove_resource::<CombatScore>();
}

/// Keep [`LiveEnemies`] at the number of training fighters still standing. A
/// downed fighter drops out until its squad respawns it.
pub(crate) fn count_training_enemies(
	live: Option<ResMut<LiveEnemies>>,
	squads: Query<(), With<TrainingBrawler>>,
	fighters: Query<(&MemberOf, &Health), Without<Downed>>,
	mut commands: Commands,
) {
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

pub(crate) fn clear_training_live_enemies(mut commands: Commands) {
	commands.remove_resource::<LiveEnemies>();
}

/// Forward a life that ended while Training is still the active mode.
pub(crate) fn note_training_life_ended(
	mut lives: MessageReader<PlayerLifeEnded>,
	mut ended: MessageWriter<TrainingLifeEnded>,
) {
	if lives.read().last().is_some() {
		ended.write(TrainingLifeEnded);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::plaza::{
		clear_stale_training_plaza, finish_training_leave, request_training_leave, TrainingPlaza,
	};
	use crate::TrainingGround;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::state::app::StatesPlugin;
	use durham::{
		fine_patch_cell_layout, playable_world_cell_layout, BaseTerrainNoise, Durham,
		DurhamTerrainConfig, TerrainCellLayout, TerrainConfig, TerrainCoverage,
		TerrainLayoutPinned, TerrainPresentPending, TerrainPresentationAssets,
		TerrainPresentationDirty, WorldBaseTerrain, WORLD_FINE_HALF_EXTENT_CELLS,
	};
	use layer_stack::{GenerationModePlugin, GenerationReadiness};
	use layer_stack::{LayerModeConfig, Scheme};
	use maybraid_game_mode_discover::Discovery;
	use terrain_layer_model::OnTerrain;
	use world_player::{PlayerSpawnXz, RespawnOrigin};

	use crate::TRAINING_FINE_HALF_EXTENT_CELLS;

	#[test]
	fn a_training_respawn_ends_the_life() {
		let training = Some(TypeId::of::<TrainingGround>());
		let origin = RespawnOrigin::began(training, training_player_policy().respawn_ends_life);
		assert!(origin.ends_life(training));
		assert!(!origin.abandoned(training));
		assert!(!origin.replace_immediately(training));
		assert!(origin.replace_in_place(training));
	}

	#[test]
	fn leaving_training_mid_respawn_replaces_the_body_at_once() {
		let training = Some(TypeId::of::<TrainingGround>());
		let discovery = Some(TypeId::of::<Discovery>());
		let origin = RespawnOrigin::began(training, training_player_policy().respawn_ends_life);
		assert!(origin.abandoned(discovery));
		assert!(origin.replace_immediately(discovery));
		assert!(!origin.ends_life(discovery), "leaving ends no life");
		assert!(origin.replace_in_place(discovery));
	}

	#[test]
	fn a_resolved_life_writes_training_life_ended() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<PlayerLifeEnded>>();
		world.init_resource::<Messages<TrainingLifeEnded>>();
		world.write_message(PlayerLifeEnded);
		world
			.run_system_once(note_training_life_ended)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let ended: Vec<_> = world.resource_mut::<Messages<TrainingLifeEnded>>().drain().collect();
		anyhow::ensure!(ended == vec![TrainingLifeEnded]);
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
		anyhow::ensure!(world.resource::<CombatScore>().downs == 1, "a session keeps its tally");

		run(&mut world, close_training_score)?;
		anyhow::ensure!(world.get_resource::<CombatScore>().is_none(), "leaving drops the score");

		run(&mut world, open_training_score)?;
		anyhow::ensure!(*world.resource::<CombatScore>() == CombatScore::default());
		Ok(())
	}

	#[test]
	fn the_enemy_count_follows_standing_training_fighters() -> anyhow::Result<()> {
		let mut world = World::new();
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
		anyhow::ensure!(run(&mut world)? == Some(2), "only training squads count");

		let mut first = world.entity_mut(first);
		let mut health = first
			.get_mut::<Health>()
			.ok_or_else(|| anyhow::anyhow!("fighter lost health"))?;
		health.apply_damage(10.0);
		anyhow::ensure!(run(&mut world)? == Some(1), "a dead fighter drops out");

		world
			.run_system_once(clear_training_live_enemies)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get_resource::<LiveEnemies>().is_none(), "leaving hides the count");
		Ok(())
	}

	#[test]
	fn plaza_waits_for_the_whole_patch() {
		use durham::TerrainStorage;

		let store = durham::HcsgStorage::default();
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
				assert!(character_persist::CharacterId::from_hex(&trainee.key).is_none());
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
		assert_eq!(life.map().readiness_key(), round.map().readiness_key());
		assert_ne!(round.next().map().readiness_key(), round.map().readiness_key());
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
		mode: layer_stack::ActiveGenerationMode,
	}

	fn route_shell(
		hop: Res<ShellHop>,
		mut flow: ResMut<NextState<ShellFlow>>,
		mut mode: ResMut<NextState<layer_stack::ActiveGenerationMode>>,
	) {
		flow.set(hop.flow);
		NextState::set_if_neq(&mut mode, hop.mode);
	}

	fn hop(
		app: &mut App,
		flow: ShellFlow,
		mode: layer_stack::ActiveGenerationMode,
	) -> anyhow::Result<()> {
		app.insert_resource(ShellHop { flow, mode });
		app.world_mut()
			.run_system_once(route_shell)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		app.update();
		Ok(())
	}

	#[derive(Resource, Default)]
	struct ModeSeenOnLoading(Option<layer_stack::ActiveGenerationMode>);

	fn note_mode_on_loading(
		mode: Res<State<layer_stack::ActiveGenerationMode>>,
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
		use layer_stack::in_generation_mode;
		let round = TrainingRound::new(7);
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Discovery>::initial(),
			GenerationModePlugin::<TrainingGround>::default(),
		));
		let playable = DurhamTerrainConfig::playable_world();
		let training = DurhamTerrainConfig::fine_patch(TRAINING_FINE_HALF_EXTENT_CELLS);
		<Discovery as Scheme<OnTerrain<Durham>>>::install(&mut app, &playable);
		<TrainingGround as Scheme<OnTerrain<Durham>>>::install(&mut app, &training);
		app.insert_resource(LayerModeConfig::<Discovery, OnTerrain<Durham>>::new(playable));
		app.insert_resource(LayerModeConfig::<TrainingGround, OnTerrain<Durham>>::new(training));
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
		app.add_systems(
			OnEnter(layer_stack::ActiveGenerationMode::of::<TrainingGround>()),
			open_training_score,
		)
		.add_systems(
			OnExit(layer_stack::ActiveGenerationMode::of::<TrainingGround>()),
			(close_training_score, request_training_leave, count_training_exit),
		)
		.add_systems(
			Last,
			(
				clear_stale_training_plaza.run_if(in_generation_mode::<TrainingGround>()),
				finish_training_leave.run_if(resource_exists::<crate::plaza::TrainingLeave>),
			),
		);
		app.init_resource::<ModeSeenOnLoading>();
		app.init_resource::<TrainingExits>();
		app.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(
			42,
		))));
		app.insert_resource(richmond::DevelopmentEntryStore::default());
		app.insert_resource(PlayerSpawnXz(None));
		let mut policies = ModePlayerPolicies::default();
		policies.register(
			TypeId::of::<Discovery>(),
			ModePlayerPolicy {
				home: Vec2::ZERO,
				keep_waypoints: true,
				respawn_ends_life: false,
				pick_first_spawn: true,
			},
		);
		app.insert_resource(policies);
		app.add_systems(OnEnter(ShellFlow::Loading), note_mode_on_loading);
		app.update();
		assert!(
			!app.world().resource::<TerrainPresentationDirty>().0,
			"the initial Discovery state does not retarget"
		);
		assert_eq!(*app.world().resource::<TerrainCoverage>(), TerrainCoverage::PlayableWorld);
		assert!(app.world().resource::<ModeSeenOnLoading>().0.is_none());
		assert_eq!(app.world().resource::<TrainingExits>().0, 0);

		hop(
			&mut app,
			ShellFlow::Loading,
			layer_stack::ActiveGenerationMode::of::<TrainingGround>(),
		)?;
		assert_eq!(
			app.world().resource::<ModeSeenOnLoading>().0,
			Some(layer_stack::ActiveGenerationMode::of::<TrainingGround>())
		);
		assert!(app
			.world()
			.resource::<State<layer_stack::ActiveGenerationMode>>()
			.get()
			.is::<TrainingGround>());
		assert_eq!(*app.world().resource::<TerrainCellLayout>(), round.layout());
		assert_eq!(*app.world().resource::<TerrainCoverage>(), TerrainCoverage::FinePatch);
		assert!(app.world().resource::<TerrainPresentationDirty>().0);
		assert!(app.world().get_resource::<CombatScore>().is_some());
		assert_eq!(app.world().resource::<TrainingExits>().0, 0);

		app.world_mut().resource_mut::<TerrainPresentationDirty>().0 = false;
		app.insert_resource(round.next_life());
		hop(
			&mut app,
			ShellFlow::Loading,
			layer_stack::ActiveGenerationMode::of::<TrainingGround>(),
		)?;
		assert!(
			!app.world().resource::<TerrainPresentationDirty>().0,
			"a new life on the same map does not move the patch"
		);
		assert_eq!(app.world().resource::<TrainingExits>().0, 0);
		assert!(app.world().get_resource::<CombatScore>().is_some());

		app.world_mut().resource_mut::<TerrainPresentationDirty>().0 = false;
		app.insert_resource(round.next());
		app.insert_resource(GenerationReadiness::new(round.map().readiness_key()));
		let wall = app.world_mut().spawn(TrainingPlaza).id();
		hop(
			&mut app,
			ShellFlow::Loading,
			layer_stack::ActiveGenerationMode::of::<TrainingGround>(),
		)?;
		assert_eq!(*app.world().resource::<TerrainCellLayout>(), round.next().layout());
		assert!(app.world().resource::<TerrainPresentationDirty>().0);
		assert_eq!(app.world().resource::<TrainingExits>().0, 0);
		assert!(app.world().get_entity(wall).is_err(), "a new map tears the wall down in Last");

		let current = *app.world().resource::<TrainingRound>();
		app.insert_resource(GenerationReadiness::new(current.map().readiness_key()));
		let wall = app.world_mut().spawn(TrainingPlaza).id();
		hop(&mut app, ShellFlow::Home, layer_stack::ActiveGenerationMode::of::<Discovery>())?;
		assert!(app
			.world()
			.resource::<State<layer_stack::ActiveGenerationMode>>()
			.get()
			.is::<Discovery>());
		assert_eq!(*app.world().resource::<TerrainCellLayout>(), playable_world_cell_layout());
		assert_eq!(app.world().resource::<TrainingExits>().0, 1);
		assert!(app.world().get_resource::<CombatScore>().is_none());
		assert!(app.world().get_entity(wall).is_err(), "leaving despawns the wall in Last");
		Ok(())
	}
}
