//! The routing boundary publishes already-authorized, target-addressed input.

use std::collections::HashMap;

use bevy::prelude::*;
use maybraid_virtual_pad_controller::{
	PadButton, PrimaryVirtualPad, VirtualPad, VirtualPadController, VirtualPadEvent,
	VirtualPadLifecycle,
};

use maybraid_virtual_pad_controller::pad::PadRearm;

use crate::{ActorTarget, InputRouter, InputSource, Routed, RoutingEvent};

#[derive(Clone, Debug)]
pub struct ActorPadSnapshot {
	pub frame: VirtualPadEvent,
	/// Gameplay is blocked by capture or suspension. Shell commands remain scoped.
	pub captured: bool,
	pub suspended: bool,
	pub trigger_press_threshold: f32,
	pub owns_back: bool,
}

#[derive(EntityEvent, Clone, Debug)]
pub struct ActorPadEvent {
	pub entity: Entity,
	pub input: Routed<ActorPadSnapshot>,
}

#[derive(EntityEvent, Clone, Debug)]
pub struct UiPadEvent {
	pub entity: Entity,
	pub input: Routed<VirtualPadEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorRouteStatus {
	pub source: InputSource,
	pub connected: bool,
	pub captured: bool,
	pub suspended: bool,
	pub assigned: bool,
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct ActorRoutingEvent {
	pub entity: Entity,
	pub event: RoutingEvent,
	pub status: ActorRouteStatus,
}

pub type RoutingLifecycle = ActorRoutingEvent;

pub(crate) fn reconcile_sources(
	primary: Option<Res<PrimaryVirtualPad>>,
	controllers: Query<(Entity, &VirtualPadController)>,
	entities: Query<Entity>,
	mut changes: MessageReader<VirtualPadLifecycle>,
	mut router: ResMut<InputRouter>,
) {
	if let Some(primary) = primary {
		router.set_local_source(primary.0);
	}
	for change in changes.read() {
		let source = InputSource::virtual_pad(change.source);
		if change.connected {
			if !router.connect(source) {
				router.cancel_source(source);
			}
		} else {
			router.disconnect(source);
		}
	}
	let retired_captures: Vec<_> = router
		.captures()
		.layers()
		.filter(|layer| {
			let owner = layer.owner();
			!owner.is_placeholder()
				&& owner
					.actor_entity()
					.or_else(|| owner.ui_entity())
					.is_some_and(|entity| !entities.contains(entity))
		})
		.map(|layer| layer.token())
		.collect();
	for capture in retired_captures {
		router.release_capture(capture);
	}
	for source in router.sources().collect::<Vec<_>>() {
		if router.actor(source).is_some_and(|actor| !entities.contains(actor.entity())) {
			router.clear_actor(source);
		}
		if router.ui(source).is_some_and(|ui| !entities.contains(ui.entity())) {
			router.clear_ui(source);
		}
		if let Some(entity) = source.entity() {
			if controllers.get(entity).is_err() {
				router.disconnect(source);
			}
		}
	}
	for (entity, _) in &controllers {
		// Register a source without assigning an actor or reconnecting it implicitly.
		router.register_source(InputSource::virtual_pad(entity));
	}
}

pub(crate) fn publish_actor_lifecycle(
	router: Res<InputRouter>,
	mut events: MessageReader<RoutingEvent>,
	mut commands: Commands,
) {
	for event in events.read().copied().filter(|event| !event.targeted) {
		queue_actor_lifecycle(&router, event, &mut commands);
	}
}

pub(crate) fn queue_actor_lifecycle(
	router: &InputRouter,
	event: RoutingEvent,
	commands: &mut Commands,
) {
	let source = event.source();
	let mut targets: Vec<ActorTarget> = Vec::new();
	for target in [event.previous(), event.next(), router.actor(source)].into_iter().flatten() {
		if !targets.contains(&target) {
			targets.push(target);
		}
	}
	for actor in targets {
		let status = ActorRouteStatus {
			source,
			connected: router.connected(source),
			captured: router.blocks_actor(source),
			suspended: router.is_suspended(source),
			assigned: router.actor(source) == Some(actor),
		};
		commands.queue(move |world: &mut World| {
			if world.entities().contains(actor.entity()) {
				world.trigger(ActorRoutingEvent { entity: actor.entity(), event, status });
			}
		});
	}
}

/// The routing revision visible when hardware publishes a source frame.
#[derive(Resource, Default)]
pub(crate) struct ProducedBindings(HashMap<Entity, ProducedBinding>);

#[derive(Default)]
struct ProducedBinding {
	sequence: Option<u64>,
	generation: u64,
	rearm_generation: u64,
	physical: VirtualPad,
	rearm: PadRearm,
}

impl ProducedBinding {
	fn rearm_to(&mut self, generation: u64) {
		if self.rearm_generation != generation {
			self.rearm.capture(&self.physical);
			self.rearm_generation = generation;
		}
	}

	fn record(&mut self, frame: &VirtualPadEvent, router: &InputRouter) {
		let source = InputSource::virtual_pad(frame.source);
		self.rearm_to(router.rearm_generation(source));
		self.sequence = Some(frame.sequence);
		self.generation = router.generation(source);
		self.physical = frame.pad.clone();
	}

	fn actor_pad(&mut self, source: InputSource, router: &InputRouter) -> VirtualPad {
		self.rearm_to(router.rearm_generation(source));
		if router.blocks_actor(source) {
			self.rearm.capture(&self.physical);
		}
		// Shell capabilities remain live through gameplay capture and suspension.
		for button in [PadButton::Start, PadButton::Select] {
			self.rearm.allow_button(button);
		}
		for key in [KeyCode::Enter, KeyCode::Tab, KeyCode::KeyM] {
			self.rearm.allow_key(key);
		}
		let mut pad = self.physical.clone();
		self.rearm.apply(&mut pad);
		pad
	}
}

pub(crate) fn record_produced_bindings(
	router: Res<InputRouter>,
	controllers: Query<Entity, With<VirtualPadController>>,
	mut frames: MessageReader<VirtualPadEvent>,
	mut bindings: ResMut<ProducedBindings>,
) {
	bindings.0.retain(|entity, _| controllers.contains(*entity));
	for frame in frames.read() {
		bindings.0.entry(frame.source).or_default().record(frame, &router);
	}
}

#[derive(Resource, Default)]
pub(crate) struct PendingPadDelivery {
	ui: Vec<Routed<VirtualPadEvent>>,
	actor: Vec<Routed<ActorPadSnapshot>>,
}

/// Bind both destinations once, before any UI handler can change assignments.
pub(crate) fn prepare_pad_frames(
	router: Res<InputRouter>,
	mut frames: MessageReader<VirtualPadEvent>,
	controllers: Query<&VirtualPadController>,
	mut bindings: ResMut<ProducedBindings>,
	mut pending: ResMut<PendingPadDelivery>,
) {
	pending.ui.clear();
	pending.actor.clear();
	for frame in frames.read() {
		let Ok(controller) = controllers.get(frame.source) else {
			continue;
		};
		let source = InputSource::virtual_pad(frame.source);
		let binding = bindings.0.entry(frame.source).or_default();
		if binding.sequence != Some(frame.sequence) {
			binding.record(frame, &router);
		}
		let actor_pad = binding.actor_pad(source, &router);
		if binding.generation != router.generation(source) {
			continue;
		}

		if let Some(input) = router.route_ui(source, frame.clone()) {
			pending.ui.push(input.for_frame(frame.sequence));
		}
		let snapshot = ActorPadSnapshot {
			frame: VirtualPadEvent { pad: actor_pad, ..frame.clone() },
			captured: router.blocks_actor(source) || router.is_suspended(source),
			suspended: router.is_suspended(source),
			trigger_press_threshold: controller.config.trigger_press_threshold,
			owns_back: router.captures().owns_back(source),
		};
		if let Some(input) = router.route_shell(source, snapshot) {
			pending.actor.push(input.for_frame(frame.sequence));
		}
	}
}

pub(crate) fn deliver_ui_frames(mut pending: ResMut<PendingPadDelivery>, mut commands: Commands) {
	for input in pending.ui.drain(..) {
		input.trigger(&mut commands, |input| UiPadEvent {
			entity: input.target().ui_entity().unwrap_or(Entity::PLACEHOLDER),
			input,
		});
	}
}

pub(crate) fn deliver_actor_frames(
	mut pending: ResMut<PendingPadDelivery>,
	mut commands: Commands,
) {
	for input in pending.actor.drain(..) {
		input.trigger(&mut commands, |input| ActorPadEvent {
			entity: input.target().actor_entity().unwrap_or(Entity::PLACEHOLDER),
			input,
		});
	}
}

/// Capture owns policy for each source; the primary resource is a temporary mirror.
pub(crate) fn sync_source_policies(
	router: Res<InputRouter>,
	mut controllers: Query<(Entity, &mut VirtualPadController)>,
	mut legacy: Option<ResMut<maybraid_virtual_pad_controller::PadInputPolicy>>,
) {
	for (entity, mut controller) in &mut controllers {
		controller.policy = router.pad_policy_for(InputSource::virtual_pad(entity));
	}
	if let Some(legacy) = legacy.as_mut() {
		**legacy = router.pad_policy();
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use maybraid_virtual_pad_controller::{PadDevices, VirtualPad};

	use crate::{CaptureKind, CaptureResolveSystems, ScopeOwner, UiTarget};

	use super::*;

	#[derive(Resource, Default)]
	struct ActorFrames(Vec<(Entity, bool)>);

	fn record_actor(input: On<ActorPadEvent>, mut frames: ResMut<ActorFrames>) {
		frames.0.push((input.entity, input.input.payload().captured));
	}

	fn dismiss_ui(input: On<UiPadEvent>, mut router: ResMut<InputRouter>, mut commands: Commands) {
		input.input.consume_frame(&mut commands);
		router.release_owner(ScopeOwner::ui(UiTarget::new(input.entity)));
	}

	#[test]
	fn same_frame_ui_dismissal_retires_previously_queued_actor_delivery() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		crate::register_controllers_core(&mut app);
		app.init_resource::<ActorFrames>()
			.add_observer(record_actor)
			.add_observer(dismiss_ui);
		let source_entity = app
			.world_mut()
			.spawn(VirtualPadController::new(PadDevices::keyboard_mouse()))
			.id();
		let source = InputSource::virtual_pad(source_entity);
		let actor = ActorTarget::new(app.world_mut().spawn_empty().id());
		let ui = UiTarget::new(app.world_mut().spawn_empty().id());
		{
			let mut router = app.world_mut().resource_mut::<InputRouter>();
			router.assign_actor(source, actor);
			router.acquire_capture(source, ScopeOwner::ui(ui), CaptureKind::MenuNav);
		}
		app.world_mut()
			.resource_mut::<Messages<VirtualPadEvent>>()
			.write(VirtualPadEvent {
				source: source_entity,
				sequence: 1,
				delta: Duration::from_millis(16),
				pad: VirtualPad::default(),
			});
		app.update();
		assert!(app.world().resource::<ActorFrames>().0.is_empty());
		assert_eq!(app.world().resource::<InputRouter>().delivered_actor(source), Some(actor));
	}

	#[derive(Resource)]
	struct CaptureNow(InputSource, UiTarget);

	fn capture_now(capture: Res<CaptureNow>, mut router: ResMut<InputRouter>) {
		router.acquire_capture(capture.0, ScopeOwner::ui(capture.1), CaptureKind::Overlay);
	}

	#[test]
	fn capture_resolution_precedes_same_frame_source_delivery() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		crate::register_controllers_core(&mut app);
		app.init_resource::<ActorFrames>()
			.add_observer(record_actor)
			.add_systems(Update, capture_now.in_set(CaptureResolveSystems));
		let source_entity = app
			.world_mut()
			.spawn(VirtualPadController::new(PadDevices::keyboard_mouse()))
			.id();
		let source = InputSource::virtual_pad(source_entity);
		let actor = ActorTarget::new(app.world_mut().spawn_empty().id());
		let ui = UiTarget::new(app.world_mut().spawn_empty().id());
		app.insert_resource(CaptureNow(source, ui));
		app.world_mut().resource_mut::<InputRouter>().assign_actor(source, actor);
		app.world_mut()
			.resource_mut::<Messages<VirtualPadEvent>>()
			.write(VirtualPadEvent {
				source: source_entity,
				sequence: 1,
				delta: Duration::from_millis(16),
				pad: VirtualPad::default(),
			});
		app.update();
		assert!(
			app.world().resource::<ActorFrames>().0.is_empty(),
			"capture retires the frame bound before capture resolution"
		);
	}

	#[test]
	fn source_and_actor_despawn_retire_queued_delivery() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		crate::register_controllers_core(&mut app);
		let source_entity = app
			.world_mut()
			.spawn(VirtualPadController::new(PadDevices::keyboard_mouse()))
			.id();
		let source = InputSource::virtual_pad(source_entity);
		let actor = ActorTarget::new(app.world_mut().spawn_empty().id());
		app.world_mut().resource_mut::<InputRouter>().assign_actor(source, actor);
		let old = app
			.world()
			.resource::<InputRouter>()
			.route_actor(source, ())
			.ok_or_else(|| anyhow::anyhow!("initial route"))?;
		app.world_mut().despawn(source_entity);
		app.update();
		assert!(!app.world().resource::<InputRouter>().accepts(&old));
		assert_eq!(app.world().resource::<InputRouter>().actor(source), None);
		let next_source_entity = app
			.world_mut()
			.spawn(VirtualPadController::new(PadDevices::keyboard_mouse()))
			.id();
		let next_source = InputSource::virtual_pad(next_source_entity);
		let next_actor = ActorTarget::new(app.world_mut().spawn_empty().id());
		app.world_mut()
			.resource_mut::<InputRouter>()
			.assign_actor(next_source, next_actor);
		let old = app
			.world()
			.resource::<InputRouter>()
			.route_actor(next_source, ())
			.ok_or_else(|| anyhow::anyhow!("next route"))?;
		app.world_mut().despawn(next_actor.entity());
		app.update();
		assert!(!app.world().resource::<InputRouter>().accepts(&old));
		assert_eq!(app.world().resource::<InputRouter>().actor(next_source), None);
		Ok(())
	}

