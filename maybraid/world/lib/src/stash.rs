//! Claimable world stashes for NPC death loot, player drops, and chests.
//!
//! One host type ([`WorldStash`]) holds a bag via [`InventoryUser`]. Claim is
//! take-all on [`CharacterIntent::StartInteraction`] (pad **X**) inside
//! [`StashPolicy::claim_radius`]. Death and player drop
//! [`Inventory::explode`] into one stash per item so each piece is claimable
//! on its own. Authored chests stay a single pile.
//!
//! Loot TTL is [`StashPolicy::loot_secs`] (default 60 s), independent of
//! mob corpse lifetime (4 s). Persistent chests omit [`DespawnAfter`].
//! Absorb never auto-equips. Claim and drop refresh [`WorldPlayerLoadout`]
//! so the live bag can persist.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use bevy::text::FontSize;
use chico_vegetation_on_terrain_playground::Player as VegetationPlayer;
use crozon_character_items::{
	ClothingHost, Inventory, InventoryItem, InventorySlot, MaterialRefParams,
};
use crozon_characters::{
	add_character_components_host, character_bounds, CharacterComponents, ClothingLayer,
	ComponentsOnly, Layers, PartNode,
};
use crozon_inventory_user::{spawn_bag, InventoryUser};
use damage::{DamageSystems, DespawnAfter, Downed};
use firearm_user::{held_scale_from_bounds, FirearmUser, FirearmUserSettings, GeneratedFirearm};
use firearms::{firearm_bounds, spawn_firearm_components};
use lod::gen::LodSceneLevel;
use lod::lod_ref::LodRef;
use lod::LodScene;
use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use player::PlayerUse;

use crate::control::WorldGameplayEnabled;
use crate::weapon::{AppliedWorldPlayerLoadout, WorldPlayerLoadout};

/// Default unclaimed-loot lifetime. Independent of the 4 s corpse clock.
pub const DEFAULT_LOOT_SECS: f32 = 60.0;

/// Default interact radius for take-all claim (XZ).
pub const DEFAULT_CLAIM_RADIUS: f32 = 4.0;

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

/// On-screen X / E prompt while a claimable stash is in radius.
#[derive(Component)]
struct StashInteractPrompt;

/// Bind-pose garment so stash clothing keeps recipe + palette.
#[derive(Clone, PartialEq)]
struct StashClothingPreview {
	layer: ClothingLayer,
}

impl Default for StashClothingPreview {
	fn default() -> Self {
		Self {
			layer: ClothingLayer::new(
				crozon_character_items::ClothingMesh::TankTop,
				crozon_character_items::ItemColor::Natural,
				ClothingHost::HUMANOID,
			),
		}
	}
}

impl CharacterComponents for StashClothingPreview {
	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("clothing", vec![self.layer.preview_part_node()])
	}
}

pub struct WorldStashPlugin;

impl Plugin for WorldStashPlugin {
	fn build(&self, app: &mut App) {
		add_character_components_host::<StashClothingPreview>(app);
		app.init_resource::<WorldStashSettings>()
			.add_systems(Startup, spawn_stash_interact_prompt)
			.add_systems(
				Update,
				(
					claim_nearby_stashes
						.after(CharacterControlSystems)
						.run_if(resource_equals(WorldGameplayEnabled(true))),
					drop_player_inventory
						.after(CharacterControlSystems)
						.run_if(resource_equals(WorldGameplayEnabled(true))),
					sync_stash_interact_prompt
						.after(CharacterControlSystems)
						.run_if(resource_equals(WorldGameplayEnabled(true))),
				),
			);
		app.add_systems(PostUpdate, detach_downed_npc_loot.after(DamageSystems::Down));
	}
}

/// Scatter radius for the first exploded item; grows slowly with count.
const EXPLODE_BASE_RADIUS: f32 = 0.55;

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

/// One [`WorldStash`] per item, ring-scattered around `origin`.
///
/// Chests should keep calling [`spawn_world_stash`] so the pile stays take-all.
pub fn spawn_exploded_stashes(
	commands: &mut Commands,
	origin: Vec3,
	inventory: Inventory,
	policy: StashPolicy,
	assets: Option<&AssetServer>,
) -> Vec<Entity> {
	let pieces = inventory.explode();
	let count = pieces.len();
	pieces
		.into_iter()
		.enumerate()
		.filter_map(|(index, bag)| {
			let transform = Transform::from_translation(origin + explode_offset(index, count));
			spawn_world_stash(commands, transform, bag, policy, assets)
		})
		.collect()
}

fn explode_offset(index: usize, count: usize) -> Vec3 {
	if count <= 1 {
		return Vec3::new(0.0, 0.05, 0.0);
	}
	let angle = index as f32 * std::f32::consts::TAU / count as f32;
	let radius = EXPLODE_BASE_RADIUS + (count as f32).sqrt() * 0.12;
	Vec3::new(angle.cos() * radius, 0.05, angle.sin() * radius)
}

