//! Downed world-player retirement and POI-based replacement.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_ragdoll::CharacterRagdollSystems;
use damage::{DamageSystems, DespawnAfter, Downed};
use durham::Durham;
use firearm_user::FirearmUser;
use firearms::WeaponTrigger;
use maybraid_character_controller::CharacterIntent;
use maybraid_input::{PadButton, VirtualPad};
use mob_characters::{LOCAL_POI, URBAN_POI, VEGETATION_POI};
use player::{CameraFollow, Player as MaybraidPlayer, PlayerUse};
use poi_intelligence::{
	mix_seed, NearbyFallback, NearbyQuery, PoiId, PoiInterest, PoiInterests, PoiRegistry,
	PoiSystems, DEFAULT_NEARBY_RADIUS,
};
use spotting_intelligence::SpotSubject;
use terrain_layer_model::{OnTerrain, TerrainView};
use threat_intelligence::{Affiliations, ThreatSubject};
use urbanization_layer_model::Urbanization;
use world_player::{
	player_position_above_surface, spawn_player_body, CharacterLocomotion, CharacterSpecies,
	ModePlayerPolicies, MoveWish, Player as VegetationPlayer, PlayerLifeEnded, PlayerLifeSet,
	RequestSetCharacter, RequestSetCharacterAppearance, RespawnOrigin,
};

use layer_stack::ActiveGenerationMode;

use crate::control::strip_world_player_motor;
use crate::map_view::WorldMapView;
use crate::weapon::WorldPlayerAppearanceRequested;
use crate::{WorldGameplayEnabled, WorldPlayerLoadout};

const MAP_OPEN_SECS: f32 = 0.18;

/// World-player downed duration, nearby POI scan, and replacement interests.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct WorldPlayerRespawnConfig {
	pub delay_secs: f32,
	/// After the glaze, wait this long for a map pick before `place_nearby`.
	pub pick_timeout_secs: f32,
	pub poi_radius: f32,
	pub fallback: NearbyFallback,
	pub interests: PoiInterests,
}

impl Default for WorldPlayerRespawnConfig {
	fn default() -> Self {
		Self {
			delay_secs: 4.0,
			pick_timeout_secs: 30.0,
			poi_radius: DEFAULT_NEARBY_RADIUS,
			fallback: NearbyFallback::new(60.0, 100.0),
			interests: default_player_respawn_interests(),
		}
	}
}

impl WorldPlayerRespawnConfig {
	/// Nearest POI outside the fallback ring's inner radius.
	pub fn nearby_query(&self) -> NearbyQuery {
		NearbyQuery::nearest_beyond(self.poi_radius, self.fallback.min_radius)
	}
}

#[derive(Debug)]
pub(crate) struct PendingPlayerRespawn {
	pub timer: Timer,
	pub pick_timer: Timer,
	pub death_at: Vec3,
	pub seed: u64,
	pub origin: RespawnOrigin,
	pub candidates: Vec<PoiId>,
	pub highlighted: Option<PoiId>,
	pub map_opened: bool,
}

#[derive(Resource, Default)]
pub(crate) struct WorldPlayerRespawnState {
	pub pending: Option<PendingPlayerRespawn>,
	generation: u64,
	last_poi: Option<PoiId>,
}

/// Discovery map picker confirmed this POI.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerChoseRespawnPoi {
	pub poi: PoiId,
}

#[derive(Component)]
struct PlayerDeathGlaze;

type DownedWorldPlayer<'a> = (
	Entity,
	&'a Transform,
	&'a mut LinearVelocity,
	Option<&'a FirearmUser>,
	Option<&'a InventoryUser>,
);

pub struct WorldPlayerLifecyclePlugin;

impl Plugin for WorldPlayerLifecyclePlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<WorldPlayerRespawnConfig>()
			.init_resource::<WorldPlayerRespawnState>()
			.init_resource::<WorldMapView>()
			.add_message::<PlayerLifeEnded>()
			.add_message::<PlayerChoseRespawnPoi>()
			.add_message::<CharacterIntent>()
			.configure_sets(Update, PlayerLifeSet::Resolve)
			.add_systems(Startup, spawn_player_death_glaze)
			.add_systems(
				PostUpdate,
				queue_downed_world_player
					.after(DamageSystems::Down)
					.after(CharacterRagdollSystems::Handoff),
			)
			.add_systems(
				Update,
				(
					drive_respawn_picker,
					respawn_world_player.after(drive_respawn_picker),
				)
					.after(PoiSystems::Index)
					.in_set(PlayerLifeSet::Resolve),
			)
			.add_systems(Update, sync_player_death_glaze.after(respawn_world_player));
	}
}

