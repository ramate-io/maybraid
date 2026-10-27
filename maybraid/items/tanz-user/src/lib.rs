//! Tanzer control mode: left-stick / A / B gestures become equipped Tanzes
//! and always advance the Skill Map.

mod actions;
mod callout;
mod execute;
mod hud;
mod input;
mod pose;
mod stack;

use bevy::prelude::*;
use characters::CharacterMotionSystems;
use damage::Downed;
#[cfg(test)]
use maybraid_character_controller::RoutedActorControl;
use maybraid_character_controller::{ActorControlEvent, CharacterBindSystems, OrdinaryLocomotion};
use maybraid_controllers_core::{
	ActorRoutingEvent, ActorTarget, ControllerPipeline, ControllersCorePlugin, PadDeliverySystems,
	RoutingEvent,
};
use maybraid_skill_map::{RoutedSkillMapFlick, SkillMapFlick, SkillMapSystems, SkillMapUser};
use menu_components::sync_loading_bar_fill;
#[cfg(test)]
use player::Player;
use player::{MotorRequestStage, PlayerLook, PlayerSystems};

pub use actions::{
	dash_ground_speed, forward_dash, healing_salute, left_strafe, right_strafe, starter_stack,
	TanzAction, DASH_BRAKE_END, DASH_COMPRESS_END, DASH_HOLD_END, DASH_PUSH_END,
	FORWARD_DASH_CRUISE, POWER_STRAFE_CRUISE,
};
pub use callout::{TanzCallout, TanzCalloutView, TANZ_CALLOUT_SECS};
pub use execute::{
	advance_tanz_execution, apply_tanz_movement, dispatch_routed_executions, drive_tanz_animation,
	match_tanz_inputs, submit_tanz_executions, ActiveTanz, BufferedTanz, ExecuteTanz,
	RoutedExecuteTanz, RoutedExecuteTanzEvent, TanzExecution, TanzPhase,
};
pub use input::{
	CommittedGesture, ModeToggle, RoutedTanzInput, TanzButton, TanzFaceSample, TanzInput,
	TanzRecognizer, TanzRecognizerConfig, TANZ_B_HOLD_SECS, TANZ_TOGGLE_SECS,
};
pub use pose::{request_inventory_tanz_icons, TanzPose};
pub use stack::{TanzDefinition, TanzDefinitionExt, TanzId, TanzInputRange, TanzStack, TanzTiming};

/// Standard locomotion versus Tanzer gesture play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlMode {
	#[default]
	Standard,
	Tanzer,
}

impl ControlMode {
	pub fn toggle(self) -> Self {
		match self {
			Self::Standard => Self::Tanzer,
			Self::Tanzer => Self::Standard,
		}
	}

	pub fn is_tanzer(self) -> bool {
		self == Self::Tanzer
	}

	pub fn script_id(self) -> &'static str {
		match self {
			Self::Standard => "standard",
			Self::Tanzer => "tanzer",
		}
	}

	pub fn from_script_id(raw: &str) -> Option<Self> {
		match raw.trim().to_ascii_lowercase().as_str() {
			"standard" | "loco" | "locomotion" => Some(Self::Standard),
			"tanzer" | "tanz" => Some(Self::Tanzer),
			_ => None,
		}
	}

	/// User-facing mode label (`Laufer` / `Tanzer`).
	pub fn mode_label(self) -> &'static str {
		match self {
			Self::Standard => "Laufer",
			Self::Tanzer => "Tanzer",
		}
	}
}

/// Per-character Tanz ownership. Mode, settings, and the temporal recognizer
/// live here so a mode change can reset stale gestures in one place.
#[derive(Component, Clone, Debug)]
pub struct TanzUser {
	pub mode: ControlMode,
	pub settings: TanzUserSettings,
	pub recognizer: TanzRecognizer,
	pub toggle: ModeToggle,
}

impl Default for TanzUser {
	fn default() -> Self {
		Self {
			mode: ControlMode::Standard,
			settings: TanzUserSettings::default(),
			recognizer: TanzRecognizer::default(),
			toggle: ModeToggle::default(),
		}
	}
}

impl TanzUser {
	pub fn reset_input(&mut self) {
		self.recognizer.reset();
		self.toggle.reset_for_mode(self.mode);
	}

	/// Hold both bumpers long enough to flip modes. Clears recognizer, execution,
	/// and leftover movement wishes so the next mode starts clean.
	pub fn apply_toggle(&mut self, dt: f32, chord: bool, execution: &mut TanzExecution) -> bool {
		if !self.toggle.sample(dt, chord, self.settings.toggle_secs, self.mode) {
			return false;
		}
		self.mode = self.mode.toggle();
		self.recognizer.reset();
		self.toggle.reset_for_mode(self.mode);
		execution.clear();
		info!("tanz control mode -> {:?}", self.mode);
		true
	}

	/// Apply a router hold / gesture contract to this user's pad history.
	pub fn apply_routing(&mut self, event: RoutingEvent) {
		let owned = self.recognizer.source() == Some(event.source())
			|| self.recognizer.has_partial()
			|| self.recognizer.is_frozen()
			|| self.toggle.is_partial()
			|| self.toggle.is_frozen();
		if !owned {
			return;
		}
		self.recognizer.apply_routing(event);
		self.toggle.apply_hold(event.hold());
	}

	/// Set the mode directly (script / agent). Same cleanup as a chord toggle.
	pub fn set_mode(&mut self, mode: ControlMode, execution: &mut TanzExecution) -> bool {
		if self.mode == mode {
			return false;
		}
		self.mode = mode;
		self.recognizer.reset();
		self.toggle.reset_for_mode(self.mode);
		execution.clear();
		info!("tanz control mode -> {:?}", self.mode);
		true
	}

	/// Ordinary locomotion participation for this actor. Written once on
	/// install and again only when mode actually changes.
	pub fn ordinary_locomotion(&self) -> bool {
		!self.mode.is_tanzer()
	}
}

/// Tunable recognition and toggle thresholds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TanzUserSettings {
	pub recognizer: TanzRecognizerConfig,
	pub toggle_secs: f32,
}

