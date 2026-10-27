//! Rich addressed pad frames → actor-owned physical state and semantic controls.

pub mod intent;
pub mod produce;
pub mod sample;
pub mod status;

pub use intent::{
	ActorIntentEvent, CharacterIntent, CharacterIntentFrame, InventoryCyclingCategory,
};
pub use maybraid_controllers_core::{
	ActorTarget, CaptureKind, ControllerPipeline, HoldEffect, InputRouter, InputRouting,
	InputSource, Routed, RoutingEvent, RoutingKind, ScopeOwner, SourceConnection, UiTarget,
};
pub use sample::{ActorControlEvent, ActorControlSample, FaceButtonSample, RoutedActorControl};
pub use status::ActorControlStatus;

use bevy::prelude::*;
use maybraid_controllers_core::{CaptureResolveSystems, ControllersCorePlugin, PadDeliverySystems};
use maybraid_virtual_pad_controller::VirtualPadPlugin;

use crate::produce::produce_character_intents;

/// Per-actor ordinary locomotion participation. Missing means enabled.
/// Tanz-user writes this once on spawn and again only when that actor's
/// Standard ↔ Tanzer mode actually changes.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrdinaryLocomotion(pub bool);

impl Default for OrdinaryLocomotion {
	fn default() -> Self {
		Self(true)
	}
}

/// Bind-stage alias so leftover `.after(CharacterControlSystems)` item / camera
/// / shell hosts keep working. Motor hosts join [`ControllerPipeline`] sets.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CharacterControlSystems;

/// Ordered Bind work inside [`CharacterControlSystems`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterBindSystems {
	/// Clear ephemeral actor samples before addressed pad delivery.
	Route,
	/// Addressed physical / semantic production has completed.
	Produce,
	/// Address UseItem / PowerUseItem to the delivered actor's handler.
	Dispatch,
}

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ControllersCorePlugin>() {
			app.add_plugins(ControllersCorePlugin);
		}
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		register_character_controller(app);
	}
}

#[derive(Resource)]
struct CharacterControllerRegistration;

/// Install addressed interpretation without installing hardware producers.
pub fn register_character_controller(app: &mut App) {
	if app.world().contains_resource::<CharacterControllerRegistration>() {
		return;
	}
	app.world_mut().insert_resource(CharacterControllerRegistration);
	maybraid_controllers_core::register_controllers_core(app);
	app.configure_sets(Update, CharacterControlSystems.in_set(ControllerPipeline::Bind))
		.configure_sets(Update, PadDeliverySystems.in_set(CharacterControlSystems))
		.configure_sets(
			Update,
			(
				CharacterBindSystems::Route
					.in_set(CharacterControlSystems)
					.after(CaptureResolveSystems)
					.before(PadDeliverySystems),
				CharacterBindSystems::Produce
					.in_set(CharacterControlSystems)
					.after(CharacterBindSystems::Route)
					.after(PadDeliverySystems),
				CharacterBindSystems::Dispatch
					.in_set(CharacterControlSystems)
					.after(CharacterBindSystems::Produce),
			),
		)
		.add_systems(Update, clear_actor_control_samples.in_set(CharacterBindSystems::Route))
		.add_observer(produce_character_intents)
		.add_observer(status::apply_actor_control_status);
}

/// Samples are valid only for the current Bind tick; silence cannot replay edges.
fn clear_actor_control_samples(
	mut controls: Query<&mut RoutedActorControl>,
	mut frames: Query<&mut CharacterIntentFrame>,
) {
	for mut control in &mut controls {
		control.clear();
	}
	for mut frame in &mut frames {
		frame.clear();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::{App, ResMut, Resource, Update};

	#[derive(Resource, Default)]
	struct StageOrder(Vec<&'static str>);

	fn note(label: &'static str) -> impl FnMut(ResMut<StageOrder>) {
		move |mut order: ResMut<StageOrder>| order.0.push(label)
	}

	#[test]
	fn character_bind_runs_inside_the_pipeline() {
		let mut app = App::new();
		ControllerPipeline::configure(&mut app);
		app.init_resource::<StageOrder>()
			.configure_sets(Update, CharacterControlSystems.in_set(ControllerPipeline::Bind))
			.add_systems(
				Update,
				(
					note("bind").in_set(CharacterControlSystems),
					note("reset").in_set(ControllerPipeline::ResetMotorRequests),
				),
			);
		app.update();
		assert_eq!(app.world().resource::<StageOrder>().0, ["bind", "reset"]);
	}
}
