//! Persistent actor ownership and delivery availability from addressed lifecycle events.

use bevy::prelude::*;
use maybraid_controllers_core::{ActorRouteStatus, ActorRoutingEvent, InputSource};

use crate::{CharacterIntentFrame, RoutedActorControl};

/// Missing/default status means an uncontrolled actor, including autonomous NPCs.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActorControlStatus {
	source: Option<InputSource>,
	connected: bool,
	captured: bool,
	suspended: bool,
}

impl ActorControlStatus {
	/// Explicit connected ownership, useful for hosts constructing actor state.
	pub const fn assigned(source: InputSource) -> Self {
		Self { source: Some(source), connected: true, captured: false, suspended: false }
	}

	pub const fn source(self) -> Option<InputSource> {
		self.source
	}
	pub const fn is_controlled(self) -> bool {
		self.source.is_some()
	}
	pub const fn is_connected(self) -> bool {
		self.connected
	}
	pub const fn is_captured(self) -> bool {
		self.captured
	}
	pub const fn is_suspended(self) -> bool {
		self.suspended
	}
	pub const fn is_frozen(self) -> bool {
		self.source.is_some() && self.connected && self.suspended
	}
	pub const fn is_delivered(self) -> bool {
		self.source.is_some() && self.connected && !self.captured && !self.suspended
	}
}

impl From<ActorRouteStatus> for ActorControlStatus {
	fn from(status: ActorRouteStatus) -> Self {
		if !status.assigned {
			return Self::default();
		}
		Self {
			source: Some(status.source),
			connected: status.connected,
			captured: status.captured,
			suspended: status.suspended,
		}
	}
}

pub fn apply_actor_control_status(event: On<ActorRoutingEvent>, mut commands: Commands) {
	let entity = event.entity;
	let status = event.status;
	commands.queue(move |world: &mut World| {
		let state = ActorControlStatus::from(status);
		let Ok(mut actor) = world.get_entity_mut(entity) else {
			return;
		};
		// A delayed retirement from the previous owner cannot clear a newer owner.
		if !status.assigned
			&& actor
				.get::<ActorControlStatus>()
				.and_then(|state| state.source())
				.is_some_and(|source| source != status.source)
		{
			return;
		}
		actor.insert(state);
		if !state.is_delivered() {
			if let Some(mut control) = actor.get_mut::<RoutedActorControl>() {
				control.clear();
			}
			if let Some(mut frame) = actor.get_mut::<CharacterIntentFrame>() {
				frame.clear();
			}
		}
	});
}
