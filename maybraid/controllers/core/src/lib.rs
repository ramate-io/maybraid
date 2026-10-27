//! Targeting, routing, capture, and schedule. No game crate deps.

pub mod actor;
pub mod capture;
pub mod route;
pub mod router;
pub mod schedule;
pub mod source;
pub mod ui;

pub use actor::ActorTarget;
pub use capture::{CaptureKind, CaptureLayer, CaptureStack, CaptureToken, ScopeOwner};
pub use route::{RouteTarget, Routed};
pub use router::{
	publish_routing_events, GestureEffect, HoldEffect, InputRouter, InputRouting, RoutingEvent,
	RoutingKind, SourceAssignment, SourceConnection, Suspension, SuspensionToken,
};
pub use schedule::{CaptureResolveSystems, ControllerPipeline};
pub use source::InputSource;
pub use ui::UiTarget;

use bevy::prelude::*;
use maybraid_input::VirtualPadPlugin;

#[derive(Resource)]
struct ControllersCoreRegistration;

/// Installs the headless controller pipeline and routing lifecycle publisher.
///
/// Call this from controller and session plugins that need routing. Repeated
/// calls are safe, so app behavior does not depend on plugin installation order.
pub fn register_controllers_core(app: &mut App) {
	if app.world().contains_resource::<ControllersCoreRegistration>() {
		return;
	}
	app.world_mut().insert_resource(ControllersCoreRegistration);
	ControllerPipeline::configure(app);
	app.add_message::<RoutingEvent>()
		.init_resource::<InputRouter>()
		.add_systems(Last, publish_routing_events);
}

/// Installs the pipeline graph and an empty [`InputRouter`].
pub struct ControllersCorePlugin;

impl Plugin for ControllersCorePlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		register_controllers_core(app);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::IntoSystem;

	#[test]
	fn plugin_installs_router_and_pipeline() {
		let mut app = App::new();
		app.add_plugins(ControllersCorePlugin);
		assert!(app.world().get_resource::<InputRouter>().is_some());
		assert!(app.world().get_resource::<Messages<RoutingEvent>>().is_some());
	}

	#[test]
	fn headless_registration_is_idempotent_and_does_not_install_hardware() {
		let mut app = App::new();
		register_controllers_core(&mut app);
		register_controllers_core(&mut app);

		assert!(!app.is_plugin_added::<VirtualPadPlugin>());
		assert!(app.world().get_resource::<InputRouter>().is_some());
		assert!(app.world().get_resource::<Messages<RoutingEvent>>().is_some());
		app.update();
		let publisher_type_id = publish_routing_events.system_type_id();
		let publisher_count = app
			.get_schedule(Last)
			.and_then(|schedule| schedule.systems().ok())
			.map(|systems| {
				systems.filter(|(_, system)| system.system_type() == publisher_type_id).count()
			});
		assert_eq!(publisher_count, Some(1));
	}

	#[derive(Resource, Default)]
	struct BindKinds(Vec<RoutingKind>);

	#[derive(Resource, Default)]
	struct LaterKinds(Vec<RoutingKind>);

	fn record_bind(mut events: MessageReader<RoutingEvent>, mut kinds: ResMut<BindKinds>) {
		kinds.0.extend(events.read().map(|event| event.kind()));
	}

	fn disconnect_after_bind(mut routing: InputRouting) {
		routing.disconnect(InputSource::local_pad());
	}

	fn record_later(mut events: MessageReader<RoutingEvent>, mut kinds: ResMut<LaterKinds>) {
		kinds.0.extend(events.read().map(|event| event.kind()));
	}

	#[test]
	fn each_reader_sees_a_lifecycle_event_once_across_stages() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		ControllerPipeline::configure(&mut app);
		app.add_message::<RoutingEvent>()
			.init_resource::<InputRouter>()
			.init_resource::<BindKinds>()
			.init_resource::<LaterKinds>()
			.add_systems(
				Update,
				(
					record_bind.in_set(ControllerPipeline::Bind),
					disconnect_after_bind
						.after(ControllerPipeline::Bind)
						.before(ControllerPipeline::ResetMotorRequests),
					record_later.in_set(ControllerPipeline::ResetMotorRequests),
				),
			);
		app.update();
		assert!(app.world().resource::<BindKinds>().0.is_empty());
		assert_eq!(app.world().resource::<LaterKinds>().0, [RoutingKind::Disconnected]);

		app.world_mut().resource_mut::<BindKinds>().0.clear();
		app.world_mut().resource_mut::<LaterKinds>().0.clear();
		app.update();
		assert_eq!(app.world().resource::<BindKinds>().0, [RoutingKind::Disconnected]);
		assert!(app.world().resource::<LaterKinds>().0.is_empty());
	}
}
