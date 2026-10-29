//! Starter held kit for the vegetation player capsule.

use bevy::prelude::*;
use character_inventory_user::{spawn_bag, InventoryUser};
use character_items::{CharacterSheet, Inventory, InventoryItem, TanzId};
use characters::{CharacterAppearance, CharacterRoot};
use damage::Health;
use firearm_user::{spawn_reticle, FirearmUser};
use grenade_user::GrenadeUser;
use grenades::GrenadeDetonated;
use maybraid_character_controller::{ActorControlStatus, CharacterControlSystems};
use maybraid_equipment::{
	EquipmentInventoryChanged, EquipmentSystems, InventoryCycling, PendingEquipmentHold,
	PendingEquipmentTeardown, ReconcileEquipmentFromInventory,
};
use maybraid_session::WorldGameplayEnabled;
use maybraid_virtual_pad_controller::PrimaryVirtualPad;

use maybraid_skill_map::{spawn_skill_maps, SkillMapEquip, SkillMapSystems};
use maybraid_vfx::{SpawnVfxExt, VfxLibrary, VfxSpawn};
use player::{
	apply_character_mobility, Player as MaybraidPlayer, PlayerCameraAim, PlayerLook, PlayerSystems,
	PlayerVisual as MaybraidPlayerVisual, PlayerYawOwner,
};
use tanz_user::{spawn_tanz_user, TanzExecution, TanzStack, TanzUser, TanzUserSystems};
use world_player::{
	CharacterSpecies, Player as VegetationPlayer, PlayerVisual as VegetationPlayerVisual,
	RequestSetCharacter, RequestSetCharacterAppearance,
};

use crate::control::InventoryEditCameraFollow;

/// Persisted character appearance, worn clothing, stats, and primary weapon for world entry.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct WorldPlayerLoadout {
	pub key: String,
	pub name: String,
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
		Self { key: key.into(), name: String::from("Player"), appearance, inventory }
	}

	/// Character display name shown on the vitals HUD. Empty input stays `"Player"`.
	pub fn with_name(mut self, name: impl Into<String>) -> Self {
		let name = name.into();
		let trimmed = name.trim();
		self.name = if trimmed.is_empty() { String::from("Player") } else { trimmed.to_string() };
		self
	}

	/// Replace the bag and rebuild worn garments from the new wear list.
	pub fn retarget_inventory(&mut self, inventory: Inventory) {
		let name = self.name.clone();
		*self = Self::new(self.key.clone(), self.appearance.clone(), inventory).with_name(name);
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
	Option<&'a GrenadeUser>,
	Option<&'a InventoryUser>,
	Option<&'a maybraid_skill_map::SkillMapUser>,
	Option<&'a TanzUser>,
	Option<&'a AppliedWorldPlayerLoadout>,
	Has<WorldPlayerAppearanceRequested>,
);

type WorldPlayerVisual<'a> = (Entity, &'a ChildOf, Has<MaybraidPlayerVisual>);

