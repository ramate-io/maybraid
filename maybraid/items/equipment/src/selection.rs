//! Inventory category and ordered equipped-queue selection.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::Inventory;
use lubikans::{LubikanAccountId, LubikanBodyOf, LubikanLedger, LubikanOwner};
use maybraid_character_controller::{
	CharacterIntent, CharacterIntentFrame, InventoryCyclingCategory,
};
use maybraid_virtual_pad_controller::PrimaryVirtualPad;

use crate::lifecycle::EquipmentInventoryChanged;

/// HUD projection of the local actor's [`InventoryCycling`]. The actor
/// component is the owner; this resource is a reader-only mirror.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveInventoryCyclingCategory(pub InventoryCyclingCategory);

impl Default for ActiveInventoryCyclingCategory {
	fn default() -> Self {
		Self(InventoryCyclingCategory::Weapons)
	}
}

impl ActiveInventoryCyclingCategory {
	pub const fn from_category(category: InventoryCyclingCategory) -> Self {
		Self(category)
	}

	pub const fn category(self) -> InventoryCyclingCategory {
		self.0
	}

	pub fn project(&mut self, category: InventoryCyclingCategory) {
		self.0 = category;
	}
}

/// Per-actor category owner. One writer: inventory category selection.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct InventoryCycling {
	category: InventoryCyclingCategory,
}

impl InventoryCycling {
	pub const fn new(category: InventoryCyclingCategory) -> Self {
		Self { category }
	}

	pub const fn category(&self) -> InventoryCyclingCategory {
		self.category
	}

	pub fn select(&mut self, category: InventoryCyclingCategory) {
		self.category = category;
	}

	/// Rotate the equipped queue for this category. Weapons holster separately.
	pub fn cycle_bag(&self, bag: &mut Inventory, dir: i8) -> bool {
		match self.category {
			InventoryCyclingCategory::SkillMaps => bag.cycle_skills(dir),
			InventoryCyclingCategory::Weapons | InventoryCyclingCategory::Lubikans => false,
		}
	}
}

/// Advance equipped Lubikan selection across owned accounts. Selection only —
/// never mutates charge.
pub struct EquippedLubikanQueue;

impl EquippedLubikanQueue {
	pub fn cycle(ledger: &mut LubikanLedger, dir: i8) -> bool {
		let order = Self::owned_account_order(ledger);
		if order.len() < 2 {
			return false;
		}
		let current = ledger.selection().resolved();
		let position = order.iter().position(|&id| id == current).unwrap_or(0);
		let len = order.len() as i32;
		let step = if dir < 0 { -1 } else { 1 };
		let next = order[(position as i32 + step).rem_euclid(len) as usize];
		Self::select_owned(ledger, next)
	}

	fn owned_account_order(ledger: &LubikanLedger) -> Vec<LubikanAccountId> {
		ledger.iter().map(|account| account.id()).collect()
	}

	fn select_owned(ledger: &mut LubikanLedger, account: LubikanAccountId) -> bool {
		if account.is_self() { ledger.select(None) } else { ledger.select(Some(account)) }
	}
}

/// D-pad category pick runs before holster begin and queue cycling on the same frame.
pub fn apply_inventory_category_selection(
	mut commands: Commands,
	frames: Query<&CharacterIntentFrame>,
) {
	for frame in &frames {
		let Some(input) = frame.get() else { continue };
		input.clone().queue(&mut commands, |world, input| {
			let Some(actor) = input.target().actor_entity() else { return };
			let selected = input
				.payload()
				.iter()
				.filter_map(|intent| match *intent {
					CharacterIntent::SelectInventoryCyclingCategory(category) => Some(category),
					_ => None,
				})
				.last();
			if let Some(category) = selected {
				if let Some(mut cycling) = world.get_mut::<InventoryCycling>(actor) {
					cycling.select(category);
				} else {
					world.entity_mut(actor).insert(InventoryCycling::new(category));
				}
			}
			let primary = world
				.get_resource::<PrimaryVirtualPad>()
				.is_none_or(|primary| input.source().entity() == Some(primary.0));
			if primary {
				let category = world
					.get::<InventoryCycling>(actor)
					.map(InventoryCycling::category)
					.unwrap_or_default();
				if let Some(mut projected) =
					world.get_resource_mut::<ActiveInventoryCyclingCategory>()
				{
					projected.project(category);
				}
			}
		});
	}
}