fn spawn_player_death_glaze(mut commands: Commands) {
	commands.spawn((
		Name::new("player-death-glaze"),
		PlayerDeathGlaze,
		Node {
			position_type: PositionType::Absolute,
			left: Val::Px(0.0),
			top: Val::Px(0.0),
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			..default()
		},
		BackgroundColor(death_glaze_color(0.0)),
		GlobalZIndex(i32::MAX - 4),
		Visibility::Hidden,
		Pickable::IGNORE,
	));
}

fn sync_player_death_glaze(
	state: Res<WorldPlayerRespawnState>,
	map: Option<Res<WorldMapView>>,
	mut glaze: Query<(&mut BackgroundColor, &mut Visibility), With<PlayerDeathGlaze>>,
) {
	let Ok((mut color, mut visibility)) = glaze.single_mut() else {
		return;
	};
	let Some(pending) = state.pending.as_ref() else {
		color.0 = death_glaze_color(0.0);
		*visibility = Visibility::Hidden;
		return;
	};
	let map_open = map.is_some_and(|map| map.open);
	color.0 = death_glaze_color(death_glaze_alpha(&pending.timer, map_open));
	*visibility = Visibility::Visible;
}

fn queue_downed_world_player(
	config: Res<WorldPlayerRespawnConfig>,
	mode: Option<Res<State<ActiveGenerationMode>>>,
	policies: Option<Res<ModePlayerPolicies>>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut commands: Commands,
	mut players: Query<DownedWorldPlayer<'_>, (With<VegetationPlayer>, Added<Downed>)>,
	mut triggers: Query<&mut WeaponTrigger>,
) {
	for (player, transform, mut velocity, firearm, inventory) in &mut players {
		state.generation = state.generation.wrapping_add(1);
		let seed = respawn_seed(state.generation, transform.translation);
		let now = mode.as_deref().and_then(|mode| mode.get().mode_id());
		let ends_life = policies.as_deref().is_some_and(|policies| policies.respawn_ends_life(now));
		state.pending = Some(PendingPlayerRespawn {
			timer: Timer::from_seconds(config.delay_secs.max(0.0), TimerMode::Once),
			pick_timer: Timer::from_seconds(config.pick_timeout_secs.max(0.0), TimerMode::Once),
			death_at: transform.translation,
			seed,
			origin: RespawnOrigin::began(now, ends_life),
			candidates: Vec::new(),
			highlighted: None,
			map_opened: false,
		});
		velocity.0 = Vec3::ZERO;
		if let Some(firearm) = firearm {
			if let Ok(mut trigger) = triggers.get_mut(firearm.held) {
				trigger.0 = false;
			}
			commands.entity(firearm.held).try_insert(DespawnAfter::seconds(0.0));
		}
		if let Some(inventory) = inventory {
			commands.entity(inventory.bag).try_despawn();
		}
		strip_world_player_motor(&mut commands, player);
		commands.entity(player).remove::<(
			VegetationPlayer,
			MaybraidPlayer,
			CameraFollow,
			PlayerUse,
			FirearmUser,
			InventoryUser,
			MoveWish,
			SpotSubject,
			ThreatSubject,
			Affiliations,
		)>();
	}
}

fn drive_respawn_picker(
	pad: Option<Res<VirtualPad>>,
	mut map: ResMut<WorldMapView>,
	registry: Res<PoiRegistry>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut chosen: MessageWriter<PlayerChoseRespawnPoi>,
	mut intents: MessageReader<CharacterIntent>,
) {
	if !map.open || !map.close_locked {
		return;
	}
	let Some(pending) = state.pending.as_mut() else {
		return;
	};
	if pending.candidates.is_empty() {
		return;
	}
	let mut step = 0i32;
	if let Some(pad) = pad.as_deref() {
		if pad.just_pressed(PadButton::DpadUp) {
			step -= 1;
		}
		if pad.just_pressed(PadButton::DpadDown) {
			step += 1;
		}
	}
	if step != 0 {
		let len = pending.candidates.len() as i32;
		let current = pending
			.highlighted
			.and_then(|id| pending.candidates.iter().position(|candidate| *candidate == id))
			.unwrap_or(0) as i32;
		let next = (current + step).rem_euclid(len) as usize;
		pending.highlighted = pending.candidates.get(next).copied();
		if let Some(record) = pending.highlighted.and_then(|id| registry.get(id)) {
			map.focus = record.position.xz();
		}
	} else if let Some(nearest) = nearest_candidate(&pending.candidates, &registry, map.focus) {
		pending.highlighted = Some(nearest);
	}
	let Some(poi) = pending.highlighted else {
		return;
	};
	if intents.read().any(|intent| {
		matches!(intent, CharacterIntent::Jump | CharacterIntent::StartInteraction)
	}) {
		chosen.write(PlayerChoseRespawnPoi { poi });
	}
}

