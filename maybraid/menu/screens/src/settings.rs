//! In-game settings: pause-menu overlay for debug and user toggles.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use maybraid_menu_controller::MenuController;
use menu_components::info::description::{set_description_for_menu, TextMenuDescription};
use menu_components::single_select::republish_menu_activate;
use menu_components::single_select::text_cursor::TextCursorColumn;
use menu_components::{
	screen_back_scene, set_brand_mode_title, BrandModeLine, BrandModeTitle, MenuFocus,
	TextColumnAlign, TextColumnAnchor, TextCursorRow, TextMenuPlugin,
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

/// Sun cascade quality. The game copies this onto the sky sun
/// (`maybraid_sky::ShadowQuality`) without a sky dependency here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InGameShadowQuality {
	Off,
	Low,
	#[default]
	High,
}

impl InGameShadowQuality {
	pub fn cycle(self) -> Self {
		match self {
			Self::High => Self::Low,
			Self::Low => Self::Off,
			Self::Off => Self::High,
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::High => "High",
			Self::Low => "Low",
			Self::Off => "Off",
		}
	}
}

/// Live pause-menu settings. The game copies [`Self::mob_hud`] onto the world
/// HUD and [`Self::shadows`] onto the sky sun quality resource.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InGameSettings {
	pub mob_hud: bool,
	pub shadows: InGameShadowQuality,
}

impl Default for InGameSettings {
	fn default() -> Self {
		Self { mob_hud: false, shadows: InGameShadowQuality::High }
	}
}

impl InGameSettings {
	pub fn mob_hud_state_label(self) -> &'static str {
		if self.mob_hud {
			"On"
		} else {
			"Off"
		}
	}

	pub fn state_label(self, choice: InGameSettingsChoice) -> &'static str {
		match choice {
			InGameSettingsChoice::Shadows => self.shadows.label(),
			InGameSettingsChoice::MobHud => self.mob_hud_state_label(),
		}
	}
}

/// Settings rows. Activate toggles or cycles the matching [`InGameSettings`] field.
///
/// In-screen: [`MenuFocus<Self>`] / [`menu_components::MenuActivate<Self>`] bubble
/// to this root. Outside the screen: [`republish_menu_activate`] copies activate
/// as a [`Message`].
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum InGameSettingsChoice {
	#[default]
	Shadows,
	MobHud,
}

impl InGameSettingsChoice {
	pub const ALL: [Self; 2] = [Self::Shadows, Self::MobHud];

	pub fn label(self) -> &'static str {
		match self {
			Self::Shadows => "Shadows",
			Self::MobHud => "Mob HUD",
		}
	}

	pub fn description(self) -> &'static str {
		match self {
			Self::Shadows => {
				"Sun cascade shadows. High is four maps to 150 m. Low is two maps to 60 m. Off disables the sun's shadow maps."
			}
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
							.with_subtext(settings.state_label(choice))
					}),
				)
				.anchored(TextColumnAnchor::Center)
				.aligned(TextColumnAlign::Center)
				.with_description(InGameSettingsChoice::Shadows.description())
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
		InGameSettingsChoice::Shadows => settings.shadows = settings.shadows.cycle(),
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
	use super::{InGameSettings, InGameSettingsChoice, InGameShadowQuality};

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
		assert_eq!(
			InGameSettings { mob_hud: false, shadows: InGameShadowQuality::High }
				.mob_hud_state_label(),
			"Off"
		);
		assert_eq!(
			InGameSettings { mob_hud: true, shadows: InGameShadowQuality::High }
				.mob_hud_state_label(),
			"On"
		);
	}

	#[test]
	fn shadows_start_high_and_cycle() {
		assert_eq!(InGameSettings::default().shadows, InGameShadowQuality::High);
		assert_eq!(InGameShadowQuality::High.cycle(), InGameShadowQuality::Low);
		assert_eq!(InGameShadowQuality::Low.cycle(), InGameShadowQuality::Off);
		assert_eq!(InGameShadowQuality::Off.cycle(), InGameShadowQuality::High);
		assert_eq!(
			InGameSettings::default().state_label(InGameSettingsChoice::Shadows),
			"High"
		);
	}
}
