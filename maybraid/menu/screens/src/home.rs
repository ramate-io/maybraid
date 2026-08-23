//! Maybraid home screen: bottom-left title plus destination labels.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use game_commands::command::TextEntryFocus;
use menu_components::{
	activate_selected_text_menu_items, emit_menu_choice, MenuComponentsPlugin, TextMenuColumn,
	TextMenuInputLock, TextMenuSystems,
};

/// Queue a home-screen spawn (despawns any existing home UI first).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowHome;

/// Marker on the spawned home-screen root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct HomeScreen;

/// Home destinations. Each pickable row stamps this as a component; [`emit_menu_choice`]
/// copies the clicked (or Enter-selected) value onto the message bus. Screens do
/// not navigate yet.
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum HomeMenuChoice {
	#[default]
	Discovery,
	Reliquary,
	Characters,
	TrainingGround,
	Settings,
}

impl HomeMenuChoice {
	pub const ALL: [Self; 5] =
		[Self::Discovery, Self::Reliquary, Self::Characters, Self::TrainingGround, Self::Settings];

	pub fn label(self) -> &'static str {
		match self {
			Self::Discovery => "Discovery",
			Self::Reliquary => "Reliquary",
			Self::Characters => "Characters",
			Self::TrainingGround => "Training Ground",
			Self::Settings => "Settings",
		}
	}
}

impl HomeScreen {
	pub fn scene() -> impl Scene + 'static {
		(
			bsn! { HomeScreen },
			TextMenuColumn::new(
				"Maybraid",
				HomeMenuChoice::ALL.into_iter().map(|choice| (choice.label(), choice)),
			)
			.scene(),
		)
	}
}

pub fn request_show_home(commands: &mut Commands) {
	commands.spawn(RequestShowHome);
}

pub struct HomeScreenPlugin;

impl Plugin for HomeScreenPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MenuComponentsPlugin>() {
			app.add_plugins(MenuComponentsPlugin);
		}
		app.add_message::<HomeMenuChoice>()
			.add_observer(emit_menu_choice::<HomeMenuChoice>)
			.add_systems(Update, sync_text_menu_input_lock.in_set(TextMenuSystems::InputLock))
			.add_systems(
				Update,
				(apply_show_home, activate_selected_text_menu_items::<HomeMenuChoice>),
			);
	}
}

fn sync_text_menu_input_lock(
	focus: Option<Res<TextEntryFocus>>,
	mut lock: ResMut<TextMenuInputLock>,
) {
	lock.0 = focus.is_some_and(|focus| focus.0);
}

fn apply_show_home(
	mut commands: Commands,
	requests: Query<Entity, With<RequestShowHome>>,
	existing: Query<Entity, With<HomeScreen>>,
) {
	if requests.is_empty() {
		return;
	}
	for entity in &existing {
		commands.entity(entity).despawn();
	}
	for entity in &requests {
		commands.entity(entity).despawn();
	}
	commands.spawn_scene(HomeScreen::scene());
}