	#[derive(Resource, Default)]
	struct UiActions(Vec<InputSource>);

	fn consume_ui(input: On<UiPadEvent>, mut commands: Commands) {
		input.input.consume_frame(&mut commands);
		input.input.clone().map(|_| ()).queue(&mut commands, |world, input| {
			world.resource_mut::<UiActions>().0.push(input.source());
		});
	}

	#[test]
	fn select_and_back_consume_only_their_source_frame_and_preserve_ui_actions() {
		use maybraid_virtual_pad_controller::PadButton;
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		crate::register_controllers_core(&mut app);
		app.init_resource::<ActorFrames>()
			.init_resource::<UiActions>()
			.add_observer(record_actor)
			.add_observer(consume_ui);
		let first_entity = app
			.world_mut()
			.spawn(VirtualPadController::new(PadDevices::keyboard_mouse()))
			.id();
		let second_entity = app
			.world_mut()
			.spawn(VirtualPadController::new(PadDevices::keyboard_mouse()))
			.id();
		let first = InputSource::virtual_pad(first_entity);
		let second = InputSource::virtual_pad(second_entity);
		let first_actor = ActorTarget::new(app.world_mut().spawn_empty().id());
		let second_actor = ActorTarget::new(app.world_mut().spawn_empty().id());
		let ui = UiTarget::new(app.world_mut().spawn_empty().id());
		{
			let mut router = app.world_mut().resource_mut::<InputRouter>();
			router.assign_actor(first, first_actor);
			router.assign_actor(second, second_actor);
			router.acquire_capture(first, ScopeOwner::ui(ui), CaptureKind::MenuNav);
		}
		for (sequence, button) in [(1, PadButton::A), (2, PadButton::B)] {
			let mut pad = VirtualPad::default();
			pad.begin_frame();
			pad.hold_digital(button);
			pad.finish_digital();
			for source in [first_entity, second_entity] {
				app.world_mut().resource_mut::<Messages<VirtualPadEvent>>().write(
					VirtualPadEvent {
						source,
						sequence,
						delta: Duration::from_millis(16),
						pad: pad.clone(),
					},
				);
			}
			app.update();
		}
		assert_eq!(app.world().resource::<UiActions>().0, [first, first]);
		assert_eq!(
			app.world().resource::<ActorFrames>().0,
			[(second_actor.entity(), false), (second_actor.entity(), false)]
		);
	}

