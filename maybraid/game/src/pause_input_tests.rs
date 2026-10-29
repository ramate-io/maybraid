use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy::scene::{Scene, ScenePlugin, WorldSceneExt};
use maybraid_character_controller::{
	ActorIntentEvent, CharacterControllerPlugin, CharacterIntent, CharacterIntentFrame,
	OrdinaryLocomotion,
};
use maybraid_controllers_core::{CaptureResolveSystems, InputRouter, InputSource, UiTarget};
use maybraid_menu_controller::{MenuController, MenuControllerPlugin, MenuStartPolicy};
use maybraid_virtual_pad_controller::{
	PadButton, PadDevices, PrimaryVirtualPad, VirtualPad, VirtualPadController, VirtualPadSystems,
};
use menu_components::TextMenu;
use menu_screens::{
	GameMode, InGameMenuChoice, InGameScreen, InGameScreenPlugin, InGameSettings,
	InGameSettingsChoice, InGameSettingsScreen, MenuScreen, RenderScaleReadout,
};

#[derive(Resource, Default)]
struct TestPadFrame {
	source: Option<Entity>,
	pad: Option<VirtualPad>,
}

#[derive(Resource, Default)]
struct SeenIntents(Vec<CharacterIntent>);

fn override_published_pad(
	primary: Res<PrimaryVirtualPad>,
	frame: Res<TestPadFrame>,
	mut controllers: Query<(Entity, &mut VirtualPadController)>,
) {
	let source = frame.source.unwrap_or(primary.0);
	let Some(pad) = frame.pad.as_ref() else {
		return;
	};
	if let Ok((_, mut controller)) = controllers.get_mut(source) {
		controller.pad = pad.clone();
	}
}

fn record_intent(event: On<ActorIntentEvent>, mut seen: ResMut<SeenIntents>) {
	seen.0.push(*event.input.payload());
}

fn paused_menu_app(scene: impl Scene + 'static) -> anyhow::Result<(App, Entity, Entity, Entity)> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		InputPlugin,
		AssetPlugin::default(),
		ScenePlugin,
		bevy::window::WindowPlugin { primary_window: None, ..default() },
	));
	app.init_asset::<Font>().init_asset::<Image>();
	app.init_resource::<bevy::picking::hover::HoverMap>();
	app.add_plugins((InGameScreenPlugin, MenuControllerPlugin, CharacterControllerPlugin));
	app.world_mut().insert_resource(State::new(crate::GameFlow::World));
	app.world_mut().insert_resource(State::new(crate::WorldPause::Menu));
	app.world_mut().insert_resource(NextState::<crate::WorldPause>::Unchanged);
	app.init_resource::<TestPadFrame>()
		.init_resource::<SeenIntents>()
		.add_observer(record_intent)
		.add_observer(crate::toggle_world_pause)
		.add_systems(
			PreUpdate,
			override_published_pad
				.after(VirtualPadSystems::Produce)
				.before(VirtualPadSystems::Derive),
		)
		.add_systems(Update, crate::configure_pause_menu.before(CaptureResolveSystems));

	let source = app.world().resource::<PrimaryVirtualPad>().0;
	let actor = app.world_mut().spawn(OrdinaryLocomotion(true)).id();
	{
		let mut router = app.world_mut().resource_mut::<InputRouter>();
		router.assign_actor(
			InputSource::virtual_pad(source),
			maybraid_controllers_core::ActorTarget::new(actor),
		);
		router.set_source_paused(InputSource::virtual_pad(source), true);
	}
	let screen = app.world_mut().spawn_scene(scene)?.id();
	app.update();
	anyhow::ensure!(
		*app.world().resource::<State<crate::WorldPause>>().get() == crate::WorldPause::Menu,
		"the fixture must begin with the world paused"
	);
	Ok((app, screen, actor, source))
}

fn pad_frame(buttons: &[PadButton], move_stick: Vec2, trigger_fire: f32) -> VirtualPad {
	let mut pad = VirtualPad::default();
	pad.begin_frame();
	pad.add_move(move_stick);
	for button in buttons {
		pad.hold_digital(*button);
	}
	pad.max_triggers(0.0, trigger_fire);
	pad.apply_trigger_digital(0.5);
	pad.finish_digital();
	pad
}

fn send_pad_frame(app: &mut App, source: Entity, pad: VirtualPad) {
	let mut frame = app.world_mut().resource_mut::<TestPadFrame>();
	frame.source = Some(source);
	frame.pad = Some(pad);
	drop(frame);
	app.update();
	let mut frame = app.world_mut().resource_mut::<TestPadFrame>();
	frame.source = None;
	frame.pad = None;
}