fn apply_claimed_loadout(
	commands: &mut Commands,
	player: Entity,
	inventory: &Inventory,
	loadout: &mut WorldPlayerLoadout,
) {
	loadout.retarget_inventory(inventory.clone());
	commands.entity(player).insert(AppliedWorldPlayerLoadout(loadout.clone()));
}

fn apply_dropped_loadout(loadout: &mut WorldPlayerLoadout, inventory: &Inventory) {
	loadout.retarget_inventory(inventory.clone());
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

fn clothing_material_ref(material: MaterialRefParams) -> MaterialRef {
	MaterialRef::named(material.id.recipe_id()).with_palette([material.color.color()])
}

fn spawn_displayed_item(
	commands: &mut Commands,
	host: Entity,
	item: &InventoryItem,
	displayed: StashDisplayedItem,
	transform: Transform,
	assets: Option<&AssetServer>,
) {
	match displayed.slot {
		InventorySlot::Clothing => {
			spawn_displayed_clothing(commands, host, item, displayed, transform, assets);
		}
		InventorySlot::Weapons => {
			spawn_displayed_weapon(commands, host, item, displayed, transform, assets);
		}
	}
}

fn spawn_displayed_clothing(
	commands: &mut Commands,
	host: Entity,
	item: &InventoryItem,
	displayed: StashDisplayedItem,
	transform: Transform,
	assets: Option<&AssetServer>,
) {
	let material = item.material().map(clothing_material_ref);
	if let (Some(mesh), Some(params), Some(_)) = (item.mesh(), item.material(), assets) {
		let preview = StashClothingPreview {
			layer: ClothingLayer::new(mesh, params.color, ClothingHost::HUMANOID)
				.with_material(params.id),
		};
		let bounds = character_bounds(&preview);
		let identity = Transform::IDENTITY;
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		let entity = commands
			.spawn_scene((
				ComponentsOnly(preview).host(&lod_ref),
				bsn! {
					template_value(transform)
				},
			))
			.id();
		commands.entity(entity).insert((
			Name::new(format!("stash-{}", item.label())),
			displayed,
			ChildOf(host),
		));
		return;
	}
	let mut visual = commands.spawn((
		Name::new(format!("stash-{}", item.label())),
		displayed,
		transform,
		Visibility::default(),
		ChildOf(host),
	));
	if let Some(material) = material {
		visual.insert((MaterialRefRoot(material), PropagateToDescendants));
	}
}

fn spawn_displayed_weapon(
	commands: &mut Commands,
	host: Entity,
	item: &InventoryItem,
	displayed: StashDisplayedItem,
	transform: Transform,
	assets: Option<&AssetServer>,
) {
	let transform = match item.firearm_spec() {
		Some(spec) => {
			let kit = GeneratedFirearm::from_spec(spec);
			let bounds = firearm_bounds(&kit);
			let scale = held_scale_from_bounds(bounds, FirearmUserSettings::default().held_length);
			let transform = transform.with_scale(Vec3::splat(scale));
			if assets.is_some() {
				for entity in spawn_firearm_components(commands, &kit, transform, bounds) {
					commands.entity(entity).insert((
						Name::new(format!("stash-{}", item.label())),
						displayed,
						ChildOf(host),
					));
				}
				return;
			}
			transform
		}
		None => transform,
	};
	commands.spawn((
		Name::new(format!("stash-{}", item.label())),
		displayed,
		transform,
		Visibility::default(),
		ChildOf(host),
	));
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
		spawn_exploded_stashes(&mut commands, downed.point, loot, policy, assets);
	}
}

fn player_origin(transform: &Transform) -> Vec3 {
	transform.translation
}

fn xz_distance(a: Vec3, b: Vec3) -> f32 {
	a.xz().distance(b.xz())
}