/// A mode that keeps the player respawns near a POI. A mode whose policy ends
/// the life replaces the body where it fell and writes [`PlayerLifeEnded`].
/// Leaving that mode mid-respawn replaces the body at once and ends nothing.
#[allow(clippy::too_many_arguments)]
fn respawn_world_player(
	time: Res<Time>,
	gameplay: Res<WorldGameplayEnabled>,
	config: Res<WorldPlayerRespawnConfig>,
	registry: Res<PoiRegistry>,
	loadout: Option<Res<WorldPlayerLoadout>>,
	locomotion: Res<CharacterLocomotion>,
	surface: TerrainView<Urbanization<richmond::Richmond<OnTerrain<Durham>>>>,
	mode: Option<Res<State<ActiveGenerationMode>>>,
	mut ended: MessageWriter<PlayerLifeEnded>,
	mut chosen: MessageReader<PlayerChoseRespawnPoi>,
	live_player: Query<(), With<VegetationPlayer>>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut map: ResMut<WorldMapView>,
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let now = mode.as_deref().and_then(|mode| mode.get().mode_id());
	let abandoned = state.pending.as_ref().is_some_and(|pending| pending.origin.abandoned(now));
	if !gameplay.0 && !abandoned {
		return;
	}
	if !live_player.is_empty() {
		close_respawn_map(&mut map);
		state.pending = None;
		return;
	}
	let last_poi = state.last_poi;
	let Some(pending) = state.pending.as_mut() else {
		return;
	};
	pending.timer.tick(time.delta());

	if pending.origin.replace_in_place(now) {
		if !pending.timer.is_finished() && !pending.origin.replace_immediately(now) {
			return;
		}
		let death_at = pending.death_at;
		let origin = pending.origin;
		close_respawn_map(&mut map);
		state.pending = None;
		if origin.ends_life(now) {
			ended.write(PlayerLifeEnded);
		}
		finish_world_player_spawn(
			&mut commands,
			&mut meshes,
			&mut materials,
			locomotion.as_ref(),
			loadout.as_deref(),
			origin,
			now,
			player_position_above_surface(death_at),
		);
		return;
	}

	if !pending.map_opened
		&& (pending.timer.elapsed_secs() >= MAP_OPEN_SECS || pending.timer.is_finished())
	{
		open_respawn_picker(pending, &mut map, &registry, &config, last_poi);
	}

	pending.pick_timer.tick(time.delta());
	let picked = chosen.read().next().map(|msg| msg.poi);
	let timed_out = pending.map_opened && pending.pick_timer.is_finished();
	if picked.is_none() && !timed_out {
		return;
	}

	let death_at = pending.death_at;
	let seed = pending.seed;
	let origin = pending.origin;
	close_respawn_map(&mut map);
	state.pending = None;

	let position = match picked.and_then(|id| registry.get(id).copied()) {
		Some(record) => {
			state.last_poi = Some(record.id);
			player_position_above_surface(surface_at(record.position, &surface))
		}
		None => {
			let placed = registry.place_nearby(
				death_at,
				config.nearby_query(),
				&config.interests,
				state.last_poi,
				seed,
				config.fallback,
			);
			state.last_poi = placed.poi;
			player_position_above_surface(surface_at(placed.position, &surface))
		}
	};
	finish_world_player_spawn(
		&mut commands,
		&mut meshes,
		&mut materials,
		locomotion.as_ref(),
		loadout.as_deref(),
		origin,
		now,
		position,
	);
}

