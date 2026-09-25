//! Training Ground setup: who plays the rounds.
//!
//! Every Training respawn is a new round: a new seeded map for your character,
//! or a new trainee on the same map. [`TrainingSpawn`] holds the character mode
//! for the round being played and for the next one, which the pause menu can
//! flip mid-session.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use maybraid_menu_controller::MenuController;
use menu_components::info::description::{set_description_for_menu, TextMenuDescription};
use menu_components::single_select::{republish_menu_activate, MenuFocus};
use menu_components::{screen_back_scene, TextCursorColumn, TextCursorRow, TextMenuPlugin};

use crate::input::add_menu_input;
use crate::show::take_menu_show_request;
use crate::MenuScreen;

/// Queue a Training setup spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowTraining;

/// Marker on the spawned Training setup root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TrainingScreen;

/// Who plays a Training round. Each pickable row stamps this as a component.
///
/// In-screen: [`MenuFocus<Self>`] / [`menu_components::MenuActivate<Self>`] bubble
/// to this root. Outside the screen: [`republish_menu_activate`] copies activate
/// as a [`Message`].
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum TrainingCharacterChoice {
	#[default]
	Active,
	Random,
}

impl TrainingCharacterChoice {
	pub const ALL: [Self; 2] = [Self::Active, Self::Random];

	pub fn label(self) -> &'static str {
		match self {
			Self::Active => "Your character",
			Self::Random => "Random trainee",
		}
	}

	pub fn description(self) -> &'static str {
		match self {
			Self::Active => {
				"Train as your active character on a new map every round. What they pick up is saved."
			}
			Self::Random => {
				"Stay on one map and respawn as a new random character. Try loadouts you have not built. Nothing is saved."
			}
		}
	}

	pub fn toggled(self) -> Self {
		match self {
			Self::Active => Self::Random,
			Self::Random => Self::Active,
		}
	}
}

/// Character mode of a Training session. [`Self::next`] takes over when the
/// next round starts.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingSpawn {
	pub next: TrainingCharacterChoice,
	pub current: TrainingCharacterChoice,
}

impl TrainingSpawn {
	pub fn new(choice: TrainingCharacterChoice) -> Self {
		Self { next: choice, current: choice }
	}

	pub fn begin_round(&mut self) -> TrainingCharacterChoice {
		self.current = self.next;
		self.current
	}
}

impl TrainingScreen {
	pub fn scene() -> impl Scene + 'static {
		let rows = TrainingCharacterChoice::ALL
			.into_iter()
			.map(|choice| TextCursorRow::new(choice.label(), choice));
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(
				TextCursorColumn::rows("Training Ground", rows)
					.with_description(TrainingCharacterChoice::Active.description())
					.scene(),
			),
			Box::new(screen_back_scene()),
		];
		bsn! {
			TrainingScreen
			MenuScreen
			MenuController
			BackgroundColor(Color::NONE)
			Node {
				width: percent(100),
				height: percent(100),
			}
			Pickable::IGNORE
			on(sync_training_description)
			on(republish_menu_activate::<TrainingCharacterChoice>)
			Children [ {children} ]
		}
	}
}

pub fn request_show_training(commands: &mut Commands) {
	commands.spawn(RequestShowTraining);
}

pub struct TrainingScreenPlugin;

impl Plugin for TrainingScreenPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.add_plugins(TextMenuPlugin::<TrainingCharacterChoice>::default())
			.add_systems(Update, apply_show_training);
	}
}

fn apply_show_training(
	mut commands: Commands,
	requests: Query<Entity, With<RequestShowTraining>>,
	existing: Query<Entity, With<MenuScreen>>,
) {
	if !take_menu_show_request(&mut commands, &requests, &existing) {
		return;
	}
	commands.spawn_scene(TrainingScreen::scene());
}

fn sync_training_description(
	focus: On<MenuFocus<TrainingCharacterChoice>>,
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
	use super::{TrainingCharacterChoice, TrainingSpawn};

	#[test]
	fn labels_and_descriptions_are_nonempty() {
		for choice in TrainingCharacterChoice::ALL {
			assert!(!choice.label().is_empty());
			assert!(!choice.description().is_empty());
		}
	}

	#[test]
	fn the_next_mode_waits_for_the_next_round() {
		let mut spawn = TrainingSpawn::new(TrainingCharacterChoice::Active);
		spawn.next = spawn.next.toggled();
		assert_eq!(spawn.current, TrainingCharacterChoice::Active);
		assert_eq!(spawn.begin_round(), TrainingCharacterChoice::Random);
		assert_eq!(spawn.current, TrainingCharacterChoice::Random);
		assert_eq!(TrainingCharacterChoice::Random.toggled(), TrainingCharacterChoice::Active);
	}
}
