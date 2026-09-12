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

/// Intermediate 3D shade scale. The window stays at the OS size; Native
/// keeps that physical viewport, 1× shades the logical size, and ¾ shades
/// 75% of logical. Used so [#824](https://github.com/ramate-io/maybraid/issues/824)
/// can tell fragment cost from geometry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InGamePixelCount {
	#[default]
	Native,
	One,
	ThreeQuarter,
}

impl InGamePixelCount {
	pub fn cycle(self) -> Self {
		match self {
			Self::Native => Self::One,
			Self::One => Self::ThreeQuarter,
			Self::ThreeQuarter => Self::Native,
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::Native => "Native",
			Self::One => "1×",
			Self::ThreeQuarter => "¾",
		}
	}

	/// Main-pass size from the live window physical viewport and OS scale.
	/// `None` is native (no override). Bevy needs a strictly smaller size.
	pub fn main_pass_size(self, viewport: UVec2, base_scale: f32) -> Option<UVec2> {
		let factor = match self {
			Self::Native => return None,
			Self::One => 1.0,
			Self::ThreeQuarter => 0.75,
		};
		let scale = factor / base_scale.max(1.0);
		let size = UVec2::new(
			((viewport.x as f32) * scale).round().max(1.0) as u32,
			((viewport.y as f32) * scale).round().max(1.0) as u32,
		);
		(size.x < viewport.x && size.y < viewport.y).then_some(size)
	}
}

/// Live pause-menu settings. The game copies [`Self::mob_hud`] onto the world
/// HUD, [`Self::shadows`] onto the sky sun, and [`Self::pixel_count`] onto the
/// world camera's extracted main-pass size.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InGameSettings {
	pub mob_hud: bool,
	pub shadows: InGameShadowQuality,
	pub pixel_count: InGamePixelCount,
}

impl Default for InGameSettings {
	fn default() -> Self {
		Self {
			mob_hud: false,
			shadows: InGameShadowQuality::High,
			pixel_count: InGamePixelCount::Native,
		}
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
			InGameSettingsChoice::PixelCount => self.pixel_count.label(),
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
	PixelCount,
	MobHud,
}

impl InGameSettingsChoice {
	pub const ALL: [Self; 3] = [Self::Shadows, Self::PixelCount, Self::MobHud];

	pub fn label(self) -> &'static str {
		match self {
			Self::Shadows => "Shadows",
			Self::PixelCount => "Pixel count",
			Self::MobHud => "Mob HUD",
		}
	}

	pub fn description(self) -> &'static str {
		match self {
			Self::Shadows => {
				"Sun cascade shadows. High is four maps to 150 m. Low is two maps to 60 m. Off disables the sun's shadow maps."
			}
			Self::PixelCount => {
				"How many pixels the 3D pass shades. The window stays native. Native is the OS scale (Retina 2×). 1× shades the logical size. ¾ is 75% of that. A big frame-time drop means fragment-bound; little change means geometry-bound."
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
		InGameSettingsChoice::PixelCount => settings.pixel_count = settings.pixel_count.cycle(),
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
	use bevy::prelude::*;

	use super::{InGamePixelCount, InGameSettings, InGameSettingsChoice, InGameShadowQuality};

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
			InGameSettings {
				mob_hud: false,
				shadows: InGameShadowQuality::High,
				pixel_count: InGamePixelCount::Native,
			}
			.mob_hud_state_label(),
			"Off"
		);
		assert_eq!(
			InGameSettings {
				mob_hud: true,
				shadows: InGameShadowQuality::High,
				pixel_count: InGamePixelCount::Native,
			}
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

	#[test]
	fn pixel_count_starts_native_and_scales_the_main_pass() {
		assert_eq!(InGameSettings::default().pixel_count, InGamePixelCount::Native);
		assert_eq!(InGamePixelCount::Native.cycle(), InGamePixelCount::One);
		assert_eq!(InGamePixelCount::One.cycle(), InGamePixelCount::ThreeQuarter);
		assert_eq!(InGamePixelCount::ThreeQuarter.cycle(), InGamePixelCount::Native);
		let retina = UVec2::new(2560, 1440);
		assert_eq!(InGamePixelCount::Native.main_pass_size(retina, 2.0), None);
		assert_eq!(InGamePixelCount::One.main_pass_size(retina, 2.0), Some(UVec2::new(1280, 720)));
		assert_eq!(
			InGamePixelCount::ThreeQuarter.main_pass_size(retina, 2.0),
			Some(UVec2::new(960, 540))
		);
		assert_eq!(InGamePixelCount::One.main_pass_size(UVec2::new(1280, 720), 1.0), None);
	}
}