fn claim_nearby_stashes(
	mut intents: MessageReader<CharacterIntent>,
	mut commands: Commands,
	mut loadout: Option<ResMut<WorldPlayerLoadout>>,
	players: Query<(Entity, &Transform, &InventoryUser), With<VegetationPlayer>>,
	stashes: Query<(Entity, &Transform, &InventoryUser, &StashPolicy), With<WorldStash>>,
	displayed: Query<(Entity, &ChildOf), With<StashDisplayedItem>>,
	mut bags: Query<&mut Inventory>,
) {
	if !intents.read().any(|intent| matches!(intent, CharacterIntent::StartInteraction)) {
		return;
	}
	for (player, player_transform, player_user) in &players {
		let origin = player_origin(player_transform);
		let Some((stash, stash_bag, policy)) = nearest_stash_in_radius(origin, &stashes) else {
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
		let snapshot = player_bag.clone();
		let visual: Vec<Entity> = displayed
			.iter()
			.filter(|(_, child)| child.parent() == stash)
			.map(|(entity, _)| entity)
			.collect();
		despawn_displayed_items(&mut commands, &visual);
		if let Some(loadout) = loadout.as_deref_mut() {
			apply_claimed_loadout(&mut commands, player, &snapshot, loadout);
		}
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
			let distance = xz_distance(transform.translation, origin);
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
	mut loadout: Option<ResMut<WorldPlayerLoadout>>,
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
		let snapshot = bag.clone();
		if let Some(firearm) = firearm {
			commands.entity(firearm.held).try_despawn();
			commands.entity(player).remove::<(FirearmUser, PlayerUse)>();
		}
		if let Some(loadout) = loadout.as_deref_mut() {
			apply_dropped_loadout(loadout, &snapshot);
		}
		spawn_exploded_stashes(
			&mut commands,
			player_origin(transform),
			loot,
			settings.ephemeral_policy(),
			assets,
		);
	}
}

fn spawn_stash_interact_prompt(mut commands: Commands) {
	commands.spawn((
		Name::new("stash-interact-prompt"),
		StashInteractPrompt,
		Node {
			position_type: PositionType::Absolute,
			bottom: Val::Px(48.0),
			width: Val::Percent(100.0),
			justify_content: JustifyContent::Center,
			..default()
		},
		Text::new("X / E  Pick up"),
		TextFont { font_size: FontSize::Px(22.0), ..default() },
		TextColor(Color::srgba(0.95, 0.92, 0.82, 0.95)),
		Pickable::IGNORE,
		Visibility::Hidden,
	));
}

fn sync_stash_interact_prompt(
	players: Query<&Transform, With<VegetationPlayer>>,
	stashes: Query<(Entity, &Transform, &InventoryUser, &StashPolicy), With<WorldStash>>,
	mut prompt: Query<&mut Visibility, With<StashInteractPrompt>>,
) {
	let in_range = players
		.iter()
		.any(|transform| nearest_stash_in_radius(player_origin(transform), &stashes).is_some());
	for mut visibility in &mut prompt {
		*visibility = if in_range { Visibility::Visible } else { Visibility::Hidden };
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
	use firearm_user::held_scale_from_bounds;
	use material_ref::MaterialId;
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

		let stashes: Vec<_> = world
			.query::<(Entity, &Transform, &StashPolicy, &DespawnAfter)>()
			.iter(&world)
			.collect();
		assert_eq!(stashes.len(), 3);
		for (stash, transform, policy, despawn) in &stashes {
			assert!((transform.translation.xz() - point.xz()).length() < 2.0);
			assert!(!policy.persist);
			assert!((policy.loot_secs - DEFAULT_LOOT_SECS).abs() < 1e-4);
			assert!((despawn.remaining_secs() - DEFAULT_LOOT_SECS).abs() < 1e-3);
			assert_ne!(*stash, body);
		}
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
	fn inventory_intent_explodes_the_bag_into_claimable_items() -> anyhow::Result<()> {
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

		let stashes: Vec<_> = world
			.query_filtered::<(Entity, &Transform), With<WorldStash>>()
			.iter(&world)
			.map(|(entity, transform)| (entity, transform.translation))
			.collect();
		assert_eq!(stashes.len(), 3);
		for (stash, translation) in &stashes {
			assert!((translation.xz() - Vec2::new(2.0, 3.0)).length() < 2.0);
			assert_ne!(*stash, player);
		}

		write_intent(&mut world, CharacterIntent::StartInteraction)?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let claimed =
			world.get::<Inventory>(player_bag).ok_or_else(|| anyhow::anyhow!("claimed"))?;
		assert_eq!(claimed.items.len(), 1);
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 2);
		Ok(())
	}

	#[test]
	fn explode_spawns_one_visible_stash_per_item() -> anyhow::Result<()> {
		let mut world = World::new();
		let spawned = world
			.run_system_once(|mut commands: Commands| {
				spawn_exploded_stashes(
					&mut commands,
					Vec3::ZERO,
					mixed_bag(),
					StashPolicy::default(),
					None,
				)
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(spawned.len(), 3);
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 3);
		assert_eq!(world.query::<&StashDisplayedItem>().iter(&world).count(), 3);

		let origins: Vec<Vec3> = world
			.query_filtered::<&Transform, With<WorldStash>>()
			.iter(&world)
			.map(|transform| transform.translation)
			.collect();
		assert!(origins.windows(2).any(|pair| pair[0].distance(pair[1]) > 0.3));
		Ok(())
	}

	#[test]
	fn claim_updates_world_player_loadout() -> anyhow::Result<()> {
		use crate::weapon::WorldPlayerLoadout;
		use crozon_characters::CharacterAppearance;

		let (mut world, player, stash) = claim_setup(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0))?;
		world.insert_resource(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			Inventory::default(),
		));
		write_intent(&mut world, CharacterIntent::StartInteraction)?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(!world.entities().contains(stash));
		let loadout = world
			.get_resource::<WorldPlayerLoadout>()
			.ok_or_else(|| anyhow::anyhow!("loadout"))?;
		assert_eq!(loadout.inventory.items.len(), 3);
		let applied = world
			.get::<crate::weapon::AppliedWorldPlayerLoadout>(player)
			.ok_or_else(|| anyhow::anyhow!("applied"))?;
		assert_eq!(applied.0.inventory.items.len(), 3);
		Ok(())
	}

	#[test]
	fn drop_clears_world_player_loadout() -> anyhow::Result<()> {
		use crate::weapon::WorldPlayerLoadout;
		use crozon_characters::CharacterAppearance;

		let mut world = World::new();
		world.init_resource::<WorldStashSettings>();
		world.init_resource::<Messages<CharacterIntent>>();
		let bag = mixed_bag();
		world.insert_resource(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			bag.clone(),
		));
		let player_bag = world.spawn(bag).id();
		world.spawn((
			VegetationPlayer,
			Transform::IDENTITY,
			InventoryUser::carrying(player_bag),
		));

		write_intent(&mut world, CharacterIntent::Inventory)?;
		world
			.run_system_once(drop_player_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let loadout = world
			.get_resource::<WorldPlayerLoadout>()
			.ok_or_else(|| anyhow::anyhow!("loadout"))?;
		assert!(loadout.inventory.items.is_empty());
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 3);
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

	#[test]
	fn clothing_keeps_recipe_and_palette() -> anyhow::Result<()> {
		let mut world = World::new();
		world
			.run_system_once(spawn_stash_system(
				Transform::IDENTITY,
				mixed_bag(),
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let material = world
			.query::<(&StashDisplayedItem, &MaterialRefRoot)>()
			.iter(&world)
			.find(|(item, _)| item.slot == InventorySlot::Clothing)
			.map(|(_, root)| root.0.clone())
			.ok_or_else(|| anyhow::anyhow!("clothing material"))?;
		assert_eq!(material.name, MaterialId::named(ClothingMaterial::Cloth.recipe_id()));
		assert_eq!(material.palette[0], ItemColor::Natural.color());
		Ok(())
	}

	#[test]
	fn dropped_weapon_uses_held_kit_scale() -> anyhow::Result<()> {
		let mut world = World::new();
		world
			.run_system_once(spawn_stash_system(
				Transform::IDENTITY,
				Inventory {
					items: vec![InventoryItem::firearm(FirearmMesh::Bullpup)],
					clothing: Vec::new(),
					weapons: vec![0],
				},
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let transform = world
			.query::<(&StashDisplayedItem, &Transform)>()
			.iter(&world)
			.find(|(item, _)| item.slot == InventorySlot::Weapons)
			.map(|(_, transform)| *transform)
			.ok_or_else(|| anyhow::anyhow!("weapon visual"))?;
		let kit = GeneratedFirearm::from_spec(
			InventoryItem::firearm(FirearmMesh::Bullpup)
				.firearm_spec()
				.ok_or_else(|| anyhow::anyhow!("spec"))?,
		);
		let expected = held_scale_from_bounds(
			firearm_bounds(&kit),
			FirearmUserSettings::default().held_length,
		);
		assert!((transform.scale.x - expected).abs() < 1e-4);
		assert!(transform.scale.x < 1.0);
		Ok(())
	}

	#[test]
	fn xz_claim_ignores_height() -> anyhow::Result<()> {
		let (mut world, player, stash) =
			claim_setup(Vec3::new(0.0, 8.0, 0.0), Vec3::new(2.0, 0.0, 0.0))?;
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
		Ok(())
	}

	#[test]
	fn interact_prompt_toggles_in_radius() -> anyhow::Result<()> {
		let (mut world, _, stash) = claim_setup(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0))?;
		world.spawn((StashInteractPrompt, Visibility::Hidden, Text::new("X / E  Pick up")));
		world
			.run_system_once(sync_stash_interact_prompt)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let visible = world
			.query_filtered::<&Visibility, With<StashInteractPrompt>>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*visible, Visibility::Visible);

		world.entity_mut(stash).insert(Transform::from_xyz(40.0, 0.0, 0.0));
		world
			.run_system_once(sync_stash_interact_prompt)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let hidden = world
			.query_filtered::<&Visibility, With<StashInteractPrompt>>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*hidden, Visibility::Hidden);
		Ok(())
	}
}
