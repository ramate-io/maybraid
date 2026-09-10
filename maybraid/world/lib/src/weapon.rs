//! Starter held kit for the vegetation player capsule.

use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	CharacterSpecies, Player as VegetationPlayer, PlayerVisual as VegetationPlayerVisual,
	PlaygroundMode, RequestSetCharacter, RequestSetCharacterAppearance,
};
use crozon_character_items::{CharacterSheet, Inventory, InventoryItem};
use crozon_characters::{CharacterAppearance, CharacterRoot};
use crozon_inventory_user::{spawn_bag, InventoryUser};
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use damage::Health;
use firearm_user::{
	live_weapon_from_stats, spawn_held_firearm, spawn_held_kit, spawn_reticle, FirearmUser,
	FirearmUserSettings, FirearmUserSystems, GeneratedFirearm, WeaponSwap,
};
use player::{
	apply_character_mobility, CameraFollow, Player as MaybraidPlayer, PlayerCameraAim, PlayerLook,
	PlayerUse, PlayerVisual as MaybraidPlayerVisual, PlayerYawOwner,
};

use crate::control::WorldGameplayEnabled;

/// Persisted character appearance, worn clothing, stats, and primary weapon for world entry.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct WorldPlayerLoadout {
	pub key: String,
	pub appearance: CharacterAppearance,
	pub inventory: Inventory,
}

impl WorldPlayerLoadout {
	pub fn new(
		key: impl Into<String>,
		appearance: CharacterAppearance,
		inventory: Inventory,
	) -> Self {
		let appearance = appearance.with_inventory_clothing(&inventory);
		Self { key: key.into(), appearance, inventory }
	}

	/// Replace the bag and rebuild worn garments from the new wear list.
	pub fn retarget_inventory(&mut self, inventory: Inventory) {
		*self = Self::new(self.key.clone(), self.appearance.clone(), inventory);
	}
}

#[derive(Component)]
pub(crate) struct AppliedWorldPlayerLoadout(pub WorldPlayerLoadout);

/// The replacement lifecycle already queued this body's configured visual.
#[derive(Component)]
pub(crate) struct WorldPlayerAppearanceRequested;

type WorldPlayerEquipment<'a> = (
	Entity,
	Option<&'a FirearmUser>,
	Option<&'a InventoryUser>,
	Option<&'a AppliedWorldPlayerLoadout>,
	Has<WorldPlayerAppearanceRequested>,
);

type WorldPlayerVisual<'a> = (Entity, &'a ChildOf, Has<MaybraidPlayerVisual>);

/// Give the world player its selected loadout once the Crozon visual exists.
///
/// [`firearm_user`] fire/pose query [`MaybraidPlayer`] / [`PlayerLook`]. Those
/// markers are not on the vegetation capsule, so stamp them here without the
/// player-crate locomotion controller (world already drives that capsule).
fn arm_world_player(
	mut commands: Commands,
	mode: Res<PlaygroundMode>,
	gameplay: Res<WorldGameplayEnabled>,
	loadout: Option<Res<WorldPlayerLoadout>>,
	players: Query<WorldPlayerEquipment<'_>, With<VegetationPlayer>>,
	visuals: Query<WorldPlayerVisual<'_>, (With<VegetationPlayerVisual>, With<CharacterRoot>)>,
) {
	if !gameplay.0 {
		return;
	}
	for (player, firearm_user, inventory_user, applied, appearance_requested) in &players {
		let Some((visual, _, presented)) =
			visuals.iter().find(|(_, child, _)| child.parent() == player)
		else {
			continue;
		};
		if !presented {
			commands.entity(visual).insert((MaybraidPlayerVisual, PlayerYawOwner::Wish));
		}
		if loadout.is_none() && applied.is_none() && firearm_user.is_some() {
			continue;
		}
		if loadout
			.as_ref()
			.zip(applied)
			.is_some_and(|(loadout, applied)| loadout.as_ref() == &applied.0)
		{
			continue;
		}
		commands.entity(player).insert((
			MaybraidPlayer,
			PlayerLook::default(),
			PlayerCameraAim::default(),
			PlayerYawOwner::Wish,
		));
		if *mode == PlaygroundMode::Character && gameplay.0 {
			commands.entity(player).insert(CameraFollow);
		}

		if let Some(user) = firearm_user {
			commands.entity(user.held).try_despawn();
			commands.entity(player).remove::<(FirearmUser, PlayerUse)>();
		}
		if let Some(user) = inventory_user {
			commands.entity(user.bag).try_despawn();
			commands.entity(player).remove::<InventoryUser>();
		}
		let Some(loadout) = loadout.as_ref() else {
			commands
				.entity(player)
				.remove::<(AppliedWorldPlayerLoadout, WorldPlayerAppearanceRequested)>();
			commands.entity(player).insert(Health::default());
			apply_character_mobility(&mut commands, player, 1.0, 1.0);
			if applied.is_some() {
				commands.spawn(RequestSetCharacter { species: CharacterSpecies::Braidman });
			}
			spawn_held_firearm(&mut commands, player);
			continue;
		};

		let sheet = loadout.inventory.character_sheet();
		commands.entity(player).insert((
			Health::from_max(f32::from(sheet.health.max(1))),
			AppliedWorldPlayerLoadout(loadout.as_ref().clone()),
		));
		apply_character_mobility(
			&mut commands,
			player,
			f32::from(sheet.running) / f32::from(CharacterSheet::BASE.running),
			f32::from(sheet.jump) / f32::from(CharacterSheet::BASE.jump),
		);
		spawn_bag(&mut commands, player, loadout.inventory.clone());
		if !appearance_requested {
			commands
				.spawn(RequestSetCharacterAppearance { appearance: loadout.appearance.clone() });
		}
		commands.entity(player).remove::<WorldPlayerAppearanceRequested>();
		hold_primary_weapon(&mut commands, player, &loadout.inventory);
	}
}

