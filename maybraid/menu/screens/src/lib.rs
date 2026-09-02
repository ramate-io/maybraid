//! Maybraid menu screen plugins.
//!
//! Screens compose [`menu_components`] widgets. The playground is the first
//! consumer; later the game app will add the same plugins.

use bevy::prelude::*;

pub mod create_character;
pub mod game_mode;
pub mod home;
pub mod in_game;
pub mod input;
pub mod loading;
pub mod show;
pub mod spin_reveal;

pub use create_character::{
	CreateCharacterPlugin, CreateCharacterReady, RequestShowCreateCharacter,
	request_show_create_character,
};
pub use game_mode::GameMode;
pub use home::{HomeMenuChoice, HomeScreen, HomeScreenPlugin, RequestShowHome, request_show_home};
pub use in_game::{
	InGameMenuChoice, InGameScreen, InGameScreenPlugin, RequestShowInGame, request_show_in_game,
	request_show_in_game_with_mode,
};
pub use input::{MenuInputPlugin, add_menu_input};
pub use loading::{
	LoadingExplainerText, LoadingProgress, LoadingScreen, LoadingScreenPlugin,
	LoadingScreenSystems, RequestShowLoading, request_loading_explainer, request_loading_progress,
	request_show_loading,
};
pub use show::{despawn_menu_screens, take_menu_show_request};
pub use spin_reveal::{
	RequestShowSpinReveal, SpinRevealChoice, SpinRevealFinished, SpinRevealItems, SpinRevealScreen,
	SpinRevealScreenPlugin, SpinRevealSystems, request_show_spin_reveal,
};

/// Marker on every full-screen menu root so show-requests can replace each other.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MenuScreen;