impl Default for TanzUserSettings {
	fn default() -> Self {
		Self { recognizer: TanzRecognizerConfig::default(), toggle_secs: TANZ_TOGGLE_SECS }
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TanzUserSystems {
	/// Locomotion participation installed after addressed pad delivery.
	Toggle,
	/// Legacy stage marker; pad recognition now runs in [`ActorControlEvent`].
	Recognize,
	/// Committed inputs → [`SkillMapFlick`]. Independent of Tanz matching.
	PublishMap,
	/// Stack matching and executor submit.
	Match,
	/// Lifecycle, cooldowns, and movement wishes.
	Execute,
	/// Full-body clip override after ordinary locomotion selection.
	Animate,
}

/// Stamp an empty Tanz runtime on a live character.
///
/// Equipped order comes from that character's inventory projection, not a
/// hidden starter grant.
pub fn spawn_tanz_user(commands: &mut Commands, user: Entity) {
	commands.entity(user).insert((
		TanzUser::default(),
		TanzStack::default(),
		TanzExecution::default(),
	));
}

pub struct TanzUserPlugin;

impl Plugin for TanzUserPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ControllersCorePlugin>() {
			app.add_plugins(ControllersCorePlugin);
		}
		ControllerPipeline::configure(app);
		MotorRequestStage::configure(app);
		app.add_message::<TanzInput>()
			.add_message::<ExecuteTanz>()
			.add_message::<RoutedExecuteTanz>()
			.add_message::<SkillMapFlick>()
			.add_message::<RoutedSkillMapFlick>()
			.add_observer(apply_tanz_routing_event)
			.add_observer(consume_tanz_control)
			.add_observer(dispatch_routed_tanz_input)
			.add_observer(execute::submit_routed_tanz)
			.configure_sets(
				Update,
				(
					TanzUserSystems::Toggle
						.in_set(ControllerPipeline::Bind)
						.after(PadDeliverySystems)
						.before(CharacterBindSystems::Produce),
					TanzUserSystems::Recognize
						.in_set(ControllerPipeline::Bind)
						.after(TanzUserSystems::Toggle)
						.before(SkillMapSystems::Steer),
					TanzUserSystems::PublishMap
						.after(TanzUserSystems::Toggle)
						.before(SkillMapSystems::Steer),
					TanzUserSystems::Match.after(TanzUserSystems::Toggle),
					TanzUserSystems::Execute
						.after(TanzUserSystems::Match)
						.in_set(MotorRequestStage::Tanz),
					TanzUserSystems::Animate
						.after(PlayerSystems::Locomotion)
						.before(CharacterMotionSystems::Anim),
				),
			)
			.add_systems(
				Update,
				(install_tanz_participation, release_tanz_participation)
					.chain()
					.in_set(TanzUserSystems::Toggle),
			)
			.add_systems(Update, publish_tanz_map_inputs.in_set(TanzUserSystems::PublishMap))
			.add_systems(Update, match_tanz_inputs.in_set(TanzUserSystems::Match))
			.add_systems(
				Update,
				(
					dispatch_routed_executions,
					submit_tanz_executions,
					advance_tanz_execution,
					apply_tanz_movement,
					interrupt_downed_tanzes,
				)
					.chain()
					.in_set(TanzUserSystems::Execute),
			)
			.add_systems(Update, drive_tanz_animation.in_set(TanzUserSystems::Animate))
			.init_resource::<TanzCallout>()
			.add_systems(Startup, (hud::spawn_tanz_mode_hud, hud::spawn_tanz_callout_hud))
			.add_systems(Update, sync_loading_bar_fill)
			.add_systems(Update, hud::sync_tanz_mode_hud.after(TanzUserSystems::Toggle))
			.add_systems(
				Update,
				(
					callout::announce_tanz_callouts.after(TanzUserSystems::Match),
					callout::tick_tanz_callout,
					hud::sync_tanz_callout_hud,
				)
					.chain(),
			);
	}
}

fn apply_tanz_routing_event(event: On<ActorRoutingEvent>, mut users: Query<&mut TanzUser>) {
	if let Ok(mut user) = users.get_mut(event.entity) {
		user.apply_routing(event.event);
	}
}

/// Actor pad delivery has already passed the source/target/epoch gate here.
/// Mode and gesture history therefore cannot advance from a retired snapshot.
fn consume_tanz_control(
	event: On<ActorControlEvent>,
	mut commands: Commands,
	mut users: Query<(&mut TanzUser, Option<&mut TanzExecution>)>,
) {
	let Ok((mut user, mut execution)) = users.get_mut(event.entity) else {
		return;
	};
	let input = &event.input;
	let sample = *input.payload();
	let entity = event.entity;
	if user.recognizer.bind_delivery(input.source(), input.binding()) {
		let mode = user.mode;
		user.toggle.reset_for_mode(mode);
	}
	if !user.toggle.is_frozen() {
		let mut fallback = TanzExecution::default();
		let execution = execution.as_deref_mut().unwrap_or(&mut fallback);
		if user.apply_toggle(sample.delta().as_secs_f32(), sample.bumper_chord(), execution) {
			commands.entity(entity).insert(OrdinaryLocomotion(user.ordinary_locomotion()));
		}
	}
	if !user.mode.is_tanzer() || user.recognizer.is_frozen() {
		return;
	}
	let config = user.settings.recognizer;
	let Some(gesture) = user.recognizer.sample(
		sample.delta().as_secs_f32(),
		sample.move_stick(),
		TanzFaceSample::from_control(sample),
		&config,
	) else {
		return;
	};
	input
		.clone()
		.map(|_| TanzInput {
			user: entity,
			direction: gesture.direction,
			magnitude: gesture.magnitude,
			coherence: gesture.coherence,
			button: gesture.button,
		})
		.trigger(&mut commands, move |input| RoutedTanzInput { entity, input });
}

/// Install participation on spawn. Mode flips write from [`consume_tanz_control`]
/// or agent `set_mode`; temporal recognizer/toggle mutation must not restamp.
fn install_tanz_participation(
	mut commands: Commands,
	added: Query<(Entity, &TanzUser), Added<TanzUser>>,
) {
	for (entity, user) in &added {
		commands.entity(entity).insert(OrdinaryLocomotion(user.ordinary_locomotion()));
	}
}

