//! Stick-flick detector. Either stick while **RB** is held; a hold is not a flick.
//! Emits actor/session-addressed [`SkillMapFlick`].

use bevy::prelude::*;
use maybraid_character_controller::{ActorControlSample, OrdinaryLocomotion, RoutedActorControl};
use maybraid_controllers_core::{
	ActorRoutingEvent, ActorTarget, GestureEffect, InputSource, RouteBinding, Routed, RoutingEvent,
	RoutingKind,
};

use crate::user::SkillMapUser;
use crate::SkillMapEnabled;

use maybraid_virtual_pad_controller::FlickLane;
pub use maybraid_virtual_pad_controller::{
	FLICK_MAX_SECS, FLICK_MIN_PEAK, FLICK_MIN_SECS, FLICK_REST,
};

/// One completed flick. Direction is the stick throw; length is how hard (0..1+).
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct SkillMapFlick {
	actor: ActorTarget,
	session: Option<Entity>,
	throw: Vec2,
}

/// Pad-derived flick retaining its source binding until the map applies it.
#[derive(Message, Clone, Debug)]
pub struct RoutedSkillMapFlick(pub Routed<SkillMapFlick>);

#[derive(EntityEvent, Clone, Debug)]
pub struct RoutedSkillMapFlickEvent {
	pub entity: Entity,
	pub input: Routed<SkillMapFlick>,
}

pub fn dispatch_routed_flicks(
	mut commands: Commands,
	mut inputs: MessageReader<RoutedSkillMapFlick>,
) {
	for input in inputs.read() {
		input.0.clone().trigger(&mut commands, |input| RoutedSkillMapFlickEvent {
			entity: input.target().actor_entity().unwrap_or(Entity::PLACEHOLDER),
			input,
		});
	}
}

impl SkillMapFlick {
	pub fn new(actor: impl Into<ActorTarget>, session: Option<Entity>, throw: Vec2) -> Self {
		Self { actor: actor.into(), session, throw }
	}

	pub const fn actor(self) -> ActorTarget {
		self.actor
	}

	pub const fn session(self) -> Option<Entity> {
		self.session
	}

	pub const fn throw(self) -> Vec2 {
		self.throw
	}

	pub fn addresses(self, user: Entity, session: Entity) -> bool {
		self.actor.matches(user) && self.session.is_none_or(|target| target == session)
	}
}

/// True while the skill-map steer modifier is down (Xbox **RB** / keyboard **V**)
/// and the Tanzer bumper chord is not held.
pub fn skill_map_steer_held(sample: ActorControlSample) -> bool {
	sample.skill_map_steer_held()
}

/// Watches move and look sticks. One flick per sample, stronger stick wins if both land.
///
/// History is scoped to one `(source, actor)` owner and the session the
/// gesture started against. Changing session cannot complete against the new
/// session. `RoutingEvent` cancel / freeze apply here.
#[derive(Component, Clone, Debug, Default)]
pub struct SkillMapController {
	owner: Option<(InputSource, ActorTarget)>,
	binding: Option<RouteBinding>,
	session: Option<Entity>,
	frozen: bool,
	accept_resume_binding: bool,
	active_secs: f32,
	move_lane: FlickLane,
	look_lane: FlickLane,
}

impl SkillMapController {
	pub fn reset(&mut self) {
		self.owner = None;
		self.binding = None;
		self.session = None;
		self.frozen = false;
		self.accept_resume_binding = false;
		self.active_secs = 0.0;
		self.move_lane.reset();
		self.look_lane.reset();
	}

	pub const fn owner(&self) -> Option<(InputSource, ActorTarget)> {
		self.owner
	}

	pub const fn session(&self) -> Option<Entity> {
		self.session
	}

	pub const fn is_frozen(&self) -> bool {
		self.frozen
	}

	pub fn has_partial(&self) -> bool {
		self.move_lane.is_active() || self.look_lane.is_active()
	}

	/// Gesture time advances only on delivered frames. Suspension leaves a
	/// partial flick at the same age until its owner resumes.
	pub fn advance_clock(&mut self, delta: std::time::Duration) -> f32 {
		if !self.frozen {
			self.active_secs += delta.as_secs_f32();
		}
		self.active_secs
	}