fn hold_primary_weapon(commands: &mut Commands, player: Entity, inventory: &Inventory) {
	let Some(InventoryItem::Firearm { spec, stats }) = inventory.primary_weapon() else {
		return;
	};
	let sheet = inventory.character_sheet();
	let live = live_weapon_from_stats(*stats, sheet.damage).with_weapon_identity(spec);
	spawn_held_kit(
		commands,
		player,
		FirearmUserSettings::default(),
		GeneratedFirearm::from_spec(*spec),
		live,
	);
}

/// Start the holster motion; the kit changes at the dip.
fn begin_weapon_swap(
	mut intents: MessageReader<CharacterIntent>,
	gameplay: Res<WorldGameplayEnabled>,
	mut commands: Commands,
	players: Query<(Entity, &InventoryUser, Has<WeaponSwap>), With<VegetationPlayer>>,
	bags: Query<&Inventory>,
) {
	if !gameplay.0 || !intents.read().any(|intent| matches!(intent, CharacterIntent::SwapActive)) {
		return;
	}
	for (player, user, swapping) in &players {
		if swapping {
			continue;
		}
		let Ok(bag) = bags.get(user.bag) else {
			continue;
		};
		if bag.weapons.len() < 2 {
			continue;
		}
		commands.entity(player).insert(WeaponSwap::start());
	}
}

/// Replace the held kit once the old gun has dipped out of the way.
fn commit_weapon_swap(
	mut commands: Commands,
	mut loadout: Option<ResMut<WorldPlayerLoadout>>,
	assets: Option<Res<AssetServer>>,
	mut players: Query<
		(Entity, &InventoryUser, Option<&FirearmUser>, &mut WeaponSwap),
		With<VegetationPlayer>,
	>,
	mut bags: Query<&mut Inventory>,
) {
	for (player, user, firearm, mut swap) in &mut players {
		if !swap.ready_to_swap() {
			continue;
		}
		let Ok(mut bag) = bags.get_mut(user.bag) else {
			continue;
		};
		if !bag.swap_active() {
			swap.mark_swapped();
			continue;
		}
		let snapshot = bag.clone();
		if let Some(firearm) = firearm {
			commands.entity(firearm.held).try_despawn();
			commands.entity(player).remove::<(FirearmUser, PlayerUse)>();
		}
		if let Some(loadout) = loadout.as_deref_mut() {
			loadout.retarget_inventory(snapshot.clone());
			commands.entity(player).insert(AppliedWorldPlayerLoadout(loadout.clone()));
		}
		if assets.is_some() {
			hold_primary_weapon(&mut commands, player, &snapshot);
		}
		swap.mark_swapped();
	}
}

fn spawn_world_reticle(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	spawn_reticle(&mut commands, &mut meshes, &mut materials);
}

