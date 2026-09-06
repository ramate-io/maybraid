//! Starter held kit for the vegetation player capsule.

use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	CharacterSpecies, Player as VegetationPlayer, PlayerVisual as VegetationPlayerVisual,
	PlaygroundMode, RequestSetCharacter, RequestSetCharacterAppearance,
};
use crozon_character_items::{CharacterSheet, Inventory, InventoryItem};
use crozon_characters::{CharacterAppearance, CharacterRoot};
use crozon_inventory_user::{spawn_bag, InventoryUser};
use damage::Health;
use firearm_user::{
	live_weapon_from_stats, spawn_held_firearm, spawn_held_kit, spawn_reticle, FirearmUser,
	FirearmUserSettings,
};
use mob_characters::GeneratedFirearm;
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
}

#[derive(Component)]
struct AppliedWorldPlayerLoadout(WorldPlayerLoadout);

type WorldPlayerEquipment<'a> = (
	Entity,
	Option<&'a FirearmUser>,
	Option<&'a InventoryUser>,
	Option<&'a AppliedWorldPlayerLoadout>,
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
	for (player, firearm_user, inventory_user, applied) in &players {
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
			commands.entity(player).remove::<AppliedWorldPlayerLoadout>();
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
		commands.spawn(RequestSetCharacterAppearance { appearance: loadout.appearance.clone() });
		if let Some(InventoryItem::Firearm { spec, stats }) = loadout.inventory.primary_weapon() {
			let live = live_weapon_from_stats(*stats, sheet.damage).with_weapon_identity(spec);
			spawn_held_kit(
				&mut commands,
				player,
				FirearmUserSettings::default(),
				GeneratedFirearm::from_spec(*spec),
				live,
			);
		}
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
	app.add_systems(Startup, spawn_world_reticle)
		.add_systems(Update, arm_world_player);
}

#[cfg(test)]
mod tests {
	use crozon_character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, Inventory, InventoryItem, ItemColor,
	};
	use crozon_characters::CharacterAppearance;

	use crate::weapon::WorldPlayerLoadout;

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
}
