//! Equipped-item dispatch. Inventory selection and the holster commit stamp
//! [`EquippedItemHandler`]; this system is the only Bind writer of
//! [`EquippedItemUse`] / [`EquippedPowerUse`].

use bevy::prelude::*;
use character_items::{Inventory, InventoryItem};
use maybraid_character_controller::{
	ActorControlEvent, ActorControlStatus, ActorIntentEvent, CharacterIntent,
};
use maybraid_controllers_core::ActorTarget;

/// Which UseItem interpreter owns this actor. Written by inventory / swap
/// commit, not by the pad.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EquippedItemHandler {
	#[default]
	None,
	Firearm,
	Grenade,
}

impl EquippedItemHandler {
	pub const fn accepts_use(self) -> bool {
		!matches!(self, Self::None)
	}

	pub const fn is_firearm(self) -> bool {
		matches!(self, Self::Firearm)
	}

	pub const fn is_grenade(self) -> bool {
		matches!(self, Self::Grenade)
	}

	/// Stamp this handler at an equipment boundary. Spawn helpers and inventory
	/// commit both call this so selection cannot drift from the held kit.
	pub fn install(self, entity: &mut EntityCommands) {
		entity.insert(self);
	}

	pub fn clear(entity: &mut EntityCommands) {
		entity.remove::<Self>();
	}

	/// Handler implied by the bag's current primary weapon.
	pub fn from_inventory(inventory: &Inventory) -> Self {
		match inventory.primary_weapon() {
			Some(InventoryItem::Firearm { .. }) => Self::Firearm,
			Some(InventoryItem::Grenade { .. }) => Self::Grenade,
			_ => Self::None,
		}
	}
}

/// Compatibility name for actor-owned routing availability.
pub type RoutedItemActor = ActorControlStatus;

/// Addressed UseItem for the delivered actor's active handler.
///
/// Written every delivered frame so interpreters can tell **held**, **released**,
/// and **not delivered** (capture / suspend) apart. Absence of this message is
/// not a release.
#[derive(EntityEvent, Clone, Copy, Debug, PartialEq)]
pub struct EquippedItemUse {
	pub entity: Entity,
	handler: EquippedItemHandler,
	squeeze: f32,
	held: bool,
}

impl EquippedItemUse {
	pub const fn new(
		actor: ActorTarget,
		handler: EquippedItemHandler,
		squeeze: f32,
		held: bool,
	) -> Self {
		Self { entity: actor.entity(), handler, squeeze, held }
	}

	pub const fn actor(self) -> ActorTarget {
		ActorTarget::new(self.entity)
	}

	pub const fn handler(self) -> EquippedItemHandler {
		self.handler
	}

	pub const fn squeeze(self) -> f32 {
		self.squeeze
	}

	pub const fn held(self) -> bool {
		self.held
	}

	pub fn addresses(self, entity: Entity) -> bool {
		self.entity == entity
	}
}

/// Addressed PowerUseItem. Lubikan activation consumes this, not the
/// broadcast intent.
#[derive(EntityEvent, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EquippedPowerUse {
	pub entity: Entity,
}

impl EquippedPowerUse {
	pub const fn new(actor: ActorTarget) -> Self {
		Self { entity: actor.entity() }
	}

	pub const fn actor(self) -> ActorTarget {
		ActorTarget::new(self.entity)
	}

	pub fn addresses(self, entity: Entity) -> bool {
		self.entity == entity
	}
}

/// Interpret every delivered control frame, including explicit neutral release.
pub fn dispatch_equipped_item(
	event: On<ActorControlEvent>,
	handlers: Query<&EquippedItemHandler>,
	mut commands: Commands,
) {
	let actor = ActorTarget::new(event.entity);
	let handler = handlers.get(event.entity).ok().copied().unwrap_or_default();
	if handler.accepts_use() {
		let sample = *event.input.payload();
		event
			.input
			.map(|_| {
				EquippedItemUse::new(actor, handler, sample.fire_squeeze(), sample.fire_held())
			})
			.trigger(&mut commands, |input| *input.payload());
	}
}

/// Power input is a one-shot semantic intent on the same actor binding.
pub fn dispatch_equipped_power(event: On<ActorIntentEvent>, mut commands: Commands) {
	if !matches!(event.input.payload(), CharacterIntent::PowerUseItem) {
		return;
	}
	let actor = ActorTarget::new(event.entity);
	event
		.input
		.map(|_| EquippedPowerUse::new(actor))
		.trigger(&mut commands, |input| *input.payload());
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_character_controller::ActorControlSample;
	use maybraid_controllers_core::{InputRouter, InputSource};

	#[derive(Resource, Default)]
	struct Seen(Vec<EquippedItemUse>);

	fn record(use_item: On<EquippedItemUse>, mut seen: ResMut<Seen>) {
		seen.0.push(*use_item);
	}

	#[test]
	fn two_actors_receive_only_their_own_fire_and_release() -> anyhow::Result<()> {
		let mut app = App::new();
		app.init_resource::<InputRouter>()
			.init_resource::<Seen>()
			.add_observer(dispatch_equipped_item)
			.add_observer(record);
		let first = app.world_mut().spawn(EquippedItemHandler::Firearm).id();
		let second = app.world_mut().spawn(EquippedItemHandler::Grenade).id();
		let a = InputSource::virtual_pad(app.world_mut().spawn_empty().id());
		let b = InputSource::virtual_pad(app.world_mut().spawn_empty().id());
		{
			let mut router = app.world_mut().resource_mut::<InputRouter>();
			router.assign_actor(a, ActorTarget::new(first));
			router.assign_actor(b, ActorTarget::new(second));
		}
		let first_input = app
			.world()
			.resource::<InputRouter>()
			.route_actor(a, ActorControlSample::default().with_fire(true, 0.8))
			.ok_or_else(|| anyhow::anyhow!("first actor route"))?;
		let second_input = app
			.world()
			.resource::<InputRouter>()
			.route_actor(b, ActorControlSample::default())
			.ok_or_else(|| anyhow::anyhow!("second actor route"))?;
		app.world_mut().trigger(ActorControlEvent { entity: first, input: first_input });
		app.world_mut()
			.trigger(ActorControlEvent { entity: second, input: second_input });
		app.world_mut().flush();
		let seen = &app.world().resource::<Seen>().0;
		assert_eq!(seen.len(), 2);
		assert!(seen.iter().any(|use_item| use_item.addresses(first)
			&& use_item.held()
			&& use_item.handler().is_firearm()));
		assert!(seen.iter().any(|use_item| use_item.addresses(second)
			&& !use_item.held()
			&& use_item.handler().is_grenade()));
		Ok(())
	}
}