pub(crate) fn configure(app: &mut App) {
	app.add_systems(Startup, spawn_world_reticle).add_systems(
		Update,
		(
			arm_world_player,
			(begin_weapon_swap, commit_weapon_swap.after(FirearmUserSystems::Swap))
				.chain()
				.after(CharacterControlSystems)
				.run_if(resource_equals(WorldGameplayEnabled(true))),
		),
	);
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use chico_vegetation_on_terrain_playground::{
		Player as VegetationPlayer, PlayerVisual as VegetationPlayerVisual, PlaygroundMode,
		RequestSetCharacterAppearance,
	};
	use crozon_character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, Inventory, InventoryItem, ItemColor,
	};
	use crozon_characters::{CharacterAppearance, CharacterRoot};

	use crate::weapon::{arm_world_player, WorldPlayerAppearanceRequested, WorldPlayerLoadout};
	use crate::WorldGameplayEnabled;

	#[test]
	fn world_loadout_keeps_primary_weapon_and_worn_clothing() {
		let inventory = Inventory::with_starter_outfit(vec![
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Tattered,
				ItemColor::Green,
			),
			InventoryItem::firearm(FirearmMesh::Reltor),
		]);
		let loadout = WorldPlayerLoadout::new("active", CharacterAppearance::default(), inventory);
		assert_eq!(
			loadout.inventory.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		if let CharacterAppearance::Braidman(config) = loadout.appearance {
			assert_eq!(config.clothing, vec![ClothingMesh::Pants]);
			assert_eq!(
				config.colors.clothing.first().map(|clothing| clothing.color),
				Some(ItemColor::Green)
			);
		}
	}

	#[test]
	fn respawn_does_not_replace_the_visual_twice() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			Inventory::default(),
		));
		let player = world.spawn((VegetationPlayer, WorldPlayerAppearanceRequested)).id();
		world.spawn((VegetationPlayerVisual, CharacterRoot, ChildOf(player)));

		world
			.run_system_once(arm_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(world.query::<&RequestSetCharacterAppearance>().iter(&world).count(), 0);
		assert!(world.get::<WorldPlayerAppearanceRequested>(player).is_none());
		Ok(())
	}

	fn two_gun_bag() -> Inventory {
		Inventory {
			items: vec![
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::firearm(FirearmMesh::Reltor),
			],
			clothing: Vec::new(),
			weapons: vec![0, 1],
		}
	}

	#[test]
	fn y_starts_a_swap_without_changing_the_kit() -> anyhow::Result<()> {
		use crozon_inventory_user::InventoryUser;
		use firearm_user::{FirearmUser, WeaponSwap};
		use maybraid_character_controller::CharacterIntent;

		use crate::weapon::begin_weapon_swap;

		let inventory = two_gun_bag();
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.insert_resource(WorldGameplayEnabled(true));
		let bag = world.spawn(inventory).id();
		let held = world.spawn_empty().id();
		let player = world
			.spawn((
				VegetationPlayer,
				InventoryUser::carrying(bag),
				FirearmUser::holding(held),
			))
			.id();
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SwapActive);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(begin_weapon_swap)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.get::<WeaponSwap>(player).is_some());
		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
		assert!(world.entities().contains(held));
		Ok(())
	}

	#[test]
	fn y_swaps_the_queued_primary() -> anyhow::Result<()> {
		use crozon_inventory_user::InventoryUser;
		use firearm_user::{FirearmUser, WeaponSwap, WEAPON_SWAP_SECS};

		use crate::weapon::commit_weapon_swap;

		let inventory = two_gun_bag();
		let mut world = World::new();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			inventory.clone(),
		));
		let bag = world.spawn(inventory).id();
		let held = world.spawn_empty().id();
		world.spawn((
			VegetationPlayer,
			InventoryUser::carrying(bag),
			FirearmUser::holding(held),
			WeaponSwap {
				elapsed: WEAPON_SWAP_SECS * 0.5,
				duration: WEAPON_SWAP_SECS,
				swapped: false,
			},
		));
		world
			.run_system_once(commit_weapon_swap)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		assert!(!world.entities().contains(held));
		let loadout = world
			.get_resource::<WorldPlayerLoadout>()
			.ok_or_else(|| anyhow::anyhow!("loadout"))?;
		assert_eq!(
			loadout.inventory.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		Ok(())
	}
}
