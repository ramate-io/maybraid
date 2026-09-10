//! Claimable world stashes for NPC death loot, player drops, and chests.
//!
//! One host type ([`WorldStash`]) holds a bag via [`InventoryUser`]. Claim is
//! take-all on [`CharacterIntent::StartInteraction`] (pad **X**) inside
//! [`StashPolicy::claim_radius`]. Drop dumps the **whole bag** on
//! [`CharacterIntent::Inventory`] (Select) — not only worn / queued items.
//!
//! Loot TTL is [`StashPolicy::loot_secs`] (default 60 s), independent of
//! mob corpse lifetime (4 s). Persistent chests omit [`DespawnAfter`].
//! Absorb never auto-equips.

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use chico_vegetation_on_terrain_playground::Player as VegetationPlayer;
use crozon_character_items::{Inventory, InventoryItem, InventorySlot};
use crozon_inventory_user::{spawn_bag, InventoryUser};
use damage::{DamageSystems, DespawnAfter, Downed};
use firearm_user::{FirearmUser, GeneratedFirearm};
use firearms::{firearm_bounds, spawn_firearm_components};
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use player::PlayerUse;

use crate::control::WorldGameplayEnabled;

/// Default unclaimed-loot lifetime. Independent of the 4 s corpse clock.
pub const DEFAULT_LOOT_SECS: f32 = 60.0;

/// Default interact radius for take-all claim.
pub const DEFAULT_CLAIM_RADIUS: f32 = 2.5;

/// Ground pile, player drop, or authored chest. The bag is a related entity.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldStash;

/// Per-stash claim / lifetime knobs.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct StashPolicy {
	pub claim_radius: f32,
	/// When true, claim empties the bag but keeps the host (chests).
	pub persist: bool,
	/// Ephemeral TTL. Ignored when [`Self::persist`] is set.
	pub loot_secs: f32,
}

impl Default for StashPolicy {
	fn default() -> Self {
		Self::ephemeral(DEFAULT_LOOT_SECS)
	}
}

impl StashPolicy {
	pub fn ephemeral(loot_secs: f32) -> Self {
		Self { claim_radius: DEFAULT_CLAIM_RADIUS, persist: false, loot_secs }
	}

	pub fn chest() -> Self {
		Self { claim_radius: DEFAULT_CLAIM_RADIUS, persist: true, loot_secs: 0.0 }
	}
}

/// World defaults for death piles and player drops.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct WorldStashSettings {
	pub loot_secs: f32,
	pub claim_radius: f32,
}

impl Default for WorldStashSettings {
	fn default() -> Self {
		Self { loot_secs: DEFAULT_LOOT_SECS, claim_radius: DEFAULT_CLAIM_RADIUS }
	}
}

impl WorldStashSettings {
	pub fn ephemeral_policy(&self) -> StashPolicy {
		StashPolicy { claim_radius: self.claim_radius, persist: false, loot_secs: self.loot_secs }
	}

	pub fn chest_policy(&self) -> StashPolicy {
		StashPolicy { claim_radius: self.claim_radius, persist: true, loot_secs: 0.0 }
	}
}

/// Marker that worn clothing / queued weapons have been spawned on the pile.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StashVisual;

/// Child spawned for a toggled clothing piece or queued firearm.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StashDisplayedItem {
	pub slot: InventorySlot,
}

pub struct WorldStashPlugin;

impl Plugin for WorldStashPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<WorldStashSettings>().add_systems(
			Update,
			(
				claim_nearby_stashes
					.after(CharacterControlSystems)
					.run_if(resource_equals(WorldGameplayEnabled(true))),
				drop_player_inventory
					.after(CharacterControlSystems)
					.run_if(resource_equals(WorldGameplayEnabled(true))),
			),
		);
		app.add_systems(PostUpdate, detach_downed_npc_loot.after(DamageSystems::Down));
	}
}

/// Spawn a stash at `transform` when `inventory` is non-empty.
///
/// The bag is parented so [`DespawnAfter`] on the host also removes loot.
/// Toggled clothing and queued weapons get [`StashDisplayedItem`] children.
pub fn spawn_world_stash(
	commands: &mut Commands,
	transform: Transform,
	inventory: Inventory,
	policy: StashPolicy,
	assets: Option<&AssetServer>,
) -> Option<Entity> {
	if inventory.items.is_empty() {
		return None;
	}
	let host = commands
		.spawn((
			Name::new(if policy.persist { "world-chest" } else { "world-stash" }),
			WorldStash,
			policy,
			StashVisual,
			transform,
			Visibility::default(),
		))
		.id();
	let bag = spawn_bag(commands, host, inventory.clone());
	commands.entity(bag).insert(ChildOf(host));
	if !policy.persist {
		commands.entity(host).insert(DespawnAfter::seconds(policy.loot_secs));
	}
	attach_stash_visuals(commands, host, &inventory, assets);
	Some(host)
}

