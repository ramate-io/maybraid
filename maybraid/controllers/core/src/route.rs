//! Explicit source → target addressing. No query-wide broadcast.

use bevy::prelude::*;

use crate::actor::ActorTarget;
use crate::source::InputSource;
use crate::ui::UiTarget;

/// Destination of a routed sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RouteTarget {
	Actor(ActorTarget),
	Ui(UiTarget),
}

impl RouteTarget {
	pub const fn actor(target: ActorTarget) -> Self {
		Self::Actor(target)
	}

	pub const fn ui(target: UiTarget) -> Self {
		Self::Ui(target)
	}

	pub const fn actor_entity(self) -> Option<bevy::prelude::Entity> {
		match self {
			Self::Actor(target) => Some(target.entity()),
			Self::Ui(_) => None,
		}
	}

	pub const fn ui_entity(self) -> Option<bevy::prelude::Entity> {
		match self {
			Self::Ui(target) => Some(target.entity()),
			Self::Actor(_) => None,
		}
	}
}

/// Sample or event with an explicit source and destination.
#[derive(Clone, Copy, Debug)]
pub struct Routed<T> {
	source: InputSource,
	target: RouteTarget,
	payload: T,
	generation: Option<u64>,
	sequence: Option<u64>,
	scope: DeliveryScope,
}

impl<T> Routed<T> {
	pub const fn new(source: InputSource, target: RouteTarget, payload: T) -> Self {
		Self {
			source,
			target,
			payload,
			generation: None,
			sequence: None,
			scope: DeliveryScope::Actor,
		}
	}

	pub const fn to_ui(source: InputSource, target: UiTarget, payload: T) -> Self {
		Self::new(source, RouteTarget::ui(target), payload)
	}

	pub const fn to_actor(source: InputSource, target: ActorTarget, payload: T) -> Self {
		Self::new(source, RouteTarget::actor(target), payload)
	}

	pub const fn source(&self) -> InputSource {
		self.source
	}

	pub const fn target(&self) -> RouteTarget {
		self.target
	}

	pub const fn payload(&self) -> &T {
		&self.payload
	}

	pub fn into_payload(self) -> T {
		self.payload
	}

	/// Consume this UI frame for actor/shell input without retiring the UI action.
	/// The routing boundary flushes UI commands before actor delivery.
	pub fn consume_frame(&self, commands: &mut Commands) {
		if !matches!(self.scope, DeliveryScope::Ui) {
			return;
		}
		let token = Routed {
			source: self.source,
			target: self.target,
			payload: (),
			generation: self.generation,
			sequence: self.sequence,
			scope: self.scope,
		};
		token.queue(commands, |world, input| {
			if let Some(sequence) = input.sequence {
				world
					.resource_mut::<crate::InputRouter>()
					.consumed_frames
					.insert(input.source, sequence);
			}
		});
	}

	pub(crate) fn for_frame(mut self, sequence: u64) -> Self {
		self.sequence = Some(sequence);
		self
	}

	/// Opaque provenance for receiver-owned held/repeat state.
	pub fn binding(&self) -> Option<RouteBinding> {
		self.generation.map(|generation| RouteBinding {
			source: self.source,
			target: self.target,
			generation,
		})
	}

	pub fn map<U>(self, map: impl FnOnce(T) -> U) -> Routed<U> {
		Routed {
			source: self.source,
			target: self.target,
			payload: map(self.payload),
			generation: self.generation,
			sequence: self.sequence,
			scope: self.scope,
		}
	}
}

/// Identity of one delivery session. Equality detects capture/focus/assignment retirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteBinding {
	source: InputSource,
	target: RouteTarget,
	generation: u64,
}

/// Permission captured when a route is produced, retained by payload mapping.
#[derive(Clone, Copy, Debug)]
pub(crate) enum DeliveryScope {
	Actor,
	Shell,
	Ui,
}

impl<T: Send + Sync + 'static> Routed<T> {
	/// Validate this original binding immediately before deferred dispatch.
	/// Receivers never have to poll the router or reconstruct an actor.
	pub fn queue(
		self,
		commands: &mut Commands,
		deliver: impl FnOnce(&mut World, Self) + Send + Sync + 'static,
	) {
		commands.queue(move |world: &mut World| {
			if world
				.get_resource::<crate::InputRouter>()
				.is_some_and(|router| router.accepts(&self))
			{
				let target = match self.target {
					RouteTarget::Actor(target) => target.entity(),
					RouteTarget::Ui(target) => target.entity(),
				};
				if world.entities().contains(target) {
					deliver(world, self);
				}
			}
		});
	}

	/// Queue a target-addressed event through the same central authorization gate.
	pub fn trigger<E>(
		self,
		commands: &mut Commands,
		event: impl FnOnce(Self) -> E + Send + Sync + 'static,
	) where
		E: EntityEvent,
		for<'a> E::Trigger<'a>: Default,
	{
		self.queue(commands, move |world, routed| {
			let expected = match routed.target {
				RouteTarget::Actor(target) => target.entity(),
				RouteTarget::Ui(target) => target.entity(),
			};
			let event = event(routed);
			if event.event_target() == expected {
				world.trigger(event);
			}
		});
	}
}

