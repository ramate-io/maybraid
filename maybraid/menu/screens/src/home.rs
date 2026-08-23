//! Maybraid home screen: bottom-left title plus destination labels.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use game_commands::command::TextEntryFocus;
use menu_components::{
	activate_selected_text_menu_items, emit_menu_choice, emit_menu_focus, MenuComponentsPlugin,
	MenuFocus, TextMenu, TextMenuColumn, TextMenuDescription, TextMenuInputLock, TextMenuSystems,
};

/// Queue a home-screen spawn (despawns any existing home UI first).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowHome;

/// Marker on the spawned home-screen root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct HomeScreen;

/// Home destinations. Each pickable row stamps this as a component.
///
/// In-screen: [`MenuFocus<Self>`] is triggered on the [`TextMenu`] and bubbles
/// here. Outside the screen: click / Enter copies this value as a [`Message`].
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
				TextMenuColumn::new(
					"Maybraid",
					HomeMenuChoice::ALL.into_iter().map(|choice| (choice.label(), choice)),
				)
				.scene(),
			),
			Box::new(TextMenuDescription::scene(HomeMenuChoice::Discovery.description())),
		];
		bsn! {
			HomeScreen
			Node {
				width: percent(100),
				height: percent(100),
			}
			Pickable::IGNORE
			on(sync_home_description)
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
		if !app.is_plugin_added::<MenuComponentsPlugin>() {
			app.add_plugins(MenuComponentsPlugin);
		}
		app.add_message::<HomeMenuChoice>()
			.add_observer(emit_menu_choice::<HomeMenuChoice>)
			.add_systems(Update, sync_text_menu_input_lock.in_set(TextMenuSystems::InputLock))
			.add_systems(
				Update,
				(
					apply_show_home,
					emit_menu_focus::<HomeMenuChoice>.after(TextMenuSystems::Navigate),
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

fn sync_home_description(
	focus: On<MenuFocus<HomeMenuChoice>>,
	parents: Query<&ChildOf, With<TextMenu>>,
	children: Query<&Children>,
	mut lines: Query<&mut Text, With<TextMenuDescription>>,
) {
	let Ok(child_of) = parents.get(focus.event().entity) else {
		return;
	};
	let Ok(siblings) = children.get(child_of.parent()) else {
		return;
	};
	for child in siblings {
		if let Ok(mut text) = lines.get_mut(*child) {
			text.0 = focus.event().choice.description().to_string();
		}
	}
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