/// Give the world player its selected loadout once the character visual exists.
///
/// [`firearm_user`] fire/pose query [`MaybraidPlayer`] / [`PlayerLook`]. The
/// player-crate locomotion controller is stamped in [`crate::control`] so
/// column buoyancy runs; this only arms kit / identity once the visual exists.
pub(crate) fn arm_world_player(
	mut commands: Commands,
	gameplay: Res<WorldGameplayEnabled>,
	inventory_edit: Option<Res<InventoryEditCameraFollow>>,
	loadout: Option<Res<WorldPlayerLoadout>>,
	primary: Option<Res<PrimaryVirtualPad>>,
	players: Query<WorldPlayerEquipment<'_>, With<VegetationPlayer>>,
	controls: Query<&ActorControlStatus, With<VegetationPlayer>>,
	visuals: Query<WorldPlayerVisual<'_>, (With<VegetationPlayerVisual>, With<CharacterRoot>)>,
) {
	if !gameplay.0 && !inventory_edit.is_some_and(|edit| edit.0) {
		return;
	}
	for (
		player,
		firearm_user,
		grenade_user,
		inventory_user,
		skill_map_user,
		tanz_user,
		applied,
		appearance_requested,
	) in &players
	{
		if !primary.as_deref().is_some_and(|primary| {
			controls
				.get(player)
				.ok()
				.and_then(|status| status.source())
				.and_then(|source| source.entity())
				== Some(primary.0)
		}) {
			continue;
		}
		let Some((visual, _, presented)) =
			visuals.iter().find(|(_, child, _)| child.parent() == player)
		else {
			continue;
		};
		if !presented {
			commands.entity(visual).insert((MaybraidPlayerVisual, PlayerYawOwner::Wish));
		}
		if skill_map_user.is_none() {
			spawn_skill_maps(&mut commands, player);
		}
		if tanz_user.is_none() {
			spawn_tanz_user(&mut commands, player);
		}
		if loadout.is_none()
			&& applied.is_none()
			&& (firearm_user.is_some() || grenade_user.is_some())
		{
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
		commands.entity(player).try_insert(InventoryCycling::default());

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
			commands.entity(player).remove::<ReconcileEquipmentFromInventory>();
			PendingEquipmentHold::request_default_firearm(&mut commands.entity(player));
			continue;
		};

		let sheet = loadout.inventory.character_sheet();
		commands.entity(player).insert((
			Health::from_max(f32::from(sheet.health.max(1))),
			AppliedWorldPlayerLoadout(loadout.as_ref().clone()),
			Name::new(loadout.name.clone()),
		));
		apply_character_mobility(
			&mut commands,
			player,
			f32::from(sheet.running) / f32::from(CharacterSheet::BASE.running),
			f32::from(sheet.jump) / f32::from(CharacterSheet::BASE.jump),
		);
		spawn_bag(&mut commands, player, loadout.inventory.clone());
		commands.entity(player).insert(skill_map_equip_from(&loadout.inventory));
		if !appearance_requested {
			commands
				.spawn(RequestSetCharacterAppearance { appearance: loadout.appearance.clone() });
		}
		commands.entity(player).remove::<WorldPlayerAppearanceRequested>();
		commands.entity(player).insert(ReconcileEquipmentFromInventory);
		let mut entity = commands.entity(player);
		PendingEquipmentTeardown::request(&mut entity);
		PendingEquipmentHold::request(&mut entity, loadout.inventory.clone());
	}
}

fn skill_map_equip_from(inventory: &Inventory) -> SkillMapEquip {
	SkillMapEquip::from_spec(inventory.primary_skill_map().and_then(InventoryItem::skill_map_spec))
}

fn apply_equipment_inventory_to_loadout(
	mut changes: MessageReader<EquipmentInventoryChanged>,
	mut loadout: Option<ResMut<WorldPlayerLoadout>>,
	primary: Option<Res<PrimaryVirtualPad>>,
	controls: Query<&ActorControlStatus, With<VegetationPlayer>>,
	mut commands: Commands,
) {
	let Some(loadout) = loadout.as_deref_mut() else {
		return;
	};
	for change in changes.read() {
		if !primary.as_deref().is_some_and(|primary| {
			controls
				.get(change.actor())
				.ok()
				.and_then(|status| status.source())
				.and_then(|source| source.entity())
				== Some(primary.0)
		}) {
			continue;
		}
		loadout.retarget_inventory(change.inventory().clone());
		commands
			.entity(change.actor())
			.insert(AppliedWorldPlayerLoadout(loadout.clone()));
	}
}