impl crate::InputRouter {
	pub fn route_actor<T>(&self, source: InputSource, payload: T) -> Option<Routed<T>> {
		let target = self.delivered_actor(source)?;
		Some(self.stamp(source, RouteTarget::Actor(target), payload, DeliveryScope::Actor))
	}

	/// Explicit shell capability: assigned actor even during capture or freeze.
	/// Gameplay interpretation must honor ActorPadSnapshot's capture flags.
	pub fn route_shell<T>(&self, source: InputSource, payload: T) -> Option<Routed<T>> {
		if !self.connected(source) {
			return None;
		}
		let target = self.actor(source)?;
		Some(self.stamp(source, RouteTarget::Actor(target), payload, DeliveryScope::Shell))
	}

	pub fn route_ui<T>(&self, source: InputSource, payload: T) -> Option<Routed<T>> {
		let target = self.delivered_ui(source)?;
		Some(self.stamp(source, RouteTarget::Ui(target), payload, DeliveryScope::Ui))
	}

	pub fn accepts<T>(&self, input: &Routed<T>) -> bool {
		if input.generation != Some(self.generation(input.source)) || !self.connected(input.source)
		{
			return false;
		}
		if !matches!(input.scope, DeliveryScope::Ui)
			&& input
				.sequence
				.is_some_and(|sequence| self.consumed_frames.get(&input.source) == Some(&sequence))
		{
			return false;
		}
		match (input.scope, input.target) {
			(DeliveryScope::Actor, RouteTarget::Actor(target)) => {
				self.delivered_actor(input.source) == Some(target)
			}
			(DeliveryScope::Shell, RouteTarget::Actor(target)) => {
				self.actor(input.source) == Some(target)
			}
			(DeliveryScope::Ui, RouteTarget::Ui(target)) => {
				self.delivered_ui(input.source) == Some(target)
			}
			_ => false,
		}
	}

