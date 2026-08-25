//! Maybraid menu screen plugins.
//!
//! Screens compose [`menu_components`] widgets. The playground is the first
//! consumer; later the game app will add the same plugins.

use bevy::prelude::*;

pub mod game_mode;
pub mod home;
pub mod in_game;
pub mod input;
pub mod loading;
pub mod show;

pub use game_mode::GameMode;
pub use home::{request_show_home, HomeMenuChoice, HomeScreen, HomeScreenPlugin, RequestShowHome};
pub use in_game::{
	request_show_in_game, request_show_in_game_with_mode, InGameMenuChoice, InGameScreen,
	InGameScreenPlugin, RequestShowInGame,
};
pub use input::{add_menu_input, MenuInputPlugin};
pub use loading::{
	request_loading_explainer, request_loading_progress, request_show_loading,
	LoadingExplainerText, LoadingProgress, LoadingScreen, LoadingScreenPlugin,
	LoadingScreenSystems, RequestShowLoading,
};
pub use show::take_menu_show_request;

/// Marker on every full-screen menu root so show-requests can replace each other.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MenuScreen;