fn sync_skill_map_equip(
	mut users: Query<(&InventoryUser, &mut SkillMapEquip)>,
	bags: Query<&Inventory>,
) {
	for (user, mut equip) in &mut users {
		let Ok(bag) = bags.get(user.bag) else {
			continue;
		};
		let next = skill_map_equip_from(bag);
		if *equip != next {
			*equip = next;
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

fn spawn_detonation_vfx(
	mut detonations: MessageReader<GrenadeDetonated>,
	mut commands: Commands,
	library: Option<Res<VfxLibrary>>,
) {
	let Some(library) = library else {
		return;
	};
	for event in detonations.read() {
		commands.spawn_vfx(
			&library.fiery_explosion,
			VfxSpawn {
				transform: Transform::from_translation(event.position),
				scale: event.effect.scale,
				intensity: event.effect.intensity,
				playback: event.effect.playback,
				seed: event.effect.seed,
				..default()
			},
		);
	}
}

fn equipped_tanzes(inventory: &Inventory) -> Vec<TanzId> {
	inventory.equipped_tanzes()
}

/// Inventory is the authority; [`TanzStack`] is a derived matching view.
///
/// Runs before match/execute so bag edits affect the next input. A missing bag
/// clears the stack. Active Tanzes and cooldowns stay; a buffered successor
/// that is no longer equipped is discarded.
fn sync_tanz_stack_from_inventory(
	mut users: Query<(Option<&InventoryUser>, &mut TanzStack, &mut TanzExecution)>,
	bags: Query<&Inventory>,
) {
	for (inventory_user, mut stack, mut execution) in &mut users {
		let next = inventory_user
			.and_then(|user| bags.get(user.bag).ok())
			.map(equipped_tanzes)
			.unwrap_or_default();
		if stack.equipped != next {
			stack.equipped = next;
		}
		if execution.buffered.is_some_and(|queued| !stack.contains(queued.id)) {
			execution.buffered = None;
		}
	}
}

pub(crate) fn configure(app: &mut App) {
	app.add_systems(Startup, spawn_world_reticle);
	configure_runtime(app);
}

/// World loadout / persistence adapters used by the assembled world and
/// headless production wiring tests.
pub(crate) fn configure_runtime(app: &mut App) {
	app.add_systems(
		Update,
		(
			arm_world_player
				.after(crate::inventory_drop::InventoryDropSystems)
				.before(EquipmentSystems::Reconcile)
				.before(EquipmentSystems::Teardown)
				.before(EquipmentSystems::Hold)
				.before(PlayerSystems::Intent),
			sync_tanz_stack_from_inventory
				.after(arm_world_player)
				.before(TanzUserSystems::Match)
				.before(TanzUserSystems::Execute),
			apply_equipment_inventory_to_loadout
				.after(EquipmentSystems::CycleQueue)
				.after(EquipmentSystems::CommitHolster)
				.after(CharacterControlSystems)
				.after(maybraid_session::SessionCapabilitySystems)
				.before(PlayerSystems::Intent)
				.run_if(resource_equals(WorldGameplayEnabled(true))),
			sync_skill_map_equip
				.after(EquipmentSystems::CycleQueue)
				.after(CharacterControlSystems)
				.after(maybraid_session::SessionCapabilitySystems)
				.before(SkillMapSystems::Spawn)
				.run_if(resource_equals(WorldGameplayEnabled(true))),
			spawn_detonation_vfx,
		),
	);
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use bevy::time::TimeUpdateStrategy;
	use character_inventory_user::InventoryUser;
	use character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, Inventory, InventoryDropRequest,
		InventoryItem, ItemColor,
	};
	use characters::{CharacterAppearance, CharacterRoot};
	use tanz_user::{BufferedTanz, ControlMode, TanzExecution, TanzId, TanzStack, TanzUser};
	use world_player::{
		Player as VegetationPlayer, PlayerVisual as VegetationPlayerVisual, PlaygroundMode,
		RequestSetCharacterAppearance,
	};

	use crate::weapon::{
		apply_equipment_inventory_to_loadout, arm_world_player, AppliedWorldPlayerLoadout,
		WorldPlayerAppearanceRequested, WorldPlayerLoadout,
	};
	use maybraid_character_controller::ActorControlStatus;
	use maybraid_controllers_core::{ActorTarget, InputRouter, InputSource};
	use maybraid_equipment::PendingEquipmentHold;
	use maybraid_equipment::{EquipmentInventoryChanged, EquippedItemHandler, InventoryCycling};
	use maybraid_session::WorldGameplayEnabled;
	use maybraid_virtual_pad_controller::PrimaryVirtualPad;

	#[derive(Resource, Default)]
	struct WorldStrikes(usize);

	fn record_world_strikes(
		mut strikes: MessageReader<fisticuffs_user::FisticuffsStrike>,
		mut count: ResMut<WorldStrikes>,
	) {
		count.0 += strikes.read().count();
	}

	fn assign_player(world: &mut World, player: Entity) {
		let source_entity = world
			.get_resource::<PrimaryVirtualPad>()
			.map(|primary| primary.0)
			.unwrap_or_else(|| world.spawn_empty().id());
		let source = InputSource::virtual_pad(source_entity);
		world.insert_resource(PrimaryVirtualPad(source_entity));
		world.entity_mut(player).insert(ActorControlStatus::assigned(source));
		world.init_resource::<InputRouter>();
		world.resource_mut::<InputRouter>().set_local_source(source_entity);
		if world.get::<InventoryCycling>(player).is_none() {
			world.entity_mut(player).insert(InventoryCycling::default());
		}
		world
			.resource_mut::<InputRouter>()
			.assign_actor(source, ActorTarget::new(player));
	}

	fn weapon_inventory(item: InventoryItem) -> Inventory {
		Inventory {
			items: vec![item],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
			tanzes: Vec::new(),
		}
	}

	fn composed_weapon_app(loadout: Option<WorldPlayerLoadout>) -> (App, Entity) {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			bevy::input::InputPlugin,
			avian3d::prelude::PhysicsPlugins::default(),
			AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
			bevy::scene::ScenePlugin,
		))
		.insert_resource(TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f32(
			1.0 / 60.0,
		)))
		.insert_resource(PlaygroundMode::Character)
		.insert_resource(WorldGameplayEnabled(true));
		maybraid_gameplay::register_gameplay_runtime(&mut app);
		crate::weapon::configure_runtime(&mut app);
		crate::inventory_drop::configure(&mut app);
		if let Some(loadout) = loadout {
			app.insert_resource(loadout);
		}
		let player = app
			.world_mut()
			.spawn((VegetationPlayer, Transform::IDENTITY, GlobalTransform::IDENTITY))
			.id();
		assign_player(app.world_mut(), player);
		app.world_mut().spawn((VegetationPlayerVisual, CharacterRoot, ChildOf(player)));
		if let Some(mut capabilities) =
			app.world_mut().get_resource_mut::<maybraid_session::SessionCapabilities>()
		{
			capabilities.install_play(true);
		}
		app.finish();
		(app, player)
	}

	#[test]
	fn composed_world_arm_realizes_inventory_items_and_legacy_fallback() -> anyhow::Result<()> {
		for (item, expected) in [
			(Some(InventoryItem::firearm(FirearmMesh::Bullpup)), EquippedItemHandler::Firearm),
			(Some(InventoryItem::standard_grenade()), EquippedItemHandler::Grenade),
			(Some(InventoryItem::Fisticuffs), EquippedItemHandler::Fisticuffs),
			(None, EquippedItemHandler::None),
		] {
			let inventory = item.map(weapon_inventory).unwrap_or_default();
			let (mut app, player) = composed_weapon_app(Some(WorldPlayerLoadout::new(
				"active",
				CharacterAppearance::default(),
				inventory,
			)));
			assert!(app.world().resource::<WorldGameplayEnabled>().0);
			assert!(app.world().get::<ActorControlStatus>(player).is_some());
			for _ in 0..3 {
				app.update();
			}
			assert_eq!(app.world().get::<EquippedItemHandler>(player).copied(), Some(expected));
			assert_eq!(
				app.world().get::<firearm_user::FirearmUser>(player).is_some(),
				expected == EquippedItemHandler::Firearm
			);
			assert_eq!(
				app.world().get::<grenade_user::GrenadeUser>(player).is_some(),
				expected == EquippedItemHandler::Grenade
			);
			assert_eq!(
				app.world().get::<fisticuffs_user::FisticuffsUser>(player).is_some(),
				expected == EquippedItemHandler::Fisticuffs
			);
			assert!(app.world().get::<PendingEquipmentHold>(player).is_none());
		}

		let (mut app, player) = composed_weapon_app(None);
		for _ in 0..3 {
			app.update();
		}
		let firearm = app
			.world()
			.get::<firearm_user::FirearmUser>(player)
			.ok_or_else(|| anyhow::anyhow!("legacy fallback firearm"))?;
		assert_eq!(
			app.world().get::<EquippedItemHandler>(player),
			Some(&EquippedItemHandler::Firearm)
		);
		let live = app
			.world()
			.get::<firearm_user::LiveWeapon>(firearm.held)
			.ok_or_else(|| anyhow::anyhow!("legacy default firearm payload"))?;
		let expected = firearm_user::LiveWeapon::default();
		assert_eq!(live.payload.amount, expected.payload.amount);
		assert_eq!(live.weapon, expected.weapon);
		assert_eq!(live.fire, expected.fire);
		assert_eq!(live.recoil.0, expected.recoil.0);
		assert_eq!(live.recoil_seed, expected.recoil_seed);
		assert_eq!(live.sight_fov, expected.sight_fov);
		Ok(())
	}

	#[test]
	fn bag_primary_change_reconciles_active_fisticuffs_through_item_runtime() -> anyhow::Result<()>
	{
		use fisticuffs_user::{AttackSpec, FisticuffsHitSensor, FisticuffsUser, Side};

		let inventory = Inventory {
			items: vec![InventoryItem::Fisticuffs, InventoryItem::firearm(FirearmMesh::Bullpup)],
			clothing: Vec::new(),
			weapons: vec![0, 1],
			skills: Vec::new(),
			tanzes: Vec::new(),
		};
		let (mut app, player) = composed_weapon_app(Some(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			inventory,
		)));
		app.init_resource::<WorldStrikes>();
		app.add_systems(PostUpdate, record_world_strikes);
		app.update();
		{
			let mut user = app
				.world_mut()
				.get_mut::<FisticuffsUser>(player)
				.ok_or_else(|| anyhow::anyhow!("fisticuffs driver"))?;
			user.state.request(AttackSpec::jab(Side::Left));
			user.state.tick(fisticuffs_user::AttackTiming::JAB.duration_secs * 0.40);
		}
		app.update();
		assert!(app
			.world_mut()
			.query::<&FisticuffsHitSensor>()
			.iter(app.world())
			.any(|sensor| sensor.attacker == player));
		let strikes = app.world().resource::<WorldStrikes>().0;
		let bag = app
			.world()
			.get::<InventoryUser>(player)
			.ok_or_else(|| anyhow::anyhow!("inventory user"))?
			.bag;
		// A changed bag with the same primary must not restart an active jab.
		{
			let mut inventory = app
				.world_mut()
				.get_mut::<Inventory>(bag)
				.ok_or_else(|| anyhow::anyhow!("inventory"))?;
			*inventory = inventory.clone();
		}
		app.update();
		assert_eq!(
			app.world().get::<FisticuffsUser>(player).map(|user| user.state.phase()),
			Some(Some(fisticuffs_user::AttackPhase::Active))
		);
		assert_eq!(app.world().resource::<WorldStrikes>().0, strikes);
		app.world_mut()
			.get_mut::<Inventory>(bag)
			.ok_or_else(|| anyhow::anyhow!("inventory"))?
			.weapons
			.swap(0, 1);
		app.update();
		assert_eq!(app.world().resource::<WorldStrikes>().0, strikes);
		assert!(app.world().get::<FisticuffsUser>(player).is_none());
		let firearm = app
			.world()
			.get::<firearm_user::FirearmUser>(player)
			.ok_or_else(|| anyhow::anyhow!("reconciled firearm"))?;
		assert_eq!(
			app.world().get::<EquippedItemHandler>(player),
			Some(&EquippedItemHandler::Firearm)
		);
		assert_eq!(
			app.world().get::<player::PlayerUse>(player).map(|use_item| use_item.driver),
			Some(firearm.held)
		);
		assert!(!app
			.world_mut()
			.query::<&FisticuffsHitSensor>()
			.iter(app.world())
			.any(|sensor| sensor.attacker == player));
		Ok(())
	}

	#[test]
	fn loadout_replacement_cancels_jab_before_active_in_the_same_frame() -> anyhow::Result<()> {
		use fisticuffs_user::{
			AttackPhase, AttackSpec, AttackTiming, FisticuffsUser, Side, JAB_STARTUP_END,
		};

		let (mut app, player) = composed_weapon_app(Some(WorldPlayerLoadout::new(
			"first",
			CharacterAppearance::default(),
			weapon_inventory(InventoryItem::Fisticuffs),
		)));
		app.init_resource::<WorldStrikes>();
		app.add_systems(PostUpdate, record_world_strikes);
		app.update();
		{
			let mut user = app
				.world_mut()
				.get_mut::<FisticuffsUser>(player)
				.ok_or_else(|| anyhow::anyhow!("initial fisticuffs driver"))?;
			user.state.request(AttackSpec::jab(Side::Left));
			user.state.tick(AttackTiming::JAB.duration_secs * (JAB_STARTUP_END - 0.002));
			assert_eq!(user.state.phase(), Some(AttackPhase::Startup));
		}
		let replacement = WorldPlayerLoadout::new(
			"second",
			CharacterAppearance::default(),
			weapon_inventory(InventoryItem::firearm(FirearmMesh::Reltor)),
		)
		.with_name("Ada");
		app.insert_resource(replacement.clone());
		app.update();
		assert!(app.world().resource::<Time>().delta_secs() > 0.0);
		assert_eq!(app.world().resource::<WorldStrikes>().0, 0);
		assert!(app.world().get::<FisticuffsUser>(player).is_none());
		assert!(app.world().get::<firearm_user::FirearmUser>(player).is_some());
		assert_eq!(app.world().get::<Name>(player).map(Name::as_str), Some("Ada"));
		assert_eq!(
			app.world().get::<AppliedWorldPlayerLoadout>(player).map(|applied| &applied.0),
			Some(&replacement)
		);
		Ok(())
	}

	#[test]
	fn clothing_drop_reholds_fisticuffs_and_removes_the_outgoing_sensor() -> anyhow::Result<()> {
		use character_items::{ClothingMaterial, ClothingMesh, ItemColor};
		use fisticuffs_user::{AttackSpec, FisticuffsHitSensor, FisticuffsUser, Side};

		let inventory = Inventory {
			items: vec![
				InventoryItem::Fisticuffs,
				InventoryItem::clothing(
					ClothingMesh::Pants,
					ClothingMaterial::Tattered,
					ItemColor::Green,
				),
			],
			clothing: vec![1],
			weapons: vec![0],
			skills: Vec::new(),
			tanzes: Vec::new(),
		};
		let (mut app, player) = composed_weapon_app(Some(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			inventory,
		)));
		app.init_resource::<WorldStrikes>();
		app.add_systems(PostUpdate, record_world_strikes);
		app.update();
		let mut user = app
			.world_mut()
			.get_mut::<FisticuffsUser>(player)
			.ok_or_else(|| anyhow::anyhow!("initial fisticuffs driver"))?;
		user.state.request(AttackSpec::jab(Side::Left));
		user.state.tick(fisticuffs_user::AttackTiming::JAB.duration_secs * 0.40);
		drop(user);
		app.update();
		assert!(app
			.world_mut()
			.query::<&FisticuffsHitSensor>()
			.iter(app.world())
			.any(|sensor| sensor.attacker == player));
		let strikes = app.world().resource::<WorldStrikes>().0;

		app.world_mut()
			.resource_mut::<Messages<InventoryDropRequest>>()
			.write(InventoryDropRequest::new(player, 1));
		app.update();
		assert_eq!(app.world().resource::<WorldStrikes>().0, strikes);
		assert!(app.world().get::<FisticuffsUser>(player).is_some());
		assert!(!app
			.world_mut()
			.query::<&FisticuffsHitSensor>()
			.iter(app.world())
			.any(|sensor| sensor.attacker == player));
		let bag = app
			.world()
			.get::<InventoryUser>(player)
			.ok_or_else(|| anyhow::anyhow!("refreshed bag user"))?
			.bag;
		let remaining = app.world().get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert!(remaining.clothing.is_empty());
		Ok(())
	}

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
		let loadout = WorldPlayerLoadout::new("active", CharacterAppearance::default(), inventory)
			.with_name("Ada");
		assert_eq!(loadout.name, "Ada");
		assert_eq!(
			WorldPlayerLoadout::new("active", CharacterAppearance::default(), Inventory::default())
				.with_name("  ")
				.name,
			"Player"
		);
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
		world.insert_resource(
			WorldPlayerLoadout::new("active", CharacterAppearance::default(), Inventory::default())
				.with_name("Ada"),
		);
		let player = world.spawn((VegetationPlayer, WorldPlayerAppearanceRequested)).id();
		assign_player(&mut world, player);
		world.spawn((VegetationPlayerVisual, CharacterRoot, ChildOf(player)));

		world
			.run_system_once(arm_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(world.query::<&RequestSetCharacterAppearance>().iter(&world).count(), 0);
		assert!(world.get::<WorldPlayerAppearanceRequested>(player).is_none());
		assert!(world.get::<maybraid_skill_map::SkillMapUser>(player).is_some());
		assert!(world.get::<TanzUser>(player).is_some());
		assert_eq!(
			world.get::<TanzUser>(player).map(|user| user.mode),
			Some(ControlMode::Standard)
		);
		assert_eq!(
			world.get::<TanzStack>(player).map(|stack| stack.equipped.clone()),
			Some(Vec::new()),
			"runtime init must not grant a hidden starter stack"
		);
		assert_eq!(world.get::<Name>(player).map(Name::as_str), Some("Ada"));
		Ok(())
	}

	#[test]
	fn tanz_stack_projects_inventory_order_and_discards_unequipped_buffer() -> anyhow::Result<()> {
		use crate::weapon::sync_tanz_stack_from_inventory;
		use character_inventory_user::{spawn_bag, InventoryUser};

		let mut world = World::new();
		let mut bag = Inventory {
			items: TanzId::STARTER.into_iter().map(InventoryItem::tanz).collect(),
			clothing: Vec::new(),
			weapons: Vec::new(),
			skills: Vec::new(),
			tanzes: vec![3, 1],
		};
		assert_eq!(bag.equipped_tanzes(), vec![TanzId::ForwardDash, TanzId::LeftStrafe]);
		let player = world
			.spawn((
				TanzUser::default(),
				TanzStack { equipped: vec![TanzId::HealingSalute] },
				TanzExecution {
					buffered: Some(BufferedTanz { id: TanzId::HealingSalute, age: 0.1 }),
					..TanzExecution::default()
				},
			))
			.id();
		spawn_bag(&mut world.commands(), player, bag.clone());
		world.flush();
		world
			.run_system_once(sync_tanz_stack_from_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world.get::<TanzStack>(player).map(|stack| stack.equipped.clone()),
			Some(vec![TanzId::ForwardDash, TanzId::LeftStrafe])
		);
		assert!(world
			.get::<TanzExecution>(player)
			.is_some_and(|execution| execution.buffered.is_none()));

		bag.tanzes.clear();
		let user = world.get::<InventoryUser>(player).ok_or_else(|| anyhow::anyhow!("user"))?;
		*world.get_mut::<Inventory>(user.bag).ok_or_else(|| anyhow::anyhow!("bag"))? = bag;
		world
			.get_mut::<TanzExecution>(player)
			.ok_or_else(|| anyhow::anyhow!("execution"))?
			.buffered = Some(BufferedTanz { id: TanzId::ForwardDash, age: 0.0 });
		world
			.run_system_once(sync_tanz_stack_from_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world.get::<TanzStack>(player).map(|stack| stack.equipped.clone()),
			Some(Vec::new())
		);
		assert!(world
			.get::<TanzExecution>(player)
			.is_some_and(|execution| execution.buffered.is_none()));
		Ok(())
	}

	#[test]
	fn pause_inventory_edit_still_applies_loadout() -> anyhow::Result<()> {
		use crate::InventoryEditCameraFollow;

		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(false));
		world.insert_resource(InventoryEditCameraFollow(true));
		world.insert_resource(
			WorldPlayerLoadout::new("active", CharacterAppearance::default(), Inventory::default())
				.with_name("Ada"),
		);
		let player = world.spawn(VegetationPlayer).id();
		assign_player(&mut world, player);
		world.spawn((VegetationPlayerVisual, CharacterRoot, ChildOf(player)));
		world
			.run_system_once(arm_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.get::<Name>(player).map(Name::as_str), Some("Ada"));
		Ok(())
	}

	fn two_map_bag() -> Inventory {
		use character_items::{SkillMapKind, SkillMapSpec};

		Inventory {
			items: vec![
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)),
			],
			clothing: Vec::new(),
			weapons: Vec::new(),
			skills: vec![0, 1],
			tanzes: Vec::new(),
		}
	}

	#[test]
	fn secondary_equipment_change_does_not_replace_primary_loadout() -> anyhow::Result<()> {
		let mut world = World::new();
		let primary_source = world.spawn_empty().id();
		let secondary_source = world.spawn_empty().id();
		world.insert_resource(PrimaryVirtualPad(primary_source));
		let initial = two_map_bag();
		world.insert_resource(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			initial.clone(),
		));
		let secondary = world
			.spawn((
				VegetationPlayer,
				ActorControlStatus::assigned(InputSource::virtual_pad(secondary_source)),
			))
			.id();
		world.init_resource::<Messages<EquipmentInventoryChanged>>();
		world.write_message(EquipmentInventoryChanged::new(secondary, Inventory::default()));
		world
			.run_system_once(apply_equipment_inventory_to_loadout)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.resource::<WorldPlayerLoadout>().inventory, initial);
		assert!(world.get::<AppliedWorldPlayerLoadout>(secondary).is_none());
		Ok(())
	}

	#[test]
	fn world_arm_queues_selected_item_for_lower_realization() -> anyhow::Result<()> {
		use character_inventory_user::{spawn_bag, InventoryUser};
		use fisticuffs_user::FisticuffsUser;
		use maybraid_equipment::{
			PendingEquipmentHold, PendingEquipmentTeardown, ReconcileEquipmentFromInventory,
		};

		let inventory = Inventory {
			items: vec![InventoryItem::Fisticuffs],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
			tanzes: Vec::new(),
		};
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			inventory.clone(),
		));
		let player = world.spawn((VegetationPlayer, WorldPlayerAppearanceRequested)).id();
		assign_player(&mut world, player);
		world.spawn((VegetationPlayerVisual, CharacterRoot, ChildOf(player)));
		spawn_bag(&mut world.commands(), player, inventory.clone());
		world.flush();
		world
			.run_system_once(arm_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		assert!(world.get::<ReconcileEquipmentFromInventory>(player).is_some());
		assert!(world.get::<PendingEquipmentTeardown>(player).is_some());
		assert!(world.get::<PendingEquipmentHold>(player).is_some());
		assert_eq!(
			world.get::<EquippedItemHandler>(player),
			Some(&EquippedItemHandler::Fisticuffs)
		);
		assert!(world.get::<FisticuffsUser>(player).is_none());
		assert!(world.get::<InventoryUser>(player).is_some());
		Ok(())
	}

	#[test]
	fn y_cycles_the_presented_skill_map_in_the_selected_category() -> anyhow::Result<()> {
		use character_inventory_user::InventoryUser;
		use character_items::{SkillMapKind, SkillMapSpec};
		use maybraid_character_controller::{
			CharacterIntent, CharacterIntentFrame, InventoryCyclingCategory,
		};
		use maybraid_equipment::{cycle_equipped_inventory_queues, EquipmentInventoryChanged};
		use maybraid_skill_map::SkillMapEquip;

		use crate::weapon::{apply_equipment_inventory_to_loadout, sync_skill_map_equip};

		let inventory = two_map_bag();
		let mut world = World::new();
		world.init_resource::<Messages<EquipmentInventoryChanged>>();
		world.insert_resource(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			inventory.clone(),
		));
		let bag = world.spawn(inventory).id();
		let player = world
			.spawn((
				VegetationPlayer,
				InventoryUser::carrying(bag),
				SkillMapEquip::from_spec(Some(SkillMapSpec::new(SkillMapKind::Fireball, 1))),
			))
			.id();
		assign_player(&mut world, player);
		if let Some(mut cycling) = world.get_mut::<InventoryCycling>(player) {
			cycling.select(InventoryCyclingCategory::SkillMaps);
		}
		let source = InputSource::virtual_pad(world.resource::<PrimaryVirtualPad>().0);
		let input = world
			.resource::<InputRouter>()
			.route_actor(source, vec![CharacterIntent::SwapActive])
			.ok_or_else(|| anyhow::anyhow!("player route"))?;
		world.entity_mut(player).insert(CharacterIntentFrame::new(input));
		world
			.run_system_once(cycle_equipped_inventory_queues)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(apply_equipment_inventory_to_loadout)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(sync_skill_map_equip)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_skill_map().and_then(InventoryItem::skill_map_spec),
			Some(SkillMapSpec::new(SkillMapKind::Dumbwave, 2))
		);
		assert_eq!(
			world.get::<SkillMapEquip>(player).copied(),
			Some(SkillMapEquip::from_spec(Some(SkillMapSpec::new(SkillMapKind::Dumbwave, 2))))
		);
		let loadout = world
			.get_resource::<WorldPlayerLoadout>()
			.ok_or_else(|| anyhow::anyhow!("loadout"))?;
		assert_eq!(
			loadout.inventory.primary_skill_map().and_then(InventoryItem::skill_map_spec),
			Some(SkillMapSpec::new(SkillMapKind::Dumbwave, 2))
		);
		Ok(())
	}

	#[test]
	fn detonation_vfx_forwards_grenade_effect_scale() -> Result<(), String> {
		use character_items::GrenadeStats;
		use grenades::{GrenadeDetonated, GrenadeEffect};
		use maybraid_vfx::{
			EffectDefinition, FlipbookAsset, ScaleBounds, VfxFlipbooks, VfxInstance, VfxLibrary,
			FIERY_EXPLOSION,
		};

		let mut world = World::new();
		world.init_resource::<Messages<GrenadeDetonated>>();
		let blank = EffectDefinition::new("stub", []);
		let fiery = EffectDefinition::new(FIERY_EXPLOSION, [])
			.with_scale_bounds(ScaleBounds { min: 0.25, max: 20.0 });
		world.insert_resource(VfxLibrary {
			flipbooks: VfxFlipbooks {
				fire: FlipbookAsset {
					image: Handle::default(),
					grid: UVec2::ONE,
					frame_count: 1,
					playback: 1.0,
				},
				smoke: FlipbookAsset {
					image: Handle::default(),
					grid: UVec2::ONE,
					frame_count: 1,
					playback: 1.0,
				},
				spark: Handle::default(),
			},
			flash: blank.clone(),
			fireball: blank.clone(),
			smoke: blank.clone(),
			sparks: blank,
			fiery_explosion: fiery,
		});
		let stats = GrenadeStats::standard();
		world.resource_mut::<Messages<GrenadeDetonated>>().write(GrenadeDetonated {
			position: Vec3::new(1.0, 2.0, 3.0),
			source: Entity::PLACEHOLDER,
			effect: GrenadeEffect::from_stats(&stats),
			radius: stats.blast_radius,
			damage: stats.blast_damage,
		});
		world
			.run_system_once(super::spawn_detonation_vfx)
			.map_err(|error| format!("{error:?}"))?;
		let mut query = world.query::<(&Transform, &VfxInstance)>();
		let Some((transform, instance)) = query.iter(&world).next() else {
			return Err(String::from("missing detonation vfx"));
		};
		assert_eq!(instance.name, FIERY_EXPLOSION);
		assert!((transform.scale.x - stats.effect_scale).abs() < 1e-4);
		Ok(())
	}
}
