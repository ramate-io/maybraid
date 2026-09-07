//! In-game settings: pause-menu overlay for debug and user toggles.

use bevy::prelude::*;
use bevy::scene::prelude::{Scene, bsn};
use maybraid_menu_controller::MenuController;
use menu_components::info::description::{TextMenuDescription, set_description_for_menu};
use menu_components::single_select::republish_menu_activate;
use menu_components::single_select::text_cursor::TextCursorColumn;
use menu_components::{
	BrandModeLine, BrandModeTitle, MenuFocus, TextColumnAlign, TextColumnAnchor, TextCursorRow,
	TextMenuPlugin, screen_back_scene, set_brand_mode_title,
};

use crate::input::add_menu_input;
use crate::show::take_menu_show_request;
use crate::{GameMode, MenuScreen};

/// Queue an in-game settings spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct RequestShowInGameSettings;

/// Marker on the spawned in-game settings root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct InGameSettingsScreen;

/// Live pause-menu settings. The game copies [`Self::mob_hud`] onto the world HUD.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InGameSettings {
	pub mob_hud: bool,
}

impl Default for InGameSettings {
	fn default() -> Self {
		Self { mob_hud: false }
	}
}

impl InGameSettings {
	pub fn mob_hud_state_label(self) -> &'static str {
		if self.mob_hud { "On" } else { "Off" }
	}
}

/// Settings rows. Activate toggles the matching [`InGameSettings`] flag.
///
/// In-screen: [`MenuFocus<Self>`] / [`menu_components::MenuActivate<Self>`] bubble
/// to this root. Outside the screen: [`republish_menu_activate`] copies activate
/// as a [`Message`].
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum InGameSettingsChoice {
	#[default]
	MobHud,
}

impl InGameSettingsChoice {
	pub const ALL: [Self; 1] = [Self::MobHud];

	pub fn label(self) -> &'static str {
		match self {
			Self::MobHud => "Mob HUD",
		}
	}

	pub fn description(self) -> &'static str {
		match self {
			Self::MobHud => {
				"Pins and colored poles on presented mob hosts. Use this to find where groups should stand."
			}
		}
	}
}

impl InGameSettingsScreen {
	pub fn scene(mode: &GameMode, settings: InGameSettings) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(
				TextCursorColumn::rows(
					"Settings",
					InGameSettingsChoice::ALL.into_iter().map(|choice| {
						TextCursorRow::new(choice.label(), choice)
							.with_subtext(settings.mob_hud_state_label())
					}),
				)
				.anchored(TextColumnAnchor::Center)
				.aligned(TextColumnAlign::Center)
				.with_description(InGameSettingsChoice::MobHud.description())
				.scene(),
			),
			Box::new(BrandModeLine::new(mode.label.clone()).scene()),
			Box::new(screen_back_scene()),
		];
		bsn! {
			InGameSettingsScreen
			MenuScreen
			MenuController
			Node {
				width: percent(100),
				height: percent(100),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
			}
			Pickable::IGNORE
			on(sync_settings_description)
			on(republish_menu_activate::<InGameSettingsChoice>)
			Children [ {children} ]
		}
	}
}

pub fn request_show_in_game_settings(commands: &mut Commands) {
	commands.spawn(RequestShowInGameSettings);
}

pub struct InGameSettingsPlugin;

impl Plugin for InGameSettingsPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.init_resource::<GameMode>()
			.init_resource::<InGameSettings>()
			.add_plugins(TextMenuPlugin::<InGameSettingsChoice>::default())
			.add_systems(
				Update,
				(
					apply_in_game_settings_choice,
					apply_show_in_game_settings,
					sync_in_game_settings_brand,
				)
					.chain(),
			);
	}
}

fn apply_in_game_settings_choice(
	mut choices: MessageReader<InGameSettingsChoice>,
	mut settings: ResMut<InGameSettings>,
	mut commands: Commands,
) {
	let Some(choice) = choices.read().last().copied() else {
		return;
	};
	match choice {
		InGameSettingsChoice::MobHud => settings.mob_hud = !settings.mob_hud,
	}
	request_show_in_game_settings(&mut commands);
}

fn apply_show_in_game_settings(
	mut commands: Commands,
	requests: Query<Entity, With<RequestShowInGameSettings>>,
	existing: Query<Entity, With<MenuScreen>>,
	mode: Res<GameMode>,
	settings: Res<InGameSettings>,
) {
	if !take_menu_show_request(&mut commands, &requests, &existing) {
		return;
	}
	commands.spawn_scene(InGameSettingsScreen::scene(&mode, *settings));
}

fn sync_in_game_settings_brand(
	mode: Res<GameMode>,
	screens: Query<Entity, With<InGameSettingsScreen>>,
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

fn sync_settings_description(
	focus: On<MenuFocus<InGameSettingsChoice>>,
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
	use super::{InGameSettings, InGameSettingsChoice};

	#[test]
	fn labels_and_descriptions_are_nonempty() {
		for choice in InGameSettingsChoice::ALL {
			assert!(!choice.label().is_empty());
			assert!(!choice.description().is_empty());
		}
	}

	#[test]
	fn mob_hud_starts_off() {
		assert!(!InGameSettings::default().mob_hud);
		assert_eq!(InGameSettings { mob_hud: false }.mob_hud_state_label(), "Off");
		assert_eq!(InGameSettings { mob_hud: true }.mob_hud_state_label(), "On");
	}
}