fn open_respawn_picker(
	pending: &mut PendingPlayerRespawn,
	map: &mut WorldMapView,
	registry: &PoiRegistry,
	config: &WorldPlayerRespawnConfig,
	last_poi: Option<PoiId>,
) {
	let excluded = last_poi.as_slice();
	let mut records = registry.nearby_in(
		pending.death_at,
		config.nearby_query(),
		&config.interests,
		excluded,
	);
	records.sort_by(|a, b| {
		xz_distance(pending.death_at, a.position)
			.total_cmp(&xz_distance(pending.death_at, b.position))
			.then_with(|| a.id.cmp(&b.id))
	});
	pending.candidates = records.iter().map(|record| record.id).collect();
	pending.highlighted = pending.candidates.first().copied();
	pending.map_opened = true;
	pending.pick_timer = Timer::from_seconds(config.pick_timeout_secs.max(0.0), TimerMode::Once);
	map.open_at(pending.death_at.xz(), true);
}

fn close_respawn_map(map: &mut WorldMapView) {
	map.close();
}

fn nearest_candidate(candidates: &[PoiId], registry: &PoiRegistry, focus: Vec2) -> Option<PoiId> {
	candidates
		.iter()
		.filter_map(|id| registry.get(*id).copied())
		.min_by(|a, b| {
			a.position.xz().distance(focus).total_cmp(&b.position.xz().distance(focus)).then_with(|| a.id.cmp(&b.id))
		})
		.map(|record| record.id)
}

fn xz_distance(a: Vec3, b: Vec3) -> f32 {
	(a.xz() - b.xz()).length()
}

fn surface_at(
	mut point: Vec3,
	surface: &TerrainView<Urbanization<richmond::Richmond<OnTerrain<Durham>>>>,
) -> Vec3 {
	let terrain_y = surface.height_or_fallback(point.xz());
	if terrain_y.is_finite() {
		point.y = terrain_y;
	}
	point
}

#[allow(clippy::too_many_arguments)]
fn finish_world_player_spawn(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	locomotion: &CharacterLocomotion,
	loadout: Option<&WorldPlayerLoadout>,
	origin: RespawnOrigin,
	now: Option<std::any::TypeId>,
	position: Vec3,
) {
	let player = spawn_player_body(commands, meshes, materials, locomotion, position);
	crate::control::apply_world_player_motor(commands, player);
	if let Some(loadout) = loadout {
		if !origin.ends_life(now) {
			commands.entity(player).insert(WorldPlayerAppearanceRequested);
		}
		commands.spawn(RequestSetCharacterAppearance { appearance: loadout.appearance.clone() });
	} else {
		commands.spawn(RequestSetCharacter { species: CharacterSpecies::Braidman });
	}
}

fn death_glaze_alpha(timer: &Timer, map_open: bool) -> f32 {
	if map_open {
		return 0.12;
	}
	let fade_in = (timer.elapsed_secs() / MAP_OPEN_SECS).clamp(0.0, 1.0);
	0.68 * fade_in
}

fn death_glaze_color(alpha: f32) -> Color {
	Color::srgba(0.2, 0.005, 0.025, alpha)
}

fn default_player_respawn_interests() -> PoiInterests {
	PoiInterests::new([
		PoiInterest::new(LOCAL_POI, 1.25),
		PoiInterest::new(URBAN_POI, 1.5),
		PoiInterest::new(VEGETATION_POI, 1.0),
	])
}