/// Cycle only the inventory owned by the actor on the validated delivery.
pub fn cycle_equipped_inventory_queues(
	mut commands: Commands,
	frames: Query<&CharacterIntentFrame>,
) {
	for frame in &frames {
		let Some(input) = frame.get() else { continue };
		input.clone().queue(&mut commands, |world, input| {
			let Some(actor) = input.target().actor_entity() else { return };
			let Some(dir) = queue_step(input.payload()) else { return };
			let category = world
				.get::<InventoryCycling>(actor)
				.map(InventoryCycling::category)
				.unwrap_or_default();
			match category {
				InventoryCyclingCategory::Lubikans => {
					if world.get::<LubikanOwner>(actor).is_none() {
						return;
					}
					if let Some(mut ledger) = LubikanBodyOf::ledger_mut(world, actor) {
						EquippedLubikanQueue::cycle(&mut ledger, dir);
					}
				}
				InventoryCyclingCategory::Weapons => {
					// Weapon queues are committed by EquipmentHolster.
				}
				InventoryCyclingCategory::SkillMaps => {
					let Some(bag_entity) = world.get::<InventoryUser>(actor).map(|user| user.bag)
					else {
						return;
					};
					let Some(mut bag) = world.get_mut::<Inventory>(bag_entity) else { return };
					if !InventoryCycling::new(category).cycle_bag(&mut bag, dir) {
						return;
					}
					let snapshot = bag.clone();
					world.write_message(EquipmentInventoryChanged::new(actor, snapshot));
				}
			}
		});
	}
}