fn attach_stash_visuals(
	commands: &mut Commands,
	host: Entity,
	inventory: &Inventory,
	assets: Option<&AssetServer>,
) {
	let mut pile = 0usize;
	for &index in &inventory.clothing {
		let Some(item) = inventory.items.get(index) else {
			continue;
		};
		spawn_displayed_item(
			commands,
			host,
			item,
			StashDisplayedItem { slot: InventorySlot::Clothing },
			pile_offset(pile),
			assets,
		);
		pile += 1;
	}
	for &index in &inventory.weapons {
		let Some(item) = inventory.items.get(index) else {
			continue;
		};
		spawn_displayed_item(
			commands,
			host,
			item,
			StashDisplayedItem { slot: InventorySlot::Weapons },
			pile_offset(pile),
			assets,
		);
		pile += 1;
	}
}

fn pile_offset(index: usize) -> Transform {
	let angle = index as f32 * 0.7;
	Transform::from_xyz(angle.cos() * 0.28, 0.08, angle.sin() * 0.28)
		.with_rotation(Quat::from_rotation_y(angle) * Quat::from_rotation_x(-0.35))
}

fn spawn_displayed_item(
	commands: &mut Commands,
	host: Entity,
	item: &InventoryItem,
	displayed: StashDisplayedItem,
	transform: Transform,
	assets: Option<&AssetServer>,
) {
	let visual = commands
		.spawn((
			Name::new(format!("stash-{}", item.label())),
			displayed,
			transform,
			Visibility::default(),
			ChildOf(host),
		))
		.id();
	if let Some(assets) = assets {
		match displayed.slot {
			InventorySlot::Clothing => {
				commands.entity(visual).insert(WorldAssetRoot(
					assets.load(GltfAssetLabel::Scene(0).from_asset(item.path())),
				));
			}
			InventorySlot::Weapons => {
				if let Some(spec) = item.firearm_spec() {
					let kit = GeneratedFirearm::from_spec(spec);
					let bounds = firearm_bounds(&kit);
					for entity in spawn_firearm_components(commands, &kit, transform, bounds) {
						commands.entity(entity).insert((
							Name::new(format!("stash-kit-{}", item.label())),
							displayed,
							ChildOf(host),
						));
					}
				}
			}
		}
	}
}

fn despawn_displayed_items(commands: &mut Commands, displayed: &[Entity]) {
	for entity in displayed {
		commands.entity(*entity).try_despawn();
	}
}

type DownedNpcLoot<'a> = (Entity, &'a Downed, &'a InventoryUser, Option<&'a FirearmUser>);

fn detach_downed_npc_loot(
	settings: Res<WorldStashSettings>,
	mut commands: Commands,
	assets: Option<Res<AssetServer>>,
	downed: Query<DownedNpcLoot<'_>, (Added<Downed>, Without<VegetationPlayer>)>,
	mut bags: Query<&mut Inventory>,
) {
	let assets = assets.as_deref();
	for (body, downed, user, firearm) in &downed {
		let loot = match bags.get_mut(user.bag) {
			Ok(mut bag) => bag.take_all(),
			Err(_) => Inventory::default(),
		};
		commands.entity(user.bag).try_despawn();
		commands.entity(body).remove::<InventoryUser>();
		if let Some(firearm) = firearm {
			commands.entity(firearm.held).try_despawn();
			commands.entity(body).remove::<(FirearmUser, PlayerUse)>();
		}
		let mut policy = settings.ephemeral_policy();
		policy.loot_secs = settings.loot_secs;
		spawn_world_stash(
			&mut commands,
			Transform::from_translation(downed.point),
			loot,
			policy,
			assets,
		);
	}
}