fn respawn_seed(generation: u64, death_at: Vec3) -> u64 {
	mix_seed(
		generation
			^ u64::from(death_at.x.to_bits()).rotate_left(11)
			^ u64::from(death_at.y.to_bits()).rotate_left(29)
			^ u64::from(death_at.z.to_bits()).rotate_left(47),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use durham::{TerrainCellLayout, TerrainEntryStore};
	use layer_stack::GenerationMode;
	use richmond::DevelopmentEntryStore;
	use urbanization_cells::UrbanizationIndex;
	use world_player::WorldBaseTerrain;

	#[test]
	fn fallback_respawn_moves_away_from_the_death_point() {
		let death = Vec3::new(10.0, 4.0, -5.0);
		let config = WorldPlayerRespawnConfig::default();
		let placed = poi_intelligence::place_nearby(
			None,
			death,
			config.nearby_query(),
			None,
			None,
			42,
			config.fallback,
		);
		let distance = (placed.position - death).xz().length();
		assert!((config.fallback.min_radius..=config.fallback.max_radius).contains(&distance));
		assert_eq!(placed.position.y, death.y);
		assert!(placed.poi.is_none());
	}

	#[test]
	fn player_respawn_prefers_urban_pois() {
		let interests = WorldPlayerRespawnConfig::default().interests;
		assert_eq!(interests.weight(URBAN_POI), Some(1.5));
		assert_eq!(interests.weight(LOCAL_POI), Some(1.25));
		assert!(interests.contains(VEGETATION_POI));
	}

	#[test]
	fn default_respawn_waits_four_seconds_and_scans_nearby() {
		let config = WorldPlayerRespawnConfig::default();
		assert_eq!(config.delay_secs, 4.0);
		assert_eq!(config.pick_timeout_secs, 30.0);
		assert_eq!(config.poi_radius, DEFAULT_NEARBY_RADIUS);
		assert_eq!(config.fallback, NearbyFallback::new(60.0, 100.0));
		assert_eq!(config.nearby_query().min_radius, config.fallback.min_radius);
	}

	#[test]
	fn death_glaze_fades_in_and_holds_a_map_vignette() {
		let mut timer = Timer::from_seconds(4.0, TimerMode::Once);
		assert_eq!(death_glaze_alpha(&timer, false), 0.0);
		timer.tick(std::time::Duration::from_secs_f32(0.5));
		assert!((death_glaze_alpha(&timer, false) - 0.68).abs() < 1e-5);
		timer.tick(std::time::Duration::from_secs_f32(3.4));
		assert!((death_glaze_alpha(&timer, false) - 0.68).abs() < 1e-5);
		assert!((death_glaze_alpha(&timer, true) - 0.12).abs() < 1e-5);
	}

	#[test]
	fn downed_player_is_retired_and_queues_a_replacement() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldPlayerRespawnConfig>();
		world.init_resource::<WorldPlayerRespawnState>();
		let held = world.spawn(WeaponTrigger(true)).id();
		let bag = world.spawn_empty().id();
		let player = world
			.spawn((
				VegetationPlayer,
				MaybraidPlayer,
				CameraFollow,
				Transform::from_xyz(3.0, 4.0, 5.0),
				LinearVelocity(Vec3::X),
				FirearmUser::holding(held),
				InventoryUser::carrying(bag),
				Downed { source: None, point: Vec3::ZERO, at: 0.0 },
			))
			.id();

		world
			.run_system_once(queue_downed_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.get::<VegetationPlayer>(player).is_none());
		assert!(world.get::<MaybraidPlayer>(player).is_none());
		assert_eq!(
			world.get::<LinearVelocity>(player).map(|velocity| velocity.0),
			Some(Vec3::ZERO)
		);
		assert_eq!(world.get::<WeaponTrigger>(held).map(|trigger| trigger.0), Some(false));
		assert!(world.get::<DespawnAfter>(held).is_some());
		assert!(!world.entities().contains(bag));
		let pending = &world.resource::<WorldPlayerRespawnState>().pending;
		assert!(pending.is_some());
		Ok(())
	}

	struct EndsLife;
	struct OtherMode;

	impl GenerationMode for EndsLife {}
	impl GenerationMode for OtherMode {}

	fn respawn_world(timer_secs: f32, gameplay: bool, still_there: bool) -> World {
		let mut world = World::new();
		world.insert_resource(WorldPlayerRespawnConfig { delay_secs: timer_secs, ..default() });
		let began = Some(std::any::TypeId::of::<EndsLife>());
		world.insert_resource(WorldPlayerRespawnState {
			pending: Some(PendingPlayerRespawn {
				timer: Timer::from_seconds(timer_secs, TimerMode::Once),
				pick_timer: Timer::from_seconds(30.0, TimerMode::Once),
				death_at: Vec3::new(3.0, 4.0, 5.0),
				seed: 7,
				origin: RespawnOrigin::began(began, true),
				candidates: Vec::new(),
				highlighted: None,
				map_opened: false,
			}),
			..default()
		});
		world.insert_resource(Time::<()>::default());
		world.insert_resource(WorldGameplayEnabled(gameplay));
		world.init_resource::<WorldMapView>();
		world.init_resource::<Messages<PlayerChoseRespawnPoi>>();
		world.init_resource::<PoiRegistry>();
		world.init_resource::<CharacterLocomotion>();
		world.init_resource::<TerrainEntryStore>();
		world.init_resource::<TerrainCellLayout>();
		world.init_resource::<DevelopmentEntryStore>();
		world.init_resource::<UrbanizationIndex>();
		world.insert_resource(WorldBaseTerrain(durham::BaseTerrainNoise::from_config(
			&durham::TerrainConfig::new(42),
		)));
		world.init_resource::<Assets<Mesh>>();
		world.init_resource::<Assets<StandardMaterial>>();
		world.init_resource::<Messages<PlayerLifeEnded>>();
		world.insert_resource(State::new(if still_there {
			ActiveGenerationMode::of::<EndsLife>()
		} else {
			ActiveGenerationMode::of::<OtherMode>()
		}));
		world.insert_resource(crate::WorldPlayerLoadout::new(
			"life",
			characters::CharacterAppearance::default(),
			character_items::Inventory::default(),
		));
		world
	}

	#[test]
	fn a_life_ending_respawn_stays_where_it_fell() -> anyhow::Result<()> {
		let mut world = respawn_world(0.0, true, true);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let ended: Vec<_> = world.resource_mut::<Messages<PlayerLifeEnded>>().drain().collect();
		assert_eq!(ended, vec![PlayerLifeEnded]);
		let mut bodies = world
			.query_filtered::<(&Transform, Has<WorldPlayerAppearanceRequested>), With<VegetationPlayer>>(
			);
		let (body, requested) = bodies.single(&world)?;
		assert_eq!(body.translation.xz(), Vec2::new(3.0, 5.0));
		assert!(!requested, "the next life's loadout dresses the body");
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		assert!(!world.resource::<WorldMapView>().open);
		Ok(())
	}

	#[test]
	fn leaving_the_mode_mid_respawn_replaces_the_body_at_once() -> anyhow::Result<()> {
		let mut world = respawn_world(4.0, false, true);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(
			world.resource::<WorldPlayerRespawnState>().pending.is_some(),
			"a paused death in the same mode keeps waiting"
		);

		world.insert_resource(State::new(ActiveGenerationMode::of::<OtherMode>()));
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let ended: Vec<_> = world.resource_mut::<Messages<PlayerLifeEnded>>().drain().collect();
		assert!(ended.is_empty(), "leaving ends no life");
		let mut bodies = world.query_filtered::<(), With<VegetationPlayer>>();
		assert_eq!(bodies.iter(&world).count(), 1);
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none(), "glaze clears");
		assert!(!world.resource::<WorldMapView>().open);
		Ok(())
	}

	fn discovery_respawn_world(elapsed: f32, pick_timeout: f32) -> World {
		let mut world = respawn_world(4.0, true, true);
		world.insert_resource(WorldPlayerRespawnConfig {
			delay_secs: 4.0,
			pick_timeout_secs: pick_timeout,
			..default()
		});
		let mut state = world.resource_mut::<WorldPlayerRespawnState>();
		let pending = state.pending.as_mut().unwrap();
		pending.origin = RespawnOrigin::began(Some(std::any::TypeId::of::<EndsLife>()), false);
		pending.timer.set_elapsed(std::time::Duration::from_secs_f32(elapsed));
		world
	}

	#[test]
	fn discovery_respawn_waits_for_a_map_pick() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2, 30.0);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.resource::<WorldMapView>().open);
		assert!(world.resource::<WorldMapView>().close_locked);
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_some());
		let mut bodies = world.query_filtered::<(), With<VegetationPlayer>>();
		assert_eq!(bodies.iter(&world).count(), 0);
		Ok(())
	}

	#[test]
	fn discovery_respawn_spawns_at_the_chosen_poi() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2, 30.0);
		let poi = world.spawn_empty().id();
		world.resource_mut::<PoiRegistry>().upsert(
			poi,
			poi_intelligence::Poi::new(PoiId(1), URBAN_POI).with_arrival_radius(8.0),
			Vec3::new(80.0, 4.0, 5.0),
			true,
			false,
		)?;
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.write_message(PlayerChoseRespawnPoi { poi: PoiId(1) });
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let mut bodies = world.query_filtered::<&Transform, With<VegetationPlayer>>();
		let body = bodies.single(&world)?;
		assert_eq!(body.translation.xz(), Vec2::new(80.0, 5.0));
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		assert!(!world.resource::<WorldMapView>().open);
		Ok(())
	}

	#[test]
	fn discovery_respawn_times_out_to_place_nearby() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2, 0.0);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let mut bodies = world.query_filtered::<&Transform, With<VegetationPlayer>>();
		let body = bodies.single(&world)?;
		let config = world.resource::<WorldPlayerRespawnConfig>().clone();
		let placed = poi_intelligence::place_nearby(
			None,
			Vec3::new(3.0, 4.0, 5.0),
			config.nearby_query(),
			None,
			None,
			7,
			config.fallback,
		);
		assert_eq!(body.translation.xz(), placed.position.xz());
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		assert!(!world.resource::<WorldMapView>().open);
		Ok(())
	}
}
