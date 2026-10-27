//! Menu navigation derived from the virtual pad.

use bevy::prelude::*;

use maybraid_virtual_pad_controller::Cardinal;
use maybraid_virtual_pad_controller::PadButton;
use maybraid_virtual_pad_controller::VirtualPad;

/// Routed menu impulse. Triggered on the focused menu entity.
#[derive(EntityEvent, Clone, Copy, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct MenuNavImpulse {
	pub entity: Entity,
	pub nav: MenuNav,
	authorization: Option<maybraid_controllers_core::Routed<()>>,
}

impl MenuNavImpulse {
	pub fn routed(input: maybraid_controllers_core::Routed<MenuNav>) -> Self {
		let entity = input.target().ui_entity().unwrap_or(Entity::PLACEHOLDER);
		let nav = *input.payload();
		Self { entity, nav, authorization: Some(input.map(|_| ())) }
	}
	pub fn deliver<E>(&self, commands: &mut Commands, event: E)
	where
		E: EntityEvent + Send + Sync + 'static,
		for<'a> E::Trigger<'a>: Default,
	{
		if let Some(input) = self.authorization {
			input.queue(commands, move |world, _| world.trigger(event));
		} else {
			commands.trigger(event);
		}
	}
	pub fn new(entity: Entity, nav: MenuNav) -> Self {
		Self { entity, nav, authorization: None }
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuNav {
	Select,
	Back,
	Up,
	Down,
	Left,
	Right,
}

/// Pad faces that fire [`MenuNav::Select`] (catalog equip / unequip).
pub const MENU_SELECT_PAD: [PadButton; 2] = [PadButton::A, PadButton::Start];

/// Menu-map keys that become [`PadButton::Start`], then [`MenuNav::Select`].
pub const MENU_SELECT_KEYS: [KeyCode; 1] = [KeyCode::Enter];

/// Pad face that asks to drop the focused inventory tile (`#1297`).
pub const INVENTORY_DROP_PAD: PadButton = PadButton::X;

/// Keys that hold [`PadButton::X`] so drop matches world interact (E / X).
pub const INVENTORY_DROP_KEYS: [KeyCode; 2] = [KeyCode::KeyE, KeyCode::KeyX];

/// Pad face that unequips the focused inventory tile (`#1356`).
pub const INVENTORY_UNEQUIP_PAD: PadButton = PadButton::Y;

/// Keys that hold [`PadButton::Y`] so unequip matches world swap (Y).
pub const INVENTORY_UNEQUIP_KEYS: [KeyCode; 1] = [KeyCode::KeyY];

impl MenuNav {
	pub fn from_cardinal(cardinal: Cardinal) -> Self {
		match cardinal {
			Cardinal::Up => Self::Up,
			Cardinal::Down => Self::Down,
			Cardinal::Left => Self::Left,
			Cardinal::Right => Self::Right,
		}
	}

	/// `A` or `Start` → Select, `B` → Back, dominant stick / look / dpad → arrows.
	pub fn from_pad(pad: &VirtualPad, threshold: f32) -> Vec<Self> {
		let mut events = Vec::new();
		if pad.just_pressed(PadButton::A) || pad.just_pressed(PadButton::Start) {
			events.push(Self::Select);
		}
		if pad.just_pressed(PadButton::B) {
			events.push(Self::Back);
		}
		if let Some(dir) = Self::direction_from_pad(pad, threshold) {
			events.push(dir);
		}
		events
	}

	pub fn direction_from_pad(pad: &VirtualPad, threshold: f32) -> Option<Self> {
		if pad.just_pressed(PadButton::DpadUp) {
			return Some(Self::Up);
		}
		if pad.just_pressed(PadButton::DpadDown) {
			return Some(Self::Down);
		}
		if pad.just_pressed(PadButton::DpadLeft) {
			return Some(Self::Left);
		}
		if pad.just_pressed(PadButton::DpadRight) {
			return Some(Self::Right);
		}
		Cardinal::from_stick(pad.dpad, threshold)
			.or_else(|| Cardinal::from_stick(pad.move_stick, threshold))
			.map(Self::from_cardinal)
	}
}

/// Current primary-session navigation, mirrored by the explicit host adapter.
/// Scoped controllers consume routed pad events instead of this resource.
#[derive(Resource, Clone, Debug, Default)]
pub struct MenuNavPad {
	pub events: Vec<MenuNav>,
}
impl MenuNavPad {
	pub fn just_pressed(&self, nav: MenuNav) -> bool {
		self.events.contains(&nav)
	}
}

/// Context actions interpreted from the same authorized rich pad frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
	Unequip,
	Drop,
}
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct MenuActionImpulse {
	pub entity: Entity,
	pub action: MenuAction,
	authorization: Option<maybraid_controllers_core::Routed<()>>,
}
impl MenuActionImpulse {
	pub fn new(entity: Entity, action: MenuAction) -> Self {
		Self { entity, action, authorization: None }
	}
	pub fn routed(input: maybraid_controllers_core::Routed<MenuAction>) -> Self {
		Self {
			entity: input.target().ui_entity().unwrap_or(Entity::PLACEHOLDER),
			action: *input.payload(),
			authorization: Some(input.map(|_| ())),
		}
	}
	pub fn deliver<E>(&self, commands: &mut Commands, event: E)
	where
		E: EntityEvent + Send + Sync + 'static,
		for<'a> E::Trigger<'a>: Default,
	{
		if let Some(input) = self.authorization {
			input.queue(commands, move |world, _| world.trigger(event));
		} else {
			commands.trigger(event);
		}
	}
}

/// Dismissal of the modal belonging to the addressed UI session.
#[derive(EntityEvent, Clone, Copy, Debug, PartialEq, Eq)]
#[entity_event(propagate, auto_propagate)]
pub struct MenuModalDismissImpulse {
	pub entity: Entity,
	pub id: crate::MenuModalId,
}

/// A Back request addressed to its owning UI subtree. Pointer requests have no pad source.
#[derive(EntityEvent, Clone, Copy, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct MenuBackRequested {
	pub entity: Entity,
	pub source: Option<Entity>,
	authorization: Option<maybraid_controllers_core::Routed<()>>,
}
impl MenuBackRequested {
	pub fn new(entity: Entity, source: Option<Entity>) -> Self {
		Self { entity, source, authorization: None }
	}
	pub fn routed(input: maybraid_controllers_core::Routed<Option<Entity>>) -> Self {
		Self {
			entity: input.target().ui_entity().unwrap_or(Entity::PLACEHOLDER),
			source: *input.payload(),
			authorization: Some(input.map(|_| ())),
		}
	}
	pub fn deliver<E>(&self, commands: &mut Commands, event: E)
	where
		E: EntityEvent + Send + Sync + 'static,
		for<'a> E::Trigger<'a>: Default,
	{
		if let Some(input) = self.authorization {
			input.queue(commands, move |world, _| world.trigger(event));
		} else {
			commands.trigger(event);
		}
	}
	pub fn queue(
		&self,
		commands: &mut Commands,
		action: impl FnOnce(&mut World) + Send + Sync + 'static,
	) {
		if let Some(input) = self.authorization {
			input.queue(commands, move |world, _| action(world));
		} else {
			commands.queue(action);
		}
	}
}

/// Marks a subtree whose text/modal state is owned by a scoped controller.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct ScopedMenuSession;

/// Input received by the explicitly selected primary menu root.
/// The source identity remains attached to this receiver across host-widget queries.
#[derive(Component, Clone, Debug)]
pub struct PrimaryMenuInput {
    pub source: Entity,
    pub frame: Option<maybraid_virtual_pad_controller::VirtualPadEvent>,
}
impl PrimaryMenuInput {
    pub fn new(source: Entity) -> Self { Self { source, frame: None } }
    pub fn current_frame(&self) -> Option<&maybraid_virtual_pad_controller::VirtualPadEvent> {
        self.frame.as_ref().filter(|frame| frame.source == self.source)
    }
    pub fn under<'a>(entity: Entity, hosts: &'a Query<&Self>, parents: &Query<&ChildOf>) -> Option<&'a Self> {
        std::iter::once(entity).chain(parents.iter_ancestors(entity)).find_map(|entity| hosts.get(entity).ok())
    }
}