	fn stamp<T>(
		&self,
		source: InputSource,
		target: RouteTarget,
		payload: T,
		scope: DeliveryScope,
	) -> Routed<T> {
		let source = self.canonical(source);
		Routed {
			source,
			target,
			payload,
			generation: Some(self.generation(source)),
			sequence: None,
			scope,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Entity;

	#[test]
	fn routed_menu_nav_is_ui_local() {
		let menu = UiTarget::new(Entity::from_bits(11));
		let routed = Routed::to_ui(InputSource::local_pad(), menu, "down");
		assert_eq!(routed.source(), InputSource::local_pad());
		assert_eq!(routed.target().ui_entity(), Some(menu.entity()));
		assert_eq!(routed.target().actor_entity(), None);
		assert_eq!(*routed.payload(), "down");
	}

	#[test]
	fn routed_actor_sample_is_actor_local() {
		let actor = ActorTarget::new(Entity::from_bits(5));
		let routed = Routed::to_actor(InputSource::local_pad(), actor, 1u8);
		assert_eq!(routed.target().actor_entity(), Some(actor.entity()));
		assert_eq!(routed.into_payload(), 1);
	}

	#[derive(Resource, Default)]
	struct Hits(Vec<(Entity, u8)>);

	#[test]
	fn queued_old_actions_never_retarget_after_assignment_retirement() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<crate::InputRouter>();
		world.init_resource::<Hits>();
		let original = ActorTarget::new(world.spawn_empty().id());
		let replacement = ActorTarget::new(world.spawn_empty().id());
		let source = InputSource::virtual_pad(world.spawn_empty().id());
		world.resource_mut::<crate::InputRouter>().assign_actor(source, original);
		let input = world
			.resource::<crate::InputRouter>()
			.route_actor(source, 7)
			.ok_or_else(|| anyhow::anyhow!("assigned route"))?;
		input.queue(&mut world.commands(), |world, input| {
			if let Some(actor) = input.target().actor_entity() {
				world.resource_mut::<Hits>().0.push((actor, input.into_payload()));
			}
		});
		world.resource_mut::<crate::InputRouter>().assign_actor(source, replacement);
		world.resource_mut::<crate::InputRouter>().assign_actor(source, original);
		world.flush();
		assert!(
			world.resource::<Hits>().0.is_empty(),
			"returning to an old actor must not revive its old session"
		);
		Ok(())
	}

	#[test]
	fn capture_and_suspension_retire_only_the_affected_source() -> anyhow::Result<()> {
		use crate::{CaptureKind, ScopeOwner, UiTarget};
		let mut world = World::new();
		world.init_resource::<crate::InputRouter>();
		let first_actor = ActorTarget::new(world.spawn_empty().id());
		let second_actor = ActorTarget::new(world.spawn_empty().id());
		let first = InputSource::virtual_pad(world.spawn_empty().id());
		let second = InputSource::virtual_pad(world.spawn_empty().id());
		let menu = UiTarget::new(world.spawn_empty().id());
		let mut router = world.resource_mut::<crate::InputRouter>();
		router.assign_actor(first, first_actor);
		router.assign_actor(second, second_actor);
		let first_input =
			router.route_actor(first, ()).ok_or_else(|| anyhow::anyhow!("first route"))?;
		let second_input =
			router.route_actor(second, ()).ok_or_else(|| anyhow::anyhow!("second route"))?;
		let capture = router.acquire_capture(first, ScopeOwner::ui(menu), CaptureKind::MenuNav);
		assert!(!router.accepts(&first_input));
		assert!(router.accepts(&second_input));
		assert_eq!(router.delivered_ui(first), Some(menu));
		assert_eq!(router.delivered_ui(second), None);
		router.release_capture(capture);
		assert!(!router.accepts(&first_input), "capture release must not revive queued holds");
		let first_input =
			router.route_actor(first, ()).ok_or_else(|| anyhow::anyhow!("restored route"))?;
		let suspension = router.suspend(first, ScopeOwner::pause());
		assert!(!router.accepts(&first_input));
		assert!(router.accepts(&second_input));
		router.resume(suspension);
		assert!(!router.accepts(&first_input));
		Ok(())
	}

	#[test]
	fn assigning_an_owned_actor_transfers_ownership_and_retires_old_delivery() -> anyhow::Result<()>
	{
		let mut router = crate::InputRouter::default();
		let actor = ActorTarget::new(Entity::from_bits(9));
		let first = InputSource::virtual_pad(Entity::from_bits(1));
		let second = InputSource::virtual_pad(Entity::from_bits(2));
		router.assign_actor(first, actor);
		let old = router.route_actor(first, ()).ok_or_else(|| anyhow::anyhow!("first route"))?;
		router.assign_actor(second, actor);
		assert_eq!(router.actor(first), None);
		assert_eq!(router.actor(second), Some(actor));
		assert!(!router.accepts(&old));
		assert!(router
			.take_events()
			.iter()
			.any(|event| event.source() == first
				&& event.hold() == crate::HoldEffect::ReleaseNeutral));
		Ok(())
	}

	#[test]
	fn ui_owner_replacement_invalidates_the_previous_generation() -> anyhow::Result<()> {
		use crate::{CaptureKind, ScopeOwner, UiTarget};
		let mut router = crate::InputRouter::default();
		let source = InputSource::virtual_pad(Entity::from_bits(1));
		let mut capture = None;
		router.replace_owned_capture(
			&mut capture,
			source,
			Some(ScopeOwner::ui(UiTarget::new(Entity::from_bits(2)))),
			CaptureKind::MenuNav,
		);
		let old = router.route_ui(source, ()).ok_or_else(|| anyhow::anyhow!("first menu"))?;
		router.replace_owned_capture(
			&mut capture,
			source,
			Some(ScopeOwner::ui(UiTarget::new(Entity::from_bits(3)))),
			CaptureKind::MenuNav,
		);
		assert!(!router.accepts(&old));
		Ok(())
	}

	#[test]
	fn stale_ui_consumption_cannot_cancel_a_replacement_binding() -> anyhow::Result<()> {
		use crate::{CaptureKind, ScopeOwner, UiTarget};
		let mut world = World::new();
		world.init_resource::<crate::InputRouter>();
		let source = InputSource::virtual_pad(world.spawn_empty().id());
		let actor = ActorTarget::new(world.spawn_empty().id());
		let first_ui = UiTarget::new(world.spawn_empty().id());
		let next_ui = UiTarget::new(world.spawn_empty().id());
		let mut capture = None;
		{
			let mut router = world.resource_mut::<crate::InputRouter>();
			router.assign_actor(source, actor);
			router.replace_owned_capture(
				&mut capture,
				source,
				Some(ScopeOwner::ui(first_ui)),
				CaptureKind::MenuNav,
			);
		}
		let old_ui = world
			.resource::<crate::InputRouter>()
			.route_ui(source, ())
			.ok_or_else(|| anyhow::anyhow!("old ui"))?
			.for_frame(3);
		old_ui.consume_frame(&mut world.commands());
		world.resource_mut::<crate::InputRouter>().replace_owned_capture(
			&mut capture,
			source,
			Some(ScopeOwner::ui(next_ui)),
			CaptureKind::MenuNav,
		);
		let current = world
			.resource::<crate::InputRouter>()
			.route_shell(source, ())
			.ok_or_else(|| anyhow::anyhow!("current shell"))?
			.for_frame(3);
		world.flush();
		assert!(world.resource::<crate::InputRouter>().accepts(&current));
		Ok(())
	}

	#[test]
	fn suspension_preserves_explicit_shell_capability_and_blocks_gameplay() -> anyhow::Result<()> {
		let mut router = crate::InputRouter::default();
		let source = InputSource::local_pad();
		router.assign_actor(source, ActorTarget::new(Entity::from_bits(4)));
		router.suspend(source, crate::ScopeOwner::pause());
		assert!(router.route_actor(source, ()).is_none());
		let shell = router
			.route_shell(source, ())
			.ok_or_else(|| anyhow::anyhow!("paused shell route"))?;
		assert!(router.accepts(&shell));
		Ok(())
	}
}