	/// Apply a router hold / gesture contract for this owner.
	pub fn apply_routing(&mut self, event: RoutingEvent) {
		if let Some((source, actor)) = self.owner {
			if source != event.source() || !event.applies_to(actor) {
				return;
			}
		} else if !self.has_partial() && !self.frozen {
			return;
		}
		match event.gesture() {
			GestureEffect::Cancel => self.reset(),
			GestureEffect::Freeze => self.frozen = true,
			GestureEffect::None => {
				if matches!(
					event.kind(),
					RoutingKind::Assigned | RoutingKind::Resumed | RoutingKind::Uncaptured
				) {
					self.frozen = false;
					if event.kind() == RoutingKind::Resumed {
						self.accept_resume_binding = true;
					}
				}
			}
		}
	}

	/// Bind this frame's owner and session. Returns whether sampling may complete.
	///
	/// A session change drops in-flight history so the return-to-rest cannot
	/// emit a flick addressed at the new session.
	pub fn bind_owner(
		&mut self,
		source: InputSource,
		actor: ActorTarget,
		session: Option<Entity>,
	) -> bool {
		self.bind_delivery(source, actor, session, None)
	}

	pub fn bind_delivery(
		&mut self,
		source: InputSource,
		actor: ActorTarget,
		session: Option<Entity>,
		binding: Option<RouteBinding>,
	) -> bool {
		if self.frozen {
			return false;
		}
		if self.owner != Some((source, actor)) {
			self.move_lane.reset();
			self.look_lane.reset();
			self.owner = Some((source, actor));
			self.binding = binding;
			self.session = session;
			self.accept_resume_binding = false;
			return true;
		}
		if self.binding != binding {
			self.binding = binding;
			if !self.accept_resume_binding {
				self.move_lane.reset();
				self.look_lane.reset();
				return false;
			}
		}
		self.accept_resume_binding = false;
		if self.session != session {
			self.move_lane.reset();
			self.look_lane.reset();
			self.session = session;
			return false;
		}
		true
	}

	pub fn sample(&mut self, t: f32, move_stick: Vec2, look_stick: Vec2) -> Option<Vec2> {
		let moved = self.move_lane.sample(t, move_stick);
		let looked = self.look_lane.sample(t, look_stick);
		match (moved, looked) {
			(Some(a), Some(b)) if b.length_squared() > a.length_squared() => Some(b),
			(Some(a), _) => Some(a),
			(None, Some(b)) => Some(b),
			(None, None) => None,
		}
	}
}

/// Apply lifecycle changes only to the actor that owns this flick history.
pub fn apply_skill_map_routing(
	event: On<ActorRoutingEvent>,
	mut controllers: Query<&mut SkillMapController>,
) {
	if let Ok(mut controller) = controllers.get_mut(event.entity) {
		controller.apply_routing(event.event);
	}
}

