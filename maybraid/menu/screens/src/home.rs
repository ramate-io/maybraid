//! Maybraid home screen: bottom-left title plus destination labels.

use bevy::prelude::*;
use game_commands::command::TextEntryFocus;
use menu_components::{
	activate_clicked_text_menu_items, activate_selected_text_menu_items, spawn_text_menu_header,
	spawn_text_menu_item, text_menu_column_node, MenuComponentsPlugin, MenuFonts, TextMenu,
	TextMenuInputLock, TextMenuSystems,
};

/// Queue a home-screen spawn (despawns any existing home UI first).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowHome;

/// Marker on the spawned home-screen root.
#[derive(Component, Debug, Clone, Copy)]
pub struct HomeScreen;

/// Home-menu destinations. Screens emit these; they do not navigate yet.
#[derive(Clone, Copy, Debug, Message, PartialEq, Eq)]
pub enum HomeMenuChoice {
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
			.add_systems(Update, sync_text_menu_input_lock.in_set(TextMenuSystems::InputLock))
			.add_systems(
				Update,
				(
					apply_show_home,
					activate_clicked_text_menu_items::<HomeMenuChoice>,
					activate_selected_text_menu_items::<HomeMenuChoice>,
				),
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
	fonts: Res<MenuFonts>,
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
	spawn_home_screen(&mut commands, &fonts);
}

fn spawn_home_screen(commands: &mut Commands, fonts: &MenuFonts) {
	commands
		.spawn((HomeScreen, TextMenu::new(HomeMenuChoice::ALL.len()), text_menu_column_node()))
		.with_children(|column| {
			spawn_text_menu_header(column, fonts, "Maybraid");
			for (index, choice) in HomeMenuChoice::ALL.iter().copied().enumerate() {
				spawn_text_menu_item(column, fonts, index, choice.label(), choice);
			}
		});
}