fn claim_nearby_stashes(
	mut intents: MessageReader<CharacterIntent>,
	mut commands: Commands,
	players: Query<(&Transform, &InventoryUser), With<VegetationPlayer>>,
	stashes: Query<(Entity, &Transform, &InventoryUser, &StashPolicy), With<WorldStash>>,
	displayed: Query<(Entity, &ChildOf), With<StashDisplayedItem>>,
	mut bags: Query<&mut Inventory>,
) {
	if !intents.read().any(|intent| matches!(intent, CharacterIntent::StartInteraction)) {
		return;
	}
	for (player_transform, player_user) in &players {
		let Some((stash, stash_bag, policy)) =
			nearest_stash_in_radius(player_transform.translation, &stashes)
		else {
			continue;
		};
		let Ok(mut source) = bags.get_mut(stash_bag) else {
			continue;
		};
		let loot = source.take_all();
		let Ok(mut player_bag) = bags.get_mut(player_user.bag) else {
			continue;
		};
		player_bag.absorb(loot);
		let visual: Vec<Entity> = displayed
			.iter()
			.filter(|(_, child)| child.parent() == stash)
			.map(|(entity, _)| entity)
			.collect();
		despawn_displayed_items(&mut commands, &visual);
		if policy.persist {
			continue;
		}
		commands.entity(stash_bag).try_despawn();
		commands.entity(stash).try_despawn();
	}
}

fn nearest_stash_in_radius(
	origin: Vec3,
	stashes: &Query<(Entity, &Transform, &InventoryUser, &StashPolicy), With<WorldStash>>,
) -> Option<(Entity, Entity, StashPolicy)> {
	stashes
		.iter()
		.filter_map(|(entity, transform, user, policy)| {
			let distance = transform.translation.distance(origin);
			(distance <= policy.claim_radius).then_some((distance, entity, user.bag, *policy))
		})
		.min_by(|a, b| a.0.total_cmp(&b.0))
		.map(|(_, entity, bag, policy)| (entity, bag, policy))
}

type DroppingPlayer<'a> = (Entity, &'a Transform, &'a InventoryUser, Option<&'a FirearmUser>);