pub fn detect_skill_map_flicks(
	enabled: Res<SkillMapEnabled>,
	mut flicks: MessageWriter<RoutedSkillMapFlick>,
	mut users: Query<(
		Entity,
		&SkillMapUser,
		Option<&OrdinaryLocomotion>,
		Option<&RoutedActorControl>,
		&mut SkillMapController,
	)>,
) {
	for (entity, user, ordinary, routed, mut controller) in &mut users {
		if !enabled.0 {
			controller.reset();
			continue;
		}
		if controller.is_frozen() {
			continue;
		}
		let Some(input) = routed.and_then(RoutedActorControl::get) else {
			continue;
		};
		if input.target().actor_entity() != Some(entity) {
			continue;
		}
		let sample = *input.payload();
		if ordinary.is_some_and(|flag| !flag.0) || !skill_map_steer_held(sample) {
			controller.reset();
			continue;
		}
		let actor = ActorTarget::new(entity);
		if !controller.bind_delivery(input.source(), actor, Some(user.maps), input.binding()) {
			continue;
		}
		let t = controller.advance_clock(sample.delta());
		if let Some(throw) = controller.sample(t, sample.move_stick(), sample.look_throw()) {
			let flick = SkillMapFlick::new(actor, controller.session(), throw);
			flicks.write(RoutedSkillMapFlick(input.clone().map(|_| flick)));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Resource, Default)]
	struct FlickHits(Vec<SkillMapFlick>);

	fn record_routed_flick(event: On<RoutedSkillMapFlickEvent>, mut hits: ResMut<FlickHits>) {
		hits.0.push(*event.input.payload());
	}

	fn throw(
		controller: &mut SkillMapController,
		t0: f32,
		stick: Vec2,
		on_move: bool,
	) -> Option<Vec2> {
		let mut last = None;
		for (i, mag) in [0.0, 0.75, 1.0, 0.4, 0.0].into_iter().enumerate() {
			let t = t0 + i as f32 * 0.04;
			let v = stick * mag;
			last = if on_move {
				controller.sample(t, v, Vec2::ZERO)
			} else {
				controller.sample(t, Vec2::ZERO, v)
			};
		}
		last
	}

	#[test]
	fn a_fast_throw_on_either_stick_is_a_flick() {
		let mut controller = SkillMapController::default();
		let moved = throw(&mut controller, 0.0, Vec2::X, true).expect("move flick");
		assert!(moved.x > 0.9);
		let looked = throw(&mut controller, 1.0, Vec2::Y, false).expect("look flick");
		assert!(looked.y > 0.9);
	}

	#[test]
	fn a_held_stick_is_not_a_flick() {
		let mut controller = SkillMapController::default();
		assert!(controller.sample(0.0, Vec2::X, Vec2::ZERO).is_none());
		assert!(controller.sample(0.3, Vec2::X, Vec2::ZERO).is_none());
		assert!(controller.sample(0.35, Vec2::ZERO, Vec2::ZERO).is_none());
	}

	#[test]
	fn rb_is_the_steer_hold() {
		assert!(!skill_map_steer_held(ActorControlSample::default()));
		assert!(skill_map_steer_held(ActorControlSample::default().with_bumpers(false, true)));
	}

	#[test]
	fn bumper_chord_is_not_the_steer_hold() {
		assert!(!skill_map_steer_held(ActorControlSample::default().with_bumpers(true, true)));
	}

	#[test]
	fn a_nudge_is_not_a_flick() {
		let mut controller = SkillMapController::default();
		assert!(controller.sample(0.0, Vec2::X * 0.15, Vec2::ZERO).is_none());
		assert!(controller.sample(0.08, Vec2::X * 0.2, Vec2::ZERO).is_none());
		assert!(controller.sample(0.12, Vec2::ZERO, Vec2::ZERO).is_none());
	}

	#[test]
	fn changing_session_does_not_complete_against_the_new_session() {
		let mut controller = SkillMapController::default();
		let actor = ActorTarget::new(Entity::from_bits(1));
		let source = InputSource::local_pad();
		let session_a = Entity::from_bits(10);
		let session_b = Entity::from_bits(11);
		assert!(controller.bind_owner(source, actor, Some(session_a)));
		assert!(controller.sample(0.00, Vec2::X * 0.0, Vec2::ZERO).is_none());
		assert!(controller.sample(0.04, Vec2::X * 0.75, Vec2::ZERO).is_none());
		assert!(controller.sample(0.08, Vec2::X * 1.0, Vec2::ZERO).is_none());
		assert!(controller.has_partial());
		assert!(!controller.bind_owner(source, actor, Some(session_b)));
		assert!(!controller.has_partial());
		assert_eq!(controller.session(), Some(session_b));
		assert!(
			controller.sample(0.12, Vec2::ZERO, Vec2::ZERO).is_none(),
			"return-to-rest after a session change is not a completion"
		);
	}

	#[test]
	fn suspension_preserves_partial_flick_time() {
		let mut controller = SkillMapController::default();
		let t = controller.advance_clock(std::time::Duration::from_millis(40));
		assert!(controller.sample(t, Vec2::X, Vec2::ZERO).is_none());
		controller.frozen = true;
		let frozen_t = controller.advance_clock(std::time::Duration::from_secs(2));
		assert_eq!(frozen_t, t);
		controller.frozen = false;
		let resumed_t = controller.advance_clock(std::time::Duration::from_millis(40));
		assert_eq!(controller.sample(resumed_t, Vec2::ZERO, Vec2::ZERO), Some(Vec2::X));
	}

	#[test]
	fn session_address_still_requires_the_original_actor() {
		let actor = Entity::from_bits(1);
		let other = Entity::from_bits(2);
		let session = Entity::from_bits(3);
		let flick = SkillMapFlick::new(actor, Some(session), Vec2::X);
		assert!(flick.addresses(actor, session));
		assert!(!flick.addresses(other, session));
	}

	#[test]
	fn same_source_new_binding_drops_a_partial_flick() -> anyhow::Result<()> {
		use maybraid_controllers_core::InputRouter;

		let mut router = InputRouter::default();
		let source = InputSource::local_pad();
		let actor = ActorTarget::new(Entity::from_bits(7));
		let session = Some(Entity::from_bits(8));
		router.assign_actor(source, actor);
		let initial = router
			.route_actor(source, ())
			.ok_or_else(|| anyhow::anyhow!("initial route"))?
			.binding();
		let mut controller = SkillMapController::default();
		assert!(controller.bind_delivery(source, actor, session, initial));
		assert!(controller.sample(0.04, Vec2::X, Vec2::ZERO).is_none());
		assert!(controller.has_partial());
		router.cancel_source(source);
		let changed = router
			.route_actor(source, ())
			.ok_or_else(|| anyhow::anyhow!("changed route"))?
			.binding();
		assert_ne!(initial, changed);
		assert!(!controller.bind_delivery(source, actor, session, changed));
		assert!(!controller.has_partial());
		assert!(controller.sample(0.08, Vec2::ZERO, Vec2::ZERO).is_none());
		Ok(())
	}

	#[test]
	fn simultaneous_sources_keep_flick_histories_separate() -> anyhow::Result<()> {
		use maybraid_controllers_core::InputRouter;

		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<InputRouter>()
			.init_resource::<SkillMapEnabled>()
			.init_resource::<FlickHits>()
			.add_message::<SkillMapFlick>()
			.add_message::<RoutedSkillMapFlick>()
			.add_observer(record_routed_flick)
			.add_systems(Update, (detect_skill_map_flicks, dispatch_routed_flicks).chain());
		let session_a = app.world_mut().spawn_empty().id();
		let session_b = app.world_mut().spawn_empty().id();
		let actor_a = app
			.world_mut()
			.spawn((SkillMapUser::mapping(session_a), OrdinaryLocomotion(true)))
			.id();
		let actor_b = app
			.world_mut()
			.spawn((SkillMapUser::mapping(session_b), OrdinaryLocomotion(true)))
			.id();
		let second_pad = app.world_mut().spawn_empty().id();
		let source_a = InputSource::local_pad();
		let source_b = InputSource::virtual_pad(second_pad);
		{
			let mut router = app.world_mut().resource_mut::<InputRouter>();
			router.assign_actor(source_a, ActorTarget::new(actor_a));
			router.assign_actor(source_b, ActorTarget::new(actor_b));
		}
		for (a, b) in [(Vec2::X, Vec2::Y), (Vec2::ZERO, Vec2::ZERO)] {
			for (source, actor, stick) in [(source_a, actor_a, a), (source_b, actor_b, b)] {
				let sample = ActorControlSample::default()
					.with_bumpers(false, true)
					.with_move_stick(stick)
					.with_delta(std::time::Duration::from_millis(40));
				let routed = app
					.world()
					.resource::<InputRouter>()
					.route_actor(source, sample)
					.ok_or_else(|| anyhow::anyhow!("missing route for {source:?}"))?;
				let mut control = RoutedActorControl::default();
				control.set(routed);
				app.world_mut().entity_mut(actor).insert(control);
			}
			app.update();
		}
		let flicks = &app.world().resource::<FlickHits>().0;
		assert_eq!(flicks.len(), 2);
		assert!(flicks
			.iter()
			.any(|flick| flick.addresses(actor_a, session_a) && flick.throw() == Vec2::X));
		assert!(flicks
			.iter()
			.any(|flick| flick.addresses(actor_b, session_b) && flick.throw() == Vec2::Y));
		Ok(())
	}

	#[test]
	fn reassignment_cancels_a_partial_flick() {
		use maybraid_controllers_core::InputRouter;

		let mut controller = SkillMapController::default();
		let source = InputSource::local_pad();
		let actor_a = ActorTarget::new(Entity::from_bits(1));
		assert!(controller.bind_owner(source, actor_a, Some(Entity::from_bits(9))));
		assert!(controller.sample(0.0, Vec2::X, Vec2::ZERO).is_none());
		assert!(controller.has_partial());
		let mut router = InputRouter::default();
		router.assign_actor(source, actor_a);
		router.take_events();
		router.assign_actor(source, ActorTarget::new(Entity::from_bits(2)));
		let event = router.take_events().last().copied().expect("reassigned");
		assert_eq!(event.kind(), RoutingKind::Reassigned);
		assert_eq!(event.gesture(), GestureEffect::Cancel);
		controller.apply_routing(event);
		assert!(!controller.has_partial());
		assert!(controller.owner().is_none());
	}
}
