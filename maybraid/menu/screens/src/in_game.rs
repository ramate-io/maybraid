//! In-game pause menu: centered actions plus upper-left brand / mode.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use maybraid_menu_controller::MenuController;
use menu_components::single_select::republish_menu_activate;
use menu_components::single_select::text_cursor::TextCursorColumn;
use menu_components::{
	set_brand_mode_title, BrandModeLine, BrandModeTitle, TextColumnAlign, TextColumnAnchor,
	TextMenuPlugin,
};

use crate::input::add_menu_input;
use crate::show::take_menu_show_request;
use crate::{GameMode, MenuScreen};

/// Queue an in-game menu spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RequestShowInGame {
	pub mode: Option<String>,
}

/// Marker on the spawned in-game menu root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct InGameScreen;

/// Pause-menu destinations. Each pickable row stamps this as a component.
///
/// In-screen: [`menu_components::MenuFocus<Self>`] /
/// [`menu_components::MenuActivate<Self>`] bubble to this root. Outside the
/// screen: [`republish_menu_activate`] copies activate as a [`Message`].
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum InGameMenuChoice {
	#[default]
	Character,
	Records,
	Help,
	Settings,
	Leave,
}

impl InGameMenuChoice {
	pub const ALL: [Self; 5] =
		[Self::Character, Self::Records, Self::Help, Self::Settings, Self::Leave];

	pub fn label(self) -> &'static str {
		match self {
			Self::Character => "Character",
			Self::Records => "Records",
			Self::Help => "Help",
			Self::Settings => "Settings",
			Self::Leave => "Leave",
		}
	}
}

impl InGameScreen {
	pub fn scene(mode: &GameMode) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(
				TextCursorColumn::untitled(
					InGameMenuChoice::ALL.into_iter().map(|choice| (choice.label(), choice)),
				)
				.anchored(TextColumnAnchor::Center)
				.aligned(TextColumnAlign::Center)
				.scene(),
			),
			Box::new(BrandModeLine::new(mode.label.clone()).scene()),
		];
		bsn! {
			InGameScreen
			MenuScreen
			MenuController
			Node {
				width: percent(100),
				height: percent(100),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
			}
			Pickable::IGNORE
			on(republish_menu_activate::<InGameMenuChoice>)
			Children [ {children} ]
		}
	}
}

pub fn request_show_in_game(commands: &mut Commands) {
	commands.spawn(RequestShowInGame { mode: None });
}

pub fn request_show_in_game_with_mode(commands: &mut Commands, mode: impl Into<String>) {
	commands.spawn(RequestShowInGame { mode: Some(mode.into()) });
}

pub struct InGameScreenPlugin;

impl Plugin for InGameScreenPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.init_resource::<GameMode>()
			.add_plugins(TextMenuPlugin::<InGameMenuChoice>::default())
			.add_systems(Update, (apply_show_in_game, sync_in_game_brand));
	}
}

fn apply_show_in_game(
	mut commands: Commands,
	requests: Query<(Entity, &RequestShowInGame)>,
	existing: Query<Entity, With<MenuScreen>>,
	mut mode: ResMut<GameMode>,
) {
	let Some((_, request)) = requests.iter().last() else {
		return;
	};
	if let Some(label) = request.mode.as_ref() {
		mode.label.clone_from(label);
	}
	if !take_menu_show_request(&mut commands, requests.iter().map(|(entity, _)| entity), &existing)
	{
		return;
	}
	commands.spawn_scene(InGameScreen::scene(&mode));
}

fn sync_in_game_brand(
	mode: Res<GameMode>,
	screens: Query<Entity, With<InGameScreen>>,
	children: Query<&Children>,
	mut titles: Query<&mut Text, With<BrandModeTitle>>,
) {
	if !mode.is_changed() {
		return;
	}
	let title = mode.title();
	for screen in &screens {
		set_brand_mode_title(screen, title.clone(), &children, &mut titles);
	}
}

#[cfg(test)]
mod tests {
	use super::InGameMenuChoice;

	#[test]
	fn labels_are_nonempty() {
		for choice in InGameMenuChoice::ALL {
			assert!(!choice.label().is_empty());
		}
	}
}