fn drop_player_inventory(
	settings: Res<WorldStashSettings>,
	mut intents: MessageReader<CharacterIntent>,
	mut commands: Commands,
	assets: Option<Res<AssetServer>>,
	players: Query<DroppingPlayer<'_>, With<VegetationPlayer>>,
	mut bags: Query<&mut Inventory>,
) {
	if !intents.read().any(|intent| matches!(intent, CharacterIntent::Inventory)) {
		return;
	}
	let assets = assets.as_deref();
	for (player, transform, user, firearm) in &players {
		let Ok(mut bag) = bags.get_mut(user.bag) else {
			continue;
		};
		let loot = bag.take_all();
		if let Some(firearm) = firearm {
			commands.entity(firearm.held).try_despawn();
			commands.entity(player).remove::<(FirearmUser, PlayerUse)>();
		}
		spawn_world_stash(
			&mut commands,
			Transform::from_translation(transform.translation),
			loot,
			settings.ephemeral_policy(),
			assets,
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use chico_vegetation_on_terrain_playground::Player as VegetationPlayer;
	use crozon_character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, InventoryItem, ItemColor,
	};
	use damage::tick_queued_despawns;
	use player::Npc;

	fn mixed_bag() -> Inventory {
		Inventory {
			items: vec![
				InventoryItem::clothing(
					ClothingMesh::Pants,
					ClothingMaterial::Cloth,
					ItemColor::Natural,
				),
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::clothing(
					ClothingMesh::Robe,
					ClothingMaterial::Cloth,
					ItemColor::Cool,
				),
			],
			clothing: vec![0],
			weapons: vec![1],
		}
	}

	fn spawn_stash_system(
		transform: Transform,
		inventory: Inventory,
		policy: StashPolicy,
	) -> impl FnMut(Commands) -> Option<Entity> {
		move |mut commands| {
			spawn_world_stash(&mut commands, transform, inventory.clone(), policy, None)
		}
	}

	#[test]
	fn default_loot_ttl_is_sixty_seconds_not_corpse_time() {
		assert_eq!(DEFAULT_LOOT_SECS, 60.0);
		assert_eq!(StashPolicy::default().loot_secs, 60.0);
		assert!(!StashPolicy::default().persist);
		assert!(StashPolicy::chest().persist);
		assert!(WorldStashSettings::default().loot_secs > 4.0);
	}

	#[test]
	fn empty_bag_does_not_spawn_a_stash() -> anyhow::Result<()> {
		let mut world = World::new();
		let spawned = world
			.run_system_once(spawn_stash_system(
				Transform::from_xyz(1.0, 0.0, 0.0),
				Inventory::default(),
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(spawned.is_none());
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 0);
		Ok(())
	}

	#[test]
	fn toggled_items_are_displayed_and_filler_is_not() -> anyhow::Result<()> {
		let mut world = World::new();
		let stash = world
			.run_system_once(spawn_stash_system(
				Transform::from_translation(Vec3::ZERO),
				mixed_bag(),
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.ok_or_else(|| anyhow::anyhow!("expected stash"))?;

		let displayed: Vec<_> = world
			.query::<(&StashDisplayedItem, &Name)>()
			.iter(&world)
			.map(|(item, name)| (*item, name.as_str().to_string()))
			.collect();
		assert_eq!(displayed.len(), 2);
		assert!(displayed.iter().any(|(item, name)| {
			item.slot == InventorySlot::Clothing && name.contains("pants")
		}));
		assert!(displayed.iter().any(|(item, name)| {
			item.slot == InventorySlot::Weapons && name.contains("bullpup")
		}));
		assert!(!displayed.iter().any(|(_, name)| name.contains("robe")));

		let user = world.get::<InventoryUser>(stash).ok_or_else(|| anyhow::anyhow!("stash bag"))?;
		let bag = world.get::<Inventory>(user.bag).ok_or_else(|| anyhow::anyhow!("inventory"))?;
		assert_eq!(bag.items.len(), 3);
		assert_eq!(bag.clothing, vec![0]);
		assert_eq!(bag.weapons, vec![1]);
		Ok(())
	}

	#[test]
	fn npc_downed_spawns_stash_and_detaches_the_body_bag() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldStashSettings>();
		let point = Vec3::new(4.0, 1.0, -2.0);
		let bag = world.spawn(mixed_bag()).id();
		let body = world
			.spawn((
				Npc,
				Transform::from_translation(point),
				InventoryUser::carrying(bag),
				Downed { source: None, point, at: 0.0 },
				DespawnAfter::seconds(4.0),
			))
			.id();

		world
			.run_system_once(detach_downed_npc_loot)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.get::<InventoryUser>(body).is_none());
		assert!(!world.entities().contains(bag));
		assert!(world
			.get::<DespawnAfter>(body)
			.is_some_and(|timer| { (timer.remaining_secs() - 4.0).abs() < 1e-4 }));

		let (stash, transform, policy, despawn) = world
			.query::<(Entity, &Transform, &StashPolicy, &DespawnAfter)>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(transform.translation, point);
		assert!(!policy.persist);
		assert!((policy.loot_secs - DEFAULT_LOOT_SECS).abs() < 1e-4);
		assert!((despawn.remaining_secs() - DEFAULT_LOOT_SECS).abs() < 1e-3);
		assert_ne!(stash, body);
		Ok(())
	}

	#[test]
	fn empty_npc_bag_does_not_leave_a_stash() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldStashSettings>();
		let bag = world.spawn(Inventory::default()).id();
		world.spawn((
			InventoryUser::carrying(bag),
			Downed { source: None, point: Vec3::ZERO, at: 0.0 },
		));

		world
			.run_system_once(detach_downed_npc_loot)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 0);
		assert!(!world.entities().contains(bag));
		Ok(())
	}

	#[test]
	fn player_down_does_not_spawn_a_stash() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldStashSettings>();
		let bag = world.spawn(mixed_bag()).id();
		world.spawn((
			VegetationPlayer,
			InventoryUser::carrying(bag),
			Downed { source: None, point: Vec3::ONE, at: 0.0 },
		));

		world
			.run_system_once(detach_downed_npc_loot)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 0);
		assert!(world.entities().contains(bag));
		Ok(())
	}

	#[test]
	fn stash_outlives_corpse_ttl_and_despawns_at_loot_secs() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Last, tick_queued_despawns);
		let stash = app
			.world_mut()
			.run_system_once(spawn_stash_system(
				Transform::IDENTITY,
				mixed_bag(),
				StashPolicy::ephemeral(DEFAULT_LOOT_SECS),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.ok_or_else(|| anyhow::anyhow!("stash"))?;
		let remaining = app
			.world()
			.get::<DespawnAfter>(stash)
			.ok_or_else(|| anyhow::anyhow!("ttl"))?
			.remaining_secs();
		assert!((remaining - DEFAULT_LOOT_SECS).abs() < 1e-3);
		assert!(remaining > 4.0);

		app.world_mut().entity_mut(stash).insert(DespawnAfter::seconds(0.0));
		app.update();
		assert!(!app.world().entities().contains(stash));
		Ok(())
	}

	fn write_intent(world: &mut World, intent: CharacterIntent) -> anyhow::Result<()> {
		world
			.run_system_once(move |mut writer: MessageWriter<CharacterIntent>| {
				writer.write(intent);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	fn claim_setup(player_at: Vec3, stash_at: Vec3) -> anyhow::Result<(World, Entity, Entity)> {
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		let player_bag = world.spawn(Inventory::default()).id();
		let player = world
			.spawn((
				VegetationPlayer,
				Transform::from_translation(player_at),
				InventoryUser::carrying(player_bag),
			))
			.id();
		let stash = world
			.run_system_once(spawn_stash_system(
				Transform::from_translation(stash_at),
				mixed_bag(),
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.ok_or_else(|| anyhow::anyhow!("stash"))?;
		Ok((world, player, stash))
	}

	#[test]
	fn x_in_radius_take_alls_and_despawns_an_ephemeral_stash() -> anyhow::Result<()> {
		let (mut world, player, stash) = claim_setup(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0))?;
		write_intent(&mut world, CharacterIntent::StartInteraction)?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(!world.entities().contains(stash));
		let user = world
			.get::<InventoryUser>(player)
			.ok_or_else(|| anyhow::anyhow!("player bag"))?;
		let bag = world.get::<Inventory>(user.bag).ok_or_else(|| anyhow::anyhow!("inventory"))?;
		assert_eq!(bag.items.len(), 3);
		assert!(bag.clothing.is_empty());
		assert!(bag.weapons.is_empty());
		Ok(())
	}

	#[test]
	fn x_out_of_radius_is_a_noop() -> anyhow::Result<()> {
		let (mut world, player, stash) = claim_setup(Vec3::ZERO, Vec3::new(20.0, 0.0, 0.0))?;
		write_intent(&mut world, CharacterIntent::StartInteraction)?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.entities().contains(stash));
		let user = world
			.get::<InventoryUser>(player)
			.ok_or_else(|| anyhow::anyhow!("player bag"))?;
		let bag = world.get::<Inventory>(user.bag).ok_or_else(|| anyhow::anyhow!("inventory"))?;
		assert!(bag.items.is_empty());
		Ok(())
	}

	#[test]
	fn inventory_intent_drops_the_whole_bag_as_a_claimable_stash() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldStashSettings>();
		world.init_resource::<Messages<CharacterIntent>>();
		let player_bag = world.spawn(mixed_bag()).id();
		let player = world
			.spawn((
				VegetationPlayer,
				Transform::from_xyz(2.0, 0.0, 3.0),
				InventoryUser::carrying(player_bag),
			))
			.id();

		write_intent(&mut world, CharacterIntent::Inventory)?;
		world
			.run_system_once(drop_player_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let player_inv = world
			.get::<Inventory>(player_bag)
			.ok_or_else(|| anyhow::anyhow!("player bag"))?;
		assert!(player_inv.items.is_empty());

		let (stash, transform) = world
			.query_filtered::<(Entity, &Transform), With<WorldStash>>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(transform.translation, Vec3::new(2.0, 0.0, 3.0));
		assert_ne!(stash, player);

		write_intent(&mut world, CharacterIntent::StartInteraction)?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let claimed =
			world.get::<Inventory>(player_bag).ok_or_else(|| anyhow::anyhow!("claimed"))?;
		assert_eq!(claimed.items.len(), 3);
		assert!(!world.entities().contains(stash));
		Ok(())
	}

	#[test]
	fn persistent_chest_stays_after_claim() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		let player_bag = world.spawn(Inventory::default()).id();
		world.spawn((VegetationPlayer, Transform::IDENTITY, InventoryUser::carrying(player_bag)));
		let chest = world
			.run_system_once(spawn_stash_system(
				Transform::IDENTITY,
				mixed_bag(),
				StashPolicy::chest(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.ok_or_else(|| anyhow::anyhow!("chest"))?;
		assert!(world.get::<DespawnAfter>(chest).is_none());

		write_intent(&mut world, CharacterIntent::StartInteraction)?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.entities().contains(chest));
		let user = world.get::<InventoryUser>(chest).ok_or_else(|| anyhow::anyhow!("chest bag"))?;
		let bag = world
			.get::<Inventory>(user.bag)
			.ok_or_else(|| anyhow::anyhow!("chest inventory"))?;
		assert!(bag.items.is_empty());
		let player_inv =
			world.get::<Inventory>(player_bag).ok_or_else(|| anyhow::anyhow!("player"))?;
		assert_eq!(player_inv.items.len(), 3);
		Ok(())
	}
}