fn release_tanz_participation(mut commands: Commands, mut removed: RemovedComponents<TanzUser>) {
	for entity in removed.read() {
		if let Ok(mut entity) = commands.get_entity(entity) {
			entity.remove::<OrdinaryLocomotion>();
		}
	}
}

/// One authorized gesture fans out to map steering and Tanz matching. Keeping
/// both consumers here prevents a route change between recognition and use.
fn dispatch_routed_tanz_input(
	event: On<RoutedTanzInput>,
	users: Query<(&TanzUser, Option<&SkillMapUser>)>,
	matchers: Query<(&TanzStack, &TanzExecution)>,
	mut flicks: MessageWriter<RoutedSkillMapFlick>,
	mut execute: MessageWriter<RoutedExecuteTanz>,
) {
	let Ok((user, maps)) = users.get(event.entity) else {
		return;
	};
	if !user.mode.is_tanzer() {
		return;
	}
	let input = *event.input.payload();
	let throw = input.stick();
	if throw != Vec2::ZERO {
		let flick =
			SkillMapFlick::new(ActorTarget::new(event.entity), maps.map(|maps| maps.maps), throw);
		flicks.write(RoutedSkillMapFlick(event.input.clone().map(|_| flick)));
	}
	if let Ok((stack, execution)) = matchers.get(event.entity) {
		if let Some(id) = stack.match_input(&input, |id| execution.cooling(id)) {
			execute.write(RoutedExecuteTanz(
				event.input.clone().map(|_| ExecuteTanz::new(event.entity, id)),
			));
		}
	}
}

fn publish_tanz_map_inputs(
	users: Query<(&TanzUser, Option<&SkillMapUser>)>,
	mut inputs: MessageReader<TanzInput>,
	mut flicks: MessageWriter<SkillMapFlick>,
) {
	for input in inputs.read() {
		let Ok((user, maps)) = users.get(input.user) else {
			continue;
		};
		if !user.mode.is_tanzer() {
			continue;
		}
		let throw = input.direction * input.magnitude;
		if throw == Vec2::ZERO {
			continue;
		}
		flicks.write(SkillMapFlick::new(
			ActorTarget::new(input.user),
			maps.map(|maps| maps.maps),
			throw,
		));
	}
}

fn interrupt_downed_tanzes(mut users: Query<(&mut TanzUser, &mut TanzExecution), Added<Downed>>) {
	for (mut user, mut execution) in &mut users {
		user.recognizer.reset();
		execution.clear();
	}
}

/// Camera-relative XZ wish from a stick-space vector.
pub fn look_wish(yaw: f32, stick: Vec2) -> Vec3 {
	PlayerLook::wish_from_yaw(yaw, stick)
}