	#[derive(Resource, Default)]
	struct Statuses(Vec<bool>);

	fn record_status(input: On<ActorRoutingEvent>, mut statuses: ResMut<Statuses>) {
		statuses.0.push(input.status.suspended);
	}

	fn suspend_after_bind(mut routing: crate::InputRouting) {
		routing.suspend(InputSource::local_pad(), ScopeOwner::pause());
	}

	#[test]
	fn late_input_routing_mutations_publish_targeted_status_in_the_same_update() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		crate::register_controllers_core(&mut app);
		app.init_resource::<Statuses>()
			.add_observer(record_status)
			.add_systems(Update, suspend_after_bind.after(crate::ControllerPipeline::Bind));
		let actor = ActorTarget::new(app.world_mut().spawn_empty().id());
		app.world_mut()
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), actor);
		app.update();
		assert_eq!(app.world().resource::<Statuses>().0.last(), Some(&true));
	}

	#[derive(Resource)]
	struct ReassignNow(InputSource, ActorTarget);

	fn reassign_now(change: Res<ReassignNow>, mut routing: crate::InputRouting) {
		routing.assign_actor(change.0, change.1);
	}

	#[test]
	fn a_physical_edge_produced_before_reassignment_never_reaches_the_new_actor() {
		use maybraid_virtual_pad_controller::PadButton;
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		crate::register_controllers_core(&mut app);
		app.init_resource::<ActorFrames>()
			.add_observer(record_actor)
			.add_systems(Update, reassign_now.in_set(CaptureResolveSystems));
		let entity = app
			.world_mut()
			.spawn(VirtualPadController::new(PadDevices::keyboard_mouse()))
			.id();
		let source = InputSource::virtual_pad(entity);
		let previous = ActorTarget::new(app.world_mut().spawn_empty().id());
		let next = ActorTarget::new(app.world_mut().spawn_empty().id());
		app.world_mut().resource_mut::<InputRouter>().assign_actor(source, previous);
		app.insert_resource(ReassignNow(source, next));
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::A);
		pad.finish_digital();
		app.world_mut()
			.resource_mut::<Messages<VirtualPadEvent>>()
			.write(VirtualPadEvent {
				source: entity,
				sequence: 1,
				delta: Duration::from_millis(16),
				pad,
			});
		app.update();
		assert_eq!(app.world().resource::<InputRouter>().actor(source), Some(next));
		assert!(app.world().resource::<ActorFrames>().0.is_empty());
	}

	fn physical_frame(pad: &mut VirtualPad, sequence: u64, held: bool) -> VirtualPadEvent {
		pad.begin_frame();
		if held {
			for button in [PadButton::A, PadButton::B, PadButton::BumperFire] {
				pad.hold_digital(button);
			}
			pad.max_triggers(0.8, 0.9);
			pad.add_move(Vec2::Y);
			pad.add_look_throw(Vec2::X);
		}
		pad.apply_trigger_digital(0.5);
		pad.finish_digital();
		pad.tick_holds(0.1);
		VirtualPadEvent {
			source: Entity::from_bits(1),
			sequence,
			delta: Duration::from_millis(100),
			pad: pad.clone(),
		}
	}

	#[test]
	fn held_controls_rearm_on_actor_transfer_without_affecting_another_source() {
		let source = InputSource::virtual_pad(Entity::from_bits(1));
		let other_source = InputSource::virtual_pad(Entity::from_bits(2));
		let mut router = InputRouter::default();
		router.assign_actor(source, ActorTarget::new(Entity::from_bits(3)));
		router.assign_actor(other_source, ActorTarget::new(Entity::from_bits(4)));
		let mut physical = VirtualPad::default();
		let mut first = ProducedBinding::default();
		let mut other = ProducedBinding::default();
		let frame = physical_frame(&mut physical, 1, true);
		first.record(&frame, &router);
		other.record(&VirtualPadEvent { source: Entity::from_bits(2), ..frame.clone() }, &router);
		assert!(first.actor_pad(source, &router).pressed(PadButton::A));
		router.assign_actor(source, ActorTarget::new(Entity::from_bits(5)));
		let filtered = first.actor_pad(source, &router);
		for button in [PadButton::A, PadButton::B, PadButton::BumperFire, PadButton::TriggerFire] {
			assert!(!filtered.pressed(button));
		}
		assert_eq!(filtered.trigger_fire, 0.0);
		assert_eq!(filtered.trigger_focus, 0.0);
		assert_eq!(filtered.move_stick, Vec2::ZERO);
		assert_eq!(filtered.look_throw, Vec2::ZERO);
		assert!(other.actor_pad(other_source, &router).pressed(PadButton::A));
		first.record(&physical_frame(&mut physical, 2, true), &router);
		assert_eq!(first.actor_pad(source, &router).trigger_fire, 0.0);
		first.record(&physical_frame(&mut physical, 3, false), &router);
		let release = first.actor_pad(source, &router);
		assert!(
			!release.just_released(PadButton::B),
			"a canceled hold must not become a stance tap"
		);
		first.record(&physical_frame(&mut physical, 4, true), &router);
		let fresh = first.actor_pad(source, &router);
		assert!(fresh.just_pressed(PadButton::A));
		assert_eq!(fresh.trigger_fire, 0.9);
	}

	#[test]
	fn capture_release_rearms_holds_but_suspension_freezes_them() {
		let source = InputSource::virtual_pad(Entity::from_bits(1));
		let mut router = InputRouter::default();
		router.assign_actor(source, ActorTarget::new(Entity::from_bits(3)));
		let mut physical = VirtualPad::default();
		let mut binding = ProducedBinding::default();
		binding.record(&physical_frame(&mut physical, 1, true), &router);
		let suspension = router.suspend(source, ScopeOwner::pause());
		assert!(binding.actor_pad(source, &router).pressed(PadButton::A));
		router.resume(suspension);
		assert!(binding.actor_pad(source, &router).pressed(PadButton::A));
		let capture = router.acquire_capture(source, ScopeOwner::console(), CaptureKind::TextEntry);
		assert!(!binding.actor_pad(source, &router).pressed(PadButton::A));
		let mut frame = physical_frame(&mut physical, 2, true);
		frame.pad.suppress_gameplay();
		frame.pad.finish_digital();
		binding.record(&frame, &router);
		binding.actor_pad(source, &router);
		router.release_capture(capture);
		binding.record(&physical_frame(&mut physical, 3, true), &router);
		let blocked = binding.actor_pad(source, &router);
		assert_eq!(blocked.trigger_fire, 0.0);
		assert_eq!(blocked.move_stick, Vec2::ZERO);
		assert!(!blocked.pressed(PadButton::BumperFire));
		binding.record(&physical_frame(&mut physical, 4, false), &router);
		binding.actor_pad(source, &router);
		binding.record(&physical_frame(&mut physical, 5, true), &router);
		assert!(binding.actor_pad(source, &router).just_pressed(PadButton::A));
	}

	#[test]
	fn keys_held_in_menu_layout_rearm_movement_without_blocking_shell_commands() {
		let source = InputSource::virtual_pad(Entity::from_bits(1));
		let mut router = InputRouter::default();
		router.assign_actor(source, ActorTarget::new(Entity::from_bits(3)));
		let capture =
			router.acquire_capture(source, ScopeOwner::map_overlay(), CaptureKind::Overlay);
		let mut physical = VirtualPad::default();
		physical.keys.press(KeyCode::KeyW);
		physical.keys.press(KeyCode::KeyM);
		physical.begin_frame();
		physical.hold_digital(PadButton::Start);
		physical.finish_digital();
		let mut frame = VirtualPadEvent {
			source: Entity::from_bits(1),
			sequence: 1,
			delta: Duration::from_millis(16),
			pad: physical.clone(),
		};
		let mut binding = ProducedBinding::default();
		binding.record(&frame, &router);
		let captured = binding.actor_pad(source, &router);
		assert!(captured.just_pressed(PadButton::Start));
		assert!(captured.keys.just_pressed(KeyCode::KeyM));
		router.release_capture(capture);
		physical.begin_frame();
		physical.add_move(Vec2::Y);
		physical.finish_digital();
		frame.sequence += 1;
		frame.pad = physical;
		binding.record(&frame, &router);
		let released_capture = binding.actor_pad(source, &router);
		assert_eq!(released_capture.move_stick, Vec2::ZERO);
		assert!(!released_capture.keys.pressed(KeyCode::KeyW));
	}

	#[test]
	fn despawned_capture_owner_retires_every_layer() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		crate::register_controllers_core(&mut app);
		let source_entity = app
			.world_mut()
			.spawn(VirtualPadController::new(PadDevices::keyboard_mouse()))
			.id();
		let source = InputSource::virtual_pad(source_entity);
		let owner = UiTarget::new(app.world_mut().spawn_empty().id());
		let overlay = UiTarget::new(app.world_mut().spawn_empty().id());
		let lower = app.world_mut().resource_mut::<InputRouter>().acquire_capture(
			source,
			ScopeOwner::ui(owner),
			CaptureKind::MenuNav,
		);
		app.world_mut().resource_mut::<InputRouter>().acquire_capture(
			source,
			ScopeOwner::ui(overlay),
			CaptureKind::Overlay,
		);
		app.world_mut().despawn(owner.entity());
		app.update();
		let router = app.world().resource::<InputRouter>();
		assert!(router.capture(lower).is_none());
		assert_eq!(
			router.top_capture_for(source).and_then(crate::CaptureLayer::ui_owner),
			Some(overlay)
		);
	}
}