fn text_menu_entity(app: &mut App) -> anyhow::Result<Entity> {
	app.world_mut()
		.query_filtered::<Entity, With<TextMenu>>()
		.iter(app.world())
		.next()
		.ok_or_else(|| anyhow::anyhow!("screen did not spawn its text menu"))
}

fn drain_choices<E: Message>(app: &mut App) -> Vec<E> {
	app.world_mut().resource_mut::<Messages<E>>().drain().collect()
}

#[test]
fn paused_ingame_screen_navigates_selects_and_keeps_gameplay_suspended() -> anyhow::Result<()> {
	let scene = InGameScreen::scene(&GameMode::default(), None);
	let (mut app, screen, actor, source) = paused_menu_app(scene)?;
	let menu = text_menu_entity(&mut app)?;
	let primary = InputSource::virtual_pad(source);
	let router = app.world().resource::<InputRouter>();
	anyhow::ensure!(router.is_suspended(primary), "pause should suspend the primary source");
	anyhow::ensure!(
		app.world()
			.get::<MenuController>(screen)
			.is_some_and(|controller| controller.start == MenuStartPolicy::Shell),
		"the pause root must reserve Start for shell resume"
	);

	// D-pad navigation reaches the real five-row pause screen while the same
	// frame also carries gameplay movement and fire values that suspension drops.
	send_pad_frame(&mut app, source, pad_frame(&[PadButton::DpadDown], Vec2::X, 0.9));
	let selected = app.world().get::<TextMenu>(menu).map(|menu| menu.selected);
	assert_eq!(selected, Some(1), "D-pad Down should select Records through Core delivery");
	assert!(app.world().resource::<SeenIntents>().0.is_empty());
	assert!(app.world().get::<CharacterIntentFrame>(actor).is_none());

	send_pad_frame(&mut app, source, pad_frame(&[PadButton::A], Vec2::ZERO, 0.0));
	assert_eq!(drain_choices::<InGameMenuChoice>(&mut app), [InGameMenuChoice::Records]);
	assert!(app.world().resource::<SeenIntents>().0.is_empty());

	// A secondary source assigned to the same UI entity must not drive the
	// explicitly primary pause host.
	let secondary = app
		.world_mut()
		.spawn(VirtualPadController::new(PadDevices::keyboard_mouse()))
		.id();
	app.world_mut()
		.resource_mut::<InputRouter>()
		.assign_ui(InputSource::virtual_pad(secondary), UiTarget::new(screen));
	app.update();
	send_pad_frame(&mut app, secondary, pad_frame(&[PadButton::DpadDown], Vec2::ZERO, 0.0));
	assert_eq!(app.world().get::<TextMenu>(menu).map(|menu| menu.selected), Some(1));
	assert!(drain_choices::<InGameMenuChoice>(&mut app).is_empty());

	// Start resumes from the root and is filtered out of row selection.
	send_pad_frame(&mut app, source, pad_frame(&[PadButton::Start], Vec2::ZERO, 0.0));
	assert!(matches!(
		app.world().resource::<NextState<crate::WorldPause>>(),
		NextState::Pending(crate::WorldPause::Playing)
	));
	assert!(drain_choices::<InGameMenuChoice>(&mut app).is_empty());
	assert_eq!(
		app.world().resource::<SeenIntents>().0,
		[CharacterIntent::InGameMenu],
		"only the shell pause command may pass from the suspended actor frame"
	);
	Ok(())
}

#[test]
fn start_on_the_real_pause_settings_screen_selects_and_keeps_pause_active() -> anyhow::Result<()> {
	let scene = InGameSettingsScreen::scene(
		&GameMode::default(),
		InGameSettings::default(),
		RenderScaleReadout::default(),
		InGameSettingsChoice::Shadows,
	);
	let (mut app, _screen, _actor, source) = paused_menu_app(scene)?;
	anyhow::ensure!(
		app.world_mut()
			.query_filtered::<Entity, With<MenuScreen>>()
			.iter(app.world())
			.next()
			.is_some(),
		"settings should be an in-game submenu"
	);

	send_pad_frame(&mut app, source, pad_frame(&[PadButton::Start], Vec2::ZERO, 0.0));
	assert_eq!(drain_choices::<InGameSettingsChoice>(&mut app), [InGameSettingsChoice::Shadows]);
	let menu = text_menu_entity(&mut app)?;
	assert_eq!(app.world().get::<TextMenu>(menu).map(|menu| menu.selected), Some(0));
	assert!(matches!(app.world().resource::<NextState<crate::WorldPause>>(), NextState::Unchanged));
	assert!(app.world().resource::<SeenIntents>().0.is_empty());
	Ok(())
}