pub fn look_yaw(look: Option<&PlayerLook>) -> f32 {
	look.map(|look| look.yaw).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_character_controller::ActorControlSample;
	use maybraid_controllers_core::{InputRouter, InputSource, RoutingEvent};
	use maybraid_skill_map::SkillMapFlick;

	#[derive(Resource, Default)]
	struct RoutedFlickHits(Vec<SkillMapFlick>);

	fn record_routed_flick(
		event: On<maybraid_skill_map::RoutedSkillMapFlickEvent>,
		mut hits: ResMut<RoutedFlickHits>,
	) {
		hits.0.push(*event.input.payload());
	}

	fn set_sample(app: &mut App, source: InputSource, entity: Entity, sample: ActorControlSample) {
		let routed = app
			.world()
			.resource::<InputRouter>()
			.route_actor(source, sample.with_delta(std::time::Duration::from_secs_f32(1.0 / 60.0)));
		assert!(routed
			.as_ref()
			.is_some_and(|input| input.target().actor_entity() == Some(entity)));
		if let Some(routed) = routed {
			let mut control = RoutedActorControl::default();
			control.set(routed);
			app.world_mut().entity_mut(entity).insert(control);
		}
	}

	fn set_local_sample(app: &mut App, entity: Entity, sample: ActorControlSample) {
		set_sample(app, InputSource::local_pad(), entity, sample);
	}

	fn apply_test_routing_events(
		mut events: MessageReader<RoutingEvent>,
		mut users: Query<(Entity, &mut TanzUser, Option<&mut RoutedActorControl>)>,
	) {
		for event in events.read().copied() {
			for (entity, mut user, control) in &mut users {
				if event.applies_to(entity) {
					user.apply_routing(event);
					if event.lost_actor().is_some_and(|lost| lost.matches(entity)) {
						if let Some(mut control) = control {
							control.clear();
						}
					}
				}
			}
		}
	}

	fn emit_test_control_events(
		mut commands: Commands,
		controls: Query<(Entity, &RoutedActorControl)>,
	) {
		for (entity, control) in &controls {
			if let Some(input) =
				control.get().filter(|input| input.target().actor_entity() == Some(entity))
			{
				input
					.clone()
					.trigger(&mut commands, |input| ActorControlEvent { entity, input });
			}
		}
	}

	#[test]
	fn script_ids_roundtrip() {
		assert_eq!(ControlMode::from_script_id("tanzer"), Some(ControlMode::Tanzer));
		assert_eq!(ControlMode::from_script_id("STANDARD"), Some(ControlMode::Standard));
		assert_eq!(ControlMode::Tanzer.script_id(), "tanzer");
		assert!(ControlMode::from_script_id("toggle").is_none());
	}

	#[test]
	fn set_mode_is_idempotent_and_clears() -> anyhow::Result<()> {
		let mut user = TanzUser { mode: ControlMode::Standard, ..TanzUser::default() };
		let mut execution = TanzExecution::default();
		execution.begin(TanzId::ForwardDash, Vec2::Y, 0.5);
		assert!(!user.set_mode(ControlMode::Standard, &mut execution));
		assert!(user.set_mode(ControlMode::Tanzer, &mut execution));
		assert_eq!(user.mode, ControlMode::Tanzer);
		assert!(execution.active.is_none());
		Ok(())
	}

	#[test]
	fn bumper_chord_toggles_once_per_hold() -> anyhow::Result<()> {
		let dt = 1.0 / 60.0;
		let mut user = TanzUser::default();
		let mut execution = TanzExecution::default();
		for _ in 0..59 {
			assert!(!user.apply_toggle(dt, true, &mut execution));
		}
		assert_eq!(user.mode, ControlMode::Standard);
		assert!(user.apply_toggle(dt, true, &mut execution));
		assert_eq!(user.mode, ControlMode::Tanzer);
		assert!(!user.apply_toggle(dt, true, &mut execution));
		assert!(!user.apply_toggle(dt, false, &mut execution));
		for _ in 0..59 {
			assert!(!user.apply_toggle(dt, true, &mut execution));
		}
		assert!(user.apply_toggle(dt, true, &mut execution));
		assert_eq!(user.mode, ControlMode::Standard);
		Ok(())
	}

	#[test]
	fn mode_change_clears_execution_and_wishes() -> anyhow::Result<()> {
		let mut user = TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() };
		user.toggle.reset_for_mode(ControlMode::Tanzer);
		let mut execution = TanzExecution::default();
		execution.begin(TanzId::LeftStrafe, Vec2::NEG_X, 0.0);
		let dt = 1.0 / 60.0;
		for _ in 0..59 {
			assert!(!user.apply_toggle(dt, true, &mut execution));
		}
		assert!(user.apply_toggle(dt, true, &mut execution));
		assert_eq!(user.mode, ControlMode::Standard);
		assert!(execution.active.is_none());
		Ok(())
	}

	#[test]
	fn tanzer_install_writes_per_actor_ordinary_locomotion() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, install_tanz_participation);
		let npc = app
			.world_mut()
			.spawn(TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() })
			.id();
		app.update();
		assert_eq!(app.world().get::<OrdinaryLocomotion>(npc).map(|flag| flag.0), Some(false));
		Ok(())
	}

	#[test]
	fn npc_tanzer_does_not_stamp_a_standard_player() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, install_tanz_participation);
		let player = app
			.world_mut()
			.spawn((Player, TanzUser { mode: ControlMode::Standard, ..TanzUser::default() }))
			.id();
		let npc = app
			.world_mut()
			.spawn(TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() })
			.id();
		app.update();
		assert_eq!(app.world().get::<OrdinaryLocomotion>(player).map(|flag| flag.0), Some(true));
		assert_eq!(app.world().get::<OrdinaryLocomotion>(npc).map(|flag| flag.0), Some(false));
		Ok(())
	}

	#[test]
	fn unmatched_directional_input_still_advances_the_map() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<TanzInput>()
			.add_message::<SkillMapFlick>()
			.add_systems(Update, publish_tanz_map_inputs);
		let user = app
			.world_mut()
			.spawn(TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() })
			.id();
		app.world_mut().write_message(TanzInput {
			user,
			direction: Vec2::NEG_X,
			magnitude: 0.9,
			coherence: 1.0,
			button: TanzButton::None,
		});
		app.update();
		let flicks: Vec<_> =
			app.world_mut().resource_mut::<Messages<SkillMapFlick>>().drain().collect();
		assert_eq!(flicks.len(), 1);
		assert_eq!(flicks[0].throw(), Vec2::NEG_X * 0.9);
		assert!(flicks[0].actor().matches(user));
		Ok(())
	}

	#[test]
	fn neutral_button_only_input_does_not_invent_a_map_direction() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<TanzInput>()
			.add_message::<SkillMapFlick>()
			.add_systems(Update, publish_tanz_map_inputs);
		let user = app
			.world_mut()
			.spawn(TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() })
			.id();
		app.world_mut().write_message(TanzInput {
			user,
			direction: Vec2::ZERO,
			magnitude: 0.0,
			coherence: 0.0,
			button: TanzButton::APress,
		});
		app.update();
		let flicks: Vec<SkillMapFlick> =
			app.world_mut().resource_mut::<Messages<SkillMapFlick>>().drain().collect();
		assert!(flicks.is_empty());
		Ok(())
	}

	#[test]
	fn empty_stack_and_cooldown_still_publish_the_actors_map() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<TanzInput>()
			.add_message::<ExecuteTanz>()
			.add_message::<RoutedExecuteTanz>()
			.add_message::<SkillMapFlick>()
			.add_message::<RoutedSkillMapFlick>()
			.add_systems(
				Update,
				(publish_tanz_map_inputs, match_tanz_inputs, submit_tanz_executions).chain(),
			);
		let empty = app
			.world_mut()
			.spawn((
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				TanzStack::default(),
				TanzExecution::default(),
			))
			.id();
		let mut cooling = TanzExecution::default();
		cooling.begin(TanzId::LeftStrafe, Vec2::NEG_X, 1.0);
		let blocked = app
			.world_mut()
			.spawn((
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				TanzStack::starter(),
				cooling,
			))
			.id();
		app.world_mut().write_message(TanzInput {
			user: empty,
			direction: Vec2::NEG_X,
			magnitude: 0.9,
			coherence: 1.0,
			button: TanzButton::None,
		});
		app.world_mut().write_message(TanzInput {
			user: blocked,
			direction: Vec2::NEG_X,
			magnitude: 0.9,
			coherence: 1.0,
			button: TanzButton::None,
		});
		app.update();
		assert!(app.world().get::<TanzExecution>(empty).is_some_and(|ex| ex.active.is_none()));
		assert_eq!(
			app.world().get::<TanzExecution>(blocked).and_then(|ex| ex.active.map(|a| a.id)),
			Some(TanzId::LeftStrafe)
		);
		let flicks: Vec<_> =
			app.world_mut().resource_mut::<Messages<SkillMapFlick>>().drain().collect();
		assert_eq!(flicks.len(), 2);
		assert!(flicks.iter().any(|flick| flick.actor().matches(empty)));
		assert!(flicks.iter().any(|flick| flick.actor().matches(blocked)));
		Ok(())
	}

	fn reset_motor_intents(mut intents: Query<&mut player::MotorIntent>) {
		for mut intent in &mut intents {
			intent.clear();
		}
	}

	fn pipeline_app() -> App {
		use bevy::time::TimeUpdateStrategy;
		use player::{apply_motor, compose_motor};

		let mut app = App::new();
		ControllerPipeline::configure(&mut app);
		app.add_plugins(MinimalPlugins)
			.insert_resource(TimeUpdateStrategy::ManualDuration(
				std::time::Duration::from_secs_f32(1.0 / 60.0),
			))
			.init_resource::<InputRouter>()
			.add_message::<RoutingEvent>()
			.init_resource::<ButtonInput<MouseButton>>()
			.add_message::<TanzInput>()
			.add_message::<ExecuteTanz>()
			.add_message::<SkillMapFlick>()
			.add_message::<RoutedExecuteTanz>()
			.add_message::<RoutedSkillMapFlick>()
			.add_observer(consume_tanz_control)
			.add_observer(dispatch_routed_tanz_input)
			.add_observer(execute::submit_routed_tanz)
			.add_systems(
				Update,
				(
					reset_motor_intents,
					apply_test_routing_events,
					emit_test_control_events,
					install_tanz_participation,
					release_tanz_participation,
					publish_tanz_map_inputs,
					match_tanz_inputs,
					maybraid_locomotion_controller::interpret_standard_locomotion,
					dispatch_routed_executions,
					submit_tanz_executions,
					advance_tanz_execution,
					apply_tanz_movement,
					compose_motor,
					apply_motor,
				)
					.chain(),
			);
		app
	}

	#[test]
	fn unrelated_tanz_actor_is_unaffected() -> anyhow::Result<()> {
		use maybraid_character_controller::ActorControlSample;

		let mut app = pipeline_app();
		let local = app
			.world_mut()
			.spawn((
				Player,
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				TanzStack::starter(),
				TanzExecution::default(),
				player::CharacterController,
				player::MotorIntent::default(),
				player::MotorSelection::default(),
				player::ResolvedMotorRequest::default(),
				player::MoveWish::default(),
				PlayerLook::default(),
			))
			.id();
		let other = app
			.world_mut()
			.spawn((
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				TanzStack::starter(),
				TanzExecution::default(),
				player::MotorIntent::default(),
				player::MotorSelection::default(),
				player::ResolvedMotorRequest::default(),
			))
			.id();
		assign_pad(&mut app, local);
		set_local_sample(
			&mut app,
			local,
			ActorControlSample::default().with_move_stick(Vec2::NEG_X),
		);
		for _ in 0..20 {
			app.update();
		}
		assert_eq!(
			app.world()
				.get::<TanzExecution>(local)
				.and_then(|ex| ex.active.map(|active| active.id)),
			Some(TanzId::LeftStrafe),
			"routed stick must select only the addressed tanzer"
		);
		assert!(
			app.world().get::<TanzExecution>(other).is_some_and(|ex| ex.active.is_none()),
			"unrelated tanzer must not consume the local pad"
		);
		Ok(())
	}

	#[test]
	fn two_sources_commit_only_their_own_tanz() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		use maybraid_controllers_core::InputRouting;

		let mut app = pipeline_app();
		let left = spawn_tanzer(&mut app, ControlMode::Tanzer);
		let right = spawn_tanzer(&mut app, ControlMode::Tanzer);
		let second_pad = app.world_mut().spawn_empty().id();
		let second_source = InputSource::virtual_pad(second_pad);
		app.world_mut().run_system_once(move |mut routing: InputRouting| {
			routing.assign_actor(InputSource::local_pad(), ActorTarget::new(left));
			routing.assign_actor(second_source, ActorTarget::new(right));
		})?;
		set_local_sample(
			&mut app,
			left,
			ActorControlSample::default().with_move_stick(Vec2::NEG_X),
		);
		set_sample(
			&mut app,
			second_source,
			right,
			ActorControlSample::default().with_move_stick(Vec2::X),
		);
		for _ in 0..20 {
			app.update();
		}
		assert_eq!(
			app.world()
				.get::<TanzExecution>(left)
				.and_then(|ex| ex.active.map(|active| active.id)),
			Some(TanzId::LeftStrafe)
		);
		assert_eq!(
			app.world()
				.get::<TanzExecution>(right)
				.and_then(|ex| ex.active.map(|active| active.id)),
			Some(TanzId::RightStrafe)
		);
		Ok(())
	}

	#[test]
	fn unmatched_gesture_moves_only_its_own_map() -> anyhow::Result<()> {
		use maybraid_skill_map::{
			apply_flicks, spawn_skill_maps, SkillMapId, SkillMapMember, SkillMapViewportCamera,
		};

		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<maybraid_skill_map::SkillMapEnabled>()
			.add_message::<TanzInput>()
			.add_message::<SkillMapFlick>()
			.add_systems(Update, (publish_tanz_map_inputs, apply_flicks).chain());
		let user_a = app
			.world_mut()
			.spawn((
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				TanzStack::default(),
				TanzExecution::default(),
			))
			.id();
		let user_b = app
			.world_mut()
			.spawn((
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				TanzStack::default(),
				TanzExecution::default(),
			))
			.id();
		let maps_a = spawn_skill_maps(&mut app.world_mut().commands(), user_a);
		let maps_b = spawn_skill_maps(&mut app.world_mut().commands(), user_b);
		app.world_mut().flush();
		let cam_a = app
			.world_mut()
			.spawn((
				SkillMapId(0),
				SkillMapMember { user: user_a, session: maps_a },
				SkillMapViewportCamera,
				Transform::default(),
				Projection::Orthographic(OrthographicProjection::default_2d()),
			))
			.id();
		let cam_b = app
			.world_mut()
			.spawn((
				SkillMapId(1),
				SkillMapMember { user: user_b, session: maps_b },
				SkillMapViewportCamera,
				Transform::default(),
				Projection::Orthographic(OrthographicProjection::default_2d()),
			))
			.id();
		app.world_mut().write_message(TanzInput {
			user: user_a,
			direction: Vec2::X,
			magnitude: 1.0,
			coherence: 1.0,
			button: TanzButton::None,
		});
		app.update();
		assert!(app.world().get::<TanzExecution>(user_a).is_some_and(|ex| ex.active.is_none()));
		let a = app.world().get::<Transform>(cam_a).expect("cam a").translation.xy();
		let b = app.world().get::<Transform>(cam_b).expect("cam b").translation.xy();
		assert!(a.x > 1.0, "unmatched Tanz still moves its map, got {a}");
		assert_eq!(b, Vec2::ZERO, "the other actor's map must stay put");
		Ok(())
	}

	#[test]
	fn switching_standard_and_tanzer_leaves_no_stale_motion() -> anyhow::Result<()> {
		use maybraid_character_controller::{
			ActorControlSample, CharacterIntent, CharacterIntentFrame,
		};
		use player::{MoveSpeedCap, MoveWish, Sprinting};

		let mut app = pipeline_app();
		let player = app
			.world_mut()
			.spawn((
				Player,
				player::CharacterController,
				TanzUser { mode: ControlMode::Standard, ..TanzUser::default() },
				TanzStack::starter(),
				TanzExecution::default(),
				player::MotorIntent::default(),
				player::MotorSelection::default(),
				player::ResolvedMotorRequest::default(),
				MoveWish::default(),
				PlayerLook::default(),
				Sprinting,
			))
			.id();
		assign_pad(&mut app, player);
		app.update();
		let intents = app
			.world()
			.resource::<InputRouter>()
			.route_actor(
				InputSource::local_pad(),
				vec![CharacterIntent::Move(Vec2::Y), CharacterIntent::StartSprint],
			)
			.ok_or_else(|| anyhow::anyhow!("missing local actor route"))?;
		app.world_mut().entity_mut(player).insert(CharacterIntentFrame::new(intents));
		app.update();
		app.world_mut().entity_mut(player).remove::<CharacterIntentFrame>();
		assert_eq!(app.world().get::<MoveWish>(player).map(|wish| wish.0), Some(-Vec3::Z));
		assert!(app.world().get::<Sprinting>(player).is_some());

		set_local_sample(&mut app, player, ActorControlSample::default().with_bumpers(true, true));
		for _ in 0..70 {
			app.update();
		}
		assert_eq!(
			app.world().get::<TanzUser>(player).map(|user| user.mode),
			Some(ControlMode::Tanzer)
		);
		assert!(app.world().get::<TanzExecution>(player).is_some_and(|ex| ex.active.is_none()));
		assert_eq!(app.world().get::<MoveWish>(player).map(|wish| wish.0), Some(Vec3::ZERO));
		assert!(app.world().get::<Sprinting>(player).is_none());
		assert!(app.world().get::<MoveSpeedCap>(player).is_none());
		let intent = app
			.world()
			.get::<player::MotorIntent>(player)
			.ok_or_else(|| anyhow::anyhow!("motor intent"))?;
		assert_eq!(
			intent.selected_base().map(|(kind, _)| kind),
			Some(player::MotorContributor::Tanz)
		);
		let resolved = intent.compose(None);
		assert_eq!(resolved.translation, Vec3::ZERO);
		assert_eq!(resolved.sprint, player::SprintRequest::Release);
		assert_eq!(resolved.speed_cap, player::SpeedCapRequest::Release);

		set_local_sample(&mut app, player, ActorControlSample::default().with_bumpers(true, true));
		for _ in 0..70 {
			app.update();
		}
		assert_eq!(
			app.world().get::<TanzUser>(player).map(|user| user.mode),
			Some(ControlMode::Standard)
		);
		assert_eq!(app.world().get::<MoveWish>(player).map(|wish| wish.0), Some(Vec3::ZERO));
		assert!(app.world().get::<MoveSpeedCap>(player).is_none());
		assert!(app.world().get::<Sprinting>(player).is_none());
		Ok(())
	}

	fn spawn_tanzer(app: &mut App, mode: ControlMode) -> Entity {
		app.world_mut()
			.spawn((
				Player,
				player::CharacterController,
				TanzUser { mode, ..TanzUser::default() },
				TanzStack::starter(),
				TanzExecution::default(),
				player::MotorIntent::default(),
				player::MotorSelection::default(),
				player::ResolvedMotorRequest::default(),
				player::MoveWish::default(),
				PlayerLook::default(),
			))
			.id()
	}

	fn assign_pad(app: &mut App, entity: Entity) {
		use bevy::ecs::system::RunSystemOnce;
		use maybraid_controllers_core::{InputRouting, InputSource};

		let actor = ActorTarget::new(entity);
		app.world_mut()
			.run_system_once(move |mut routing: InputRouting| {
				routing.connect(InputSource::local_pad());
				routing.assign_actor(InputSource::local_pad(), actor);
			})
			.expect("assign pad");
	}

	#[test]
	fn reassignment_cancels_a_partial_tanz_gesture() -> anyhow::Result<()> {
		use maybraid_character_controller::ActorControlSample;
		use maybraid_controllers_core::{InputRouter, InputSource};

		let mut app = pipeline_app();
		let first = spawn_tanzer(&mut app, ControlMode::Tanzer);
		let second = spawn_tanzer(&mut app, ControlMode::Tanzer);
		assign_pad(&mut app, first);
		set_local_sample(
			&mut app,
			first,
			ActorControlSample::default().with_move_stick(Vec2::NEG_X),
		);
		for _ in 0..6 {
			app.update();
		}
		assert!(
			app.world()
				.get::<TanzUser>(first)
				.is_some_and(|user| user.recognizer.has_partial()),
			"six frames is a partial LeftStrafe, not a commit"
		);
		assert!(app.world().get::<TanzExecution>(first).is_some_and(|ex| ex.active.is_none()));

		assign_pad(&mut app, second);
		set_local_sample(
			&mut app,
			second,
			ActorControlSample::default().with_move_stick(Vec2::NEG_X),
		);
		app.update();
		assert!(
			app.world()
				.get::<TanzUser>(first)
				.is_some_and(|user| !user.recognizer.has_partial()),
			"reassignment must cancel the previous owner's history"
		);
		assert!(app.world().get::<TanzExecution>(first).is_some_and(|ex| ex.active.is_none()));
		assert!(app.world().get::<TanzExecution>(second).is_some_and(|ex| ex.active.is_none()));

		assign_pad(&mut app, first);
		set_local_sample(
			&mut app,
			first,
			ActorControlSample::default().with_move_stick(Vec2::NEG_X),
		);
		for _ in 0..2 {
			app.update();
		}
		assert!(
			app.world().get::<TanzExecution>(first).is_some_and(|ex| ex.active.is_none()),
			"handoff must not complete the cancelled gesture"
		);

		for _ in 0..12 {
			app.update();
		}
		assert_eq!(
			app.world().get::<TanzExecution>(first).and_then(|ex| ex.active.map(|a| a.id)),
			Some(TanzId::LeftStrafe)
		);
		assert!(app.world().get::<TanzExecution>(second).is_some_and(|ex| ex.active.is_none()));
		assert!(app.world().resource::<InputRouter>().actor(InputSource::local_pad()).is_some());
		Ok(())
	}

	#[test]
	fn disconnect_releases_a_bumper_hold() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		use maybraid_character_controller::ActorControlSample;
		use maybraid_controllers_core::{InputRouting, InputSource};

		let mut app = pipeline_app();
		let player = spawn_tanzer(&mut app, ControlMode::Standard);
		assign_pad(&mut app, player);
		set_local_sample(&mut app, player, ActorControlSample::default().with_bumpers(true, true));
		for _ in 0..30 {
			app.update();
		}
		assert_eq!(
			app.world().get::<TanzUser>(player).map(|user| user.mode),
			Some(ControlMode::Standard)
		);
		assert!(app.world().get::<TanzUser>(player).is_some_and(|user| user.toggle.is_partial()));

		app.world_mut()
			.run_system_once(|mut routing: InputRouting| {
				routing.disconnect(InputSource::local_pad());
			})
			.expect("disconnect");
		app.world_mut().entity_mut(player).insert(RoutedActorControl::default());
		app.update();
		assert!(
			app.world()
				.get::<TanzUser>(player)
				.is_some_and(|user| !user.toggle.is_partial()),
			"disconnect must release the bumper hold"
		);
		assert_eq!(
			app.world().get::<TanzUser>(player).map(|user| user.mode),
			Some(ControlMode::Standard)
		);

		assign_pad(&mut app, player);
		set_local_sample(&mut app, player, ActorControlSample::default().with_bumpers(true, true));
		for _ in 0..30 {
			app.update();
		}
		assert_eq!(
			app.world().get::<TanzUser>(player).map(|user| user.mode),
			Some(ControlMode::Standard),
			"a leftover half-hold must not complete after reconnect"
		);
		Ok(())
	}

	#[test]
	fn changing_map_session_does_not_complete_a_standard_flick() -> anyhow::Result<()> {
		use bevy::time::TimeUpdateStrategy;
		use maybraid_character_controller::ActorControlSample;
		use maybraid_controllers_core::InputRouter;
		use maybraid_skill_map::{
			detect_skill_map_flicks, dispatch_routed_flicks, RoutedSkillMapFlick,
			SkillMapController, SkillMapEnabled, SkillMapFlick, SkillMapUser,
		};

		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(TimeUpdateStrategy::ManualDuration(
				std::time::Duration::from_secs_f32(0.04),
			))
			.init_resource::<InputRouter>()
			.add_message::<RoutingEvent>()
			.init_resource::<SkillMapEnabled>()
			.init_resource::<RoutedFlickHits>()
			.add_message::<SkillMapFlick>()
			.add_message::<RoutedSkillMapFlick>()
			.add_observer(record_routed_flick)
			.add_systems(Update, (detect_skill_map_flicks, dispatch_routed_flicks).chain());
		let session_a = app.world_mut().spawn_empty().id();
		let session_b = app.world_mut().spawn_empty().id();
		let user = app
			.world_mut()
			.spawn((SkillMapUser::mapping(session_a), OrdinaryLocomotion(true)))
			.id();
		assign_pad(&mut app, user);
		let sample = |stick: Vec2| {
			ActorControlSample::default().with_bumpers(false, true).with_move_stick(stick)
		};
		for stick in [Vec2::ZERO, Vec2::X * 0.75, Vec2::X] {
			set_local_sample(&mut app, user, sample(stick));
			app.update();
		}
		assert!(app
			.world()
			.get::<SkillMapController>(user)
			.is_some_and(SkillMapController::has_partial));
		app.world_mut().entity_mut(user).insert(SkillMapUser::mapping(session_b));
		for stick in [Vec2::X * 0.4, Vec2::ZERO] {
			set_local_sample(&mut app, user, sample(stick));
			app.update();
		}
		let flicks = &app.world().resource::<RoutedFlickHits>().0;
		assert!(
			flicks.iter().all(|flick| flick.session() != Some(session_b)),
			"a session change must not complete a flick against the new session: {flicks:?}"
		);
		assert!(flicks.is_empty(), "the in-flight flick should be dropped, got {flicks:?}");
		Ok(())
	}

	#[test]
	fn removing_tanz_user_releases_ordinary_locomotion() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_systems(Update, (install_tanz_participation, release_tanz_participation));
		let user = app.world_mut().spawn(TanzUser::default()).id();
		app.update();
		assert!(app.world().get::<OrdinaryLocomotion>(user).is_some());
		app.world_mut().entity_mut(user).remove::<TanzUser>();
		app.update();
		assert!(app.world().get::<OrdinaryLocomotion>(user).is_none());
		Ok(())
	}

	#[derive(Resource, Default)]
	struct NavHits(Vec<menu_components::MenuNav>);

	fn record_menu_nav(impulse: On<menu_components::MenuNavImpulse>, mut hits: ResMut<NavHits>) {
		hits.0.push(impulse.event().nav);
	}

	fn lifecycle_plugins() -> App {
		use bevy::time::TimeUpdateStrategy;
		use maybraid_character_controller::CharacterControllerPlugin;
		use maybraid_menu_controller::MenuControllerPlugin;
		use maybraid_session::SessionPlugin;

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			bevy::input::InputPlugin,
			bevy::asset::AssetPlugin::default(),
			CharacterControllerPlugin,
			TanzUserPlugin,
			SessionPlugin,
			MenuControllerPlugin,
		))
		.insert_resource(TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f32(
			1.0 / 60.0,
		)))
		.add_message::<bevy::input::keyboard::KeyboardInput>()
		.add_message::<bevy::window::Ime>()
		.add_message::<bevy::input::mouse::MouseMotion>()
		.add_message::<bevy::input::gamepad::GamepadRumbleRequest>()
		.add_message::<maybraid_skill_map::SkillMapFlick>()
		.init_resource::<ButtonInput<KeyCode>>()
		.init_resource::<ButtonInput<MouseButton>>()
		.init_resource::<bevy::picking::hover::HoverMap>();
		app.finish();
		app.cleanup();
		app
	}

	fn step_lifecycle(app: &mut App) {
		app.world_mut().run_schedule(bevy::app::First);
		app.world_mut().run_schedule(Update);
	}

	fn assign_lifecycle(app: &mut App, entity: Entity) {
		use bevy::ecs::system::RunSystemOnce;
		use maybraid_controllers_core::{InputRouting, InputSource};

		let actor = ActorTarget::new(entity);
		app.world_mut()
			.run_system_once(move |mut routing: InputRouting| {
				routing.assign_actor(InputSource::local_pad(), actor);
			})
			.expect("assign lifecycle actor");
	}

	#[test]
	fn plugin_menu_nav_while_in_tanz_mode_does_not_commit_gestures() -> anyhow::Result<()> {
		use maybraid_character_controller::RoutedActorControl;
		use maybraid_menu_controller::MenuController;
		use maybraid_virtual_pad_controller::{PrimaryVirtualPad, VirtualPad, VirtualPadEvent};
		use menu_components::{HudMenu, HudOverlayMenu};

		let mut app = lifecycle_plugins();
		app.init_resource::<NavHits>().add_observer(record_menu_nav);
		let player = app
			.world_mut()
			.spawn((
				Player,
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				TanzStack::starter(),
				TanzExecution::default(),
			))
			.id();
		assign_lifecycle(&mut app, player);
		let catalog = app.world_mut().spawn((HudMenu::new(4), HudOverlayMenu)).id();
		let root = app.world_mut().spawn(MenuController::default()).id();
		app.world_mut().entity_mut(root).add_child(catalog);

		let source = app.world().resource::<PrimaryVirtualPad>().0;
		for sequence in 1..=22 {
			let mut pad = VirtualPad::default();
			pad.begin_frame();
			if sequence > 2 {
				pad.add_move(Vec2::NEG_X);
			}
			pad.finish_digital();
			app.world_mut().write_message(VirtualPadEvent {
				source,
				sequence,
				delta: std::time::Duration::from_secs_f32(1.0 / 60.0),
				pad,
			});
			step_lifecycle(&mut app);
		}
		assert!(
			app.world()
				.get::<RoutedActorControl>(player)
				.is_none_or(|control| control.actor().is_none()),
			"menu capture must revoke actor delivery"
		);
		assert!(
			app.world().get::<TanzExecution>(player).is_some_and(|ex| ex.active.is_none()),
			"captured tanzer must not commit a gesture"
		);
		assert!(
			!app.world().resource::<NavHits>().0.is_empty(),
			"menu must still receive nav while the actor is in Tanz mode"
		);
		Ok(())
	}

	#[test]
	fn plugin_pause_during_a_healing_tanz_freezes_the_local_actor() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		use damage::Health;
		use maybraid_controllers_core::{InputRouter, InputRouting, InputSource};
		use maybraid_session::SessionCapabilities;

		let mut app = lifecycle_plugins();
		app.world_mut().resource_mut::<SessionCapabilities>().install_play(true);
		let mut execution = TanzExecution::default();
		execution.submit(TanzId::HealingSalute);
		let player = app
			.world_mut()
			.spawn((
				Player,
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				TanzStack::starter(),
				execution,
				Health { current: 40.0, max: 100.0 },
			))
			.id();
		assign_lifecycle(&mut app, player);
		app.world_mut()
			.run_system_once(|mut routing: InputRouting| {
				routing.set_paused(true);
			})
			.expect("pause");
		assert!(app.world().resource::<InputRouter>().is_suspended(InputSource::local_pad()));
		let mut npc_execution = TanzExecution::default();
		npc_execution.submit(TanzId::HealingSalute);
		let npc = app
			.world_mut()
			.spawn((
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				TanzStack::starter(),
				npc_execution,
				Health { current: 40.0, max: 100.0 },
			))
			.id();
		for _ in 0..20 {
			step_lifecycle(&mut app);
		}
		let player_health = app.world().get::<Health>(player).expect("player health").current;
		assert!(
			(player_health - 40.0).abs() < 1e-4,
			"paused local healing Tanz must not apply, got {player_health}"
		);
		assert!(
			app.world()
				.get::<TanzExecution>(player)
				.is_some_and(|ex| { ex.active.is_some_and(|active| active.age == 0.0) }),
			"paused execution timers must freeze"
		);
		let npc_health = app.world().get::<Health>(npc).expect("npc health").current;
		assert!(
			(npc_health - 65.0).abs() < 1e-3,
			"session pause must not freeze NPC Tanz execution, got {npc_health}"
		);
		Ok(())
	}

	#[test]
	fn plugin_ordinary_sampling_does_not_restamp_without_a_transition() -> anyhow::Result<()> {
		use maybraid_character_controller::OrdinaryLocomotion;
		use maybraid_virtual_pad_controller::{
			PadButton, PrimaryVirtualPad, VirtualPad, VirtualPadEvent,
		};

		let mut app = lifecycle_plugins();
		let player = app
			.world_mut()
			.spawn((Player, TanzUser::default(), TanzStack::default(), TanzExecution::default()))
			.id();
		assign_lifecycle(&mut app, player);
		step_lifecycle(&mut app);
		assert_eq!(app.world().get::<OrdinaryLocomotion>(player).map(|flag| flag.0), Some(true));
		app.world_mut().entity_mut(player).insert(OrdinaryLocomotion(false));
		let source = app.world().resource::<PrimaryVirtualPad>().0;
		for sequence in 1..=12 {
			let mut pad = VirtualPad::default();
			pad.begin_frame();
			pad.add_move(Vec2::Y);
			pad.hold_digital(PadButton::A);
			pad.finish_digital();
			app.world_mut().write_message(VirtualPadEvent {
				source,
				sequence,
				delta: std::time::Duration::from_secs_f32(1.0 / 60.0),
				pad,
			});
			step_lifecycle(&mut app);
		}
		assert_eq!(
			app.world().get::<TanzUser>(player).map(|user| user.mode),
			Some(ControlMode::Standard)
		);
		assert_eq!(
			app.world().get::<OrdinaryLocomotion>(player).map(|flag| flag.0),
			Some(false),
			"toggle + recognition must not restamp OrdinaryLocomotion without a mode flip"
		);
		Ok(())
	}
}
