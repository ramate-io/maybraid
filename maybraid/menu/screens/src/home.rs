//! Maybraid home screen: bottom-left title plus destination labels.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use maybraid_menu_controller::MenuController;
use menu_components::info::description::{set_description_for_menu, TextMenuDescription};
use menu_components::single_select::text_cursor::TextCursorColumn;
use menu_components::single_select::{republish_menu_activate, MenuFocus};
use menu_components::TextMenuPlugin;

use crate::input::add_menu_input;
use crate::show::take_menu_show_request;
use crate::MenuScreen;

/// Queue a home-screen spawn (despawns any existing home UI first).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowHome;

/// Marker on the spawned home-screen root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct HomeScreen;

/// Home destinations. Each pickable row stamps this as a component.
///
/// In-screen: [`MenuFocus<Self>`] / [`menu_components::MenuActivate<Self>`] bubble
/// to this root. Outside the screen: [`republish_menu_activate`] copies activate
/// as a [`Message`].
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

	pub fn description(self) -> &'static str {
		match self {
			Self::Discovery => {
				"Roam the world of Maybraid. Find the remnants of the hidden thread. Grow within the world."
			}
			Self::Reliquary => {
				"Team up to gather and return artifacts from an opponent's reliquary. Move fast. Wager yourself."
			}
			Self::Characters => "Create and edit your characters. Check your inventory.",
			Self::TrainingGround => "Run around with your characters in a small arena.",
			Self::Settings => "Adjust user and system settings to your liking.",
		}
	}
}

impl HomeScreen {
	pub fn scene() -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(
				TextCursorColumn::new(
					"Maybraid",
					HomeMenuChoice::ALL.into_iter().map(|choice| (choice.label(), choice)),
				)
				.scene(),
			),
			Box::new(TextMenuDescription::scene(HomeMenuChoice::Discovery.description())),
		];
		bsn! {
			HomeScreen
			MenuScreen
			MenuController
			BackgroundColor(Color::NONE)
			Node {
				width: percent(100),
				height: percent(100),
			}
			Pickable::IGNORE
			on(sync_home_description)
			on(republish_menu_activate::<HomeMenuChoice>)
			Children [ {children} ]
		}
	}
}

pub fn request_show_home(commands: &mut Commands) {
	commands.spawn(RequestShowHome);
}

pub struct HomeScreenPlugin;

impl Plugin for HomeScreenPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.add_plugins(TextMenuPlugin::<HomeMenuChoice>::default())
			.add_systems(Update, apply_show_home);
	}
}

fn apply_show_home(
	mut commands: Commands,
	requests: Query<Entity, With<RequestShowHome>>,
	existing: Query<Entity, With<MenuScreen>>,
) {
	if !take_menu_show_request(&mut commands, &requests, &existing) {
		return;
	}
	commands.spawn_scene(HomeScreen::scene());
}

fn sync_home_description(
	focus: On<MenuFocus<HomeMenuChoice>>,
	children: Query<&Children>,
	mut lines: Query<&mut Text, With<TextMenuDescription>>,
) {
	set_description_for_menu(
		focus.event().entity,
		focus.event().choice.description(),
		&children,
		&mut lines,
	);
}

#[cfg(test)]
mod tests {
	use super::HomeMenuChoice;

	#[test]
	fn descriptions_are_nonempty() {
		for choice in HomeMenuChoice::ALL {
			assert!(!choice.description().is_empty());
		}
	}
}