fn queue_step(intents: &[CharacterIntent]) -> Option<i8> {
	for intent in intents {
		match *intent {
			CharacterIntent::SwapActive => return Some(1),
			CharacterIntent::CycleEquippedQueue(step) => return Some(step),
			_ => {}
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use character_items::InventoryItem;
	use lubikans::{LubikanDefinition, LubikanDefinitionId, LubikanLedger, LubikanSelection};
	use maybraid_controllers_core::{ActorTarget, InputRouter, InputSource};

	use super::*;

	fn assign_actor(world: &mut World, extra: impl Bundle) -> Entity {
		world.init_resource::<InputRouter>();
		let actor = world.spawn((InventoryCycling::default(), extra)).id();
		world
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), ActorTarget::new(actor));
		actor
	}

	fn set_frame(
		world: &mut World,
		actor: Entity,
		intents: Vec<CharacterIntent>,
	) -> anyhow::Result<()> {
		let input = world
			.resource::<InputRouter>()
			.route_actor(InputSource::local_pad(), intents)
			.ok_or_else(|| anyhow::anyhow!("actor route"))?;
		world.entity_mut(actor).insert(CharacterIntentFrame::new(input));
		Ok(())
	}

	#[test]
	fn reassigned_source_cannot_apply_a_cached_category() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<ActiveInventoryCyclingCategory>();
		let first = assign_actor(&mut world, ());
		let second = world.spawn(InventoryCycling::default()).id();
		set_frame(
			&mut world,
			first,
			vec![CharacterIntent::SelectInventoryCyclingCategory(
				InventoryCyclingCategory::SkillMaps,
			)],
		)?;
		world
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), ActorTarget::new(second));
		world
			.run_system_once(apply_inventory_category_selection)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world
				.get::<InventoryCycling>(first)
				.ok_or_else(|| anyhow::anyhow!("first actor cycling"))?
				.category(),
			InventoryCyclingCategory::Weapons
		);
		Ok(())
	}

	#[test]
	fn category_selection_updates_the_owner() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<ActiveInventoryCyclingCategory>();
		world.init_resource::<InputRouter>();
		let actor = world.spawn(InventoryCycling::default()).id();
		world
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), ActorTarget::new(actor));
		set_frame(
			&mut world,
			actor,
			vec![CharacterIntent::SelectInventoryCyclingCategory(
				InventoryCyclingCategory::SkillMaps,
			)],
		)?;
		world
			.run_system_once(apply_inventory_category_selection)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world.get::<InventoryCycling>(actor).map(InventoryCycling::category),
			Some(InventoryCyclingCategory::SkillMaps)
		);
		assert_eq!(
			world.resource::<ActiveInventoryCyclingCategory>().category(),
			InventoryCyclingCategory::SkillMaps
		);
		Ok(())
	}

	#[test]
	fn selecting_a_category_does_not_cycle_items() -> anyhow::Result<()> {
		use character_items::{SkillMapKind, SkillMapSpec};

		let inventory = Inventory {
			items: vec![
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)),
			],
			clothing: Vec::new(),
			weapons: Vec::new(),
			skills: vec![0, 1],
			tanzes: Vec::new(),
		};
		let mut world = World::new();
		world.init_resource::<Messages<EquipmentInventoryChanged>>();
		world.insert_resource(ActiveInventoryCyclingCategory(InventoryCyclingCategory::SkillMaps));
		let bag = world.spawn(inventory).id();
		let actor = assign_actor(&mut world, InventoryUser::carrying(bag));
		if let Some(mut cycling) = world.get_mut::<InventoryCycling>(actor) {
			cycling.select(InventoryCyclingCategory::SkillMaps);
		}
		set_frame(
			&mut world,
			actor,
			vec![CharacterIntent::SelectInventoryCyclingCategory(
				InventoryCyclingCategory::Weapons,
			)],
		)?;
		world
			.run_system_once(apply_inventory_category_selection)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(cycle_equipped_inventory_queues)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_skill_map().and_then(InventoryItem::skill_map_spec),
			Some(SkillMapSpec::new(SkillMapKind::Fireball, 1))
		);
		Ok(())
	}

	#[test]
	fn same_frame_category_then_y_uses_the_new_category() -> anyhow::Result<()> {
		use character_items::{SkillMapKind, SkillMapSpec};

		let inventory = Inventory {
			items: vec![
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)),
			],
			clothing: Vec::new(),
			weapons: Vec::new(),
			skills: vec![0, 1],
			tanzes: Vec::new(),
		};
		let mut world = World::new();
		world.init_resource::<Messages<EquipmentInventoryChanged>>();
		world.insert_resource(ActiveInventoryCyclingCategory(InventoryCyclingCategory::Weapons));
		let bag = world.spawn(inventory).id();
		let actor = assign_actor(&mut world, InventoryUser::carrying(bag));
		set_frame(
			&mut world,
			actor,
			vec![
				CharacterIntent::SelectInventoryCyclingCategory(
					InventoryCyclingCategory::SkillMaps,
				),
				CharacterIntent::SwapActive,
			],
		)?;
		world
			.run_system_once(apply_inventory_category_selection)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(cycle_equipped_inventory_queues)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_skill_map().and_then(InventoryItem::skill_map_spec),
			Some(SkillMapSpec::new(SkillMapKind::Dumbwave, 2))
		);
		Ok(())
	}

	#[test]
	fn lubikan_cycle_changes_selection_not_charge() -> anyhow::Result<()> {
		use lubikans::LubikanSelfConfig;

		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(2), 12.0, 4.0);
		let config = LubikanSelfConfig::default();
		let mut ledger = LubikanLedger::with_self(&config);
		let extra_id = ledger.ensure_owned(extra);
		let _ = ledger.apply_to_equipped(5.0);
		assert!(ledger.select(Some(extra_id)));
		let extra_charge = ledger
			.account(extra_id)
			.ok_or_else(|| anyhow::anyhow!("extra"))?
			.charge()
			.current();
		assert!(EquippedLubikanQueue::cycle(&mut ledger, 1));
		assert_eq!(ledger.selection(), LubikanSelection::default());
		let extra_after = ledger
			.account(extra_id)
			.ok_or_else(|| anyhow::anyhow!("extra"))?
			.charge()
			.current();
		assert_eq!(extra_charge, extra_after);
		Ok(())
	}

	#[test]
	fn lubikan_cycle_follows_edited_order_in_both_directions() {
		let mut ledger = LubikanLedger::default();
		let first = ledger.ensure_owned(LubikanDefinition::new(
			LubikanDefinitionId::from_raw(9),
			20.0,
			5.0,
		));
		let second = ledger.ensure_owned(LubikanDefinition::new(
			LubikanDefinitionId::from_raw(2),
			20.0,
			5.0,
		));
		let third = ledger.ensure_owned(LubikanDefinition::new(
			LubikanDefinitionId::from_raw(5),
			20.0,
			5.0,
		));
		let mut order = lubikans::LubikanOrder::from_ledger(&ledger);
		assert!(order.move_account(second, -1));
		assert!(ledger.apply_order(&order));
		for expected in [second, first, third, lubikans::LubikanAccountId::SELF] {
			assert!(EquippedLubikanQueue::cycle(&mut ledger, 1));
			assert_eq!(ledger.selection().resolved(), expected);
		}
		assert!(EquippedLubikanQueue::cycle(&mut ledger, -1));
		assert_eq!(ledger.selection().resolved(), third);
	}

	#[test]
	fn addressed_cycle_resolves_the_stable_character_ledger() -> anyhow::Result<()> {
		let mut world = World::new();
		let mut ledger = LubikanLedger::default();
		let account = ledger.ensure_owned(LubikanDefinition::new(
			LubikanDefinitionId::from_raw(3),
			20.0,
			5.0,
		));
		ledger.apply_to(account, 9.0);
		let character = world.spawn((lubikans::LubikanCharacter, ledger)).id();
		let body = assign_actor(&mut world, (LubikanOwner, LubikanBodyOf(character)));
		world
			.entity_mut(body)
			.insert(InventoryCycling::new(InventoryCyclingCategory::Lubikans));
		set_frame(&mut world, body, vec![CharacterIntent::CycleEquippedQueue(1)])?;
		world
			.run_system_once(cycle_equipped_inventory_queues)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let ledger =
			LubikanBodyOf::ledger(&world, body).ok_or_else(|| anyhow::anyhow!("ledger"))?;
		assert_eq!(ledger.selection().resolved(), account);
		assert_eq!(ledger.equipped().charge().current(), 9.0);
		assert!(world.get::<LubikanLedger>(body).is_none());
		Ok(())
	}

	#[test]
	fn single_owned_lubikan_cycle_is_safe() -> anyhow::Result<()> {
		use lubikans::LubikanSelfConfig;

		let mut ledger = LubikanLedger::with_self(&LubikanSelfConfig::default());
		assert!(!EquippedLubikanQueue::cycle(&mut ledger, 1));
		assert!(ledger.selection().is_self());
		Ok(())
	}
}
