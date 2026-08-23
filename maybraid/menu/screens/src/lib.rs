//! Maybraid menu screen plugins.
//!
//! Screens compose [`menu_components`] widgets. The playground is the first
//! consumer; later the game app will add the same plugins.

use bevy::prelude::*;

pub mod home;
pub mod loading;

pub use home::{request_show_home, HomeMenuChoice, HomeScreen, HomeScreenPlugin, RequestShowHome};
pub use loading::{
	request_loading_explainer, request_loading_progress, request_show_loading,
	LoadingExplainerText, LoadingProgress, LoadingScreen, LoadingScreenPlugin,
	LoadingScreenSystems, RequestShowLoading,
};

/// Marker on every full-screen menu root so show-requests can replace each other.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MenuScreen;
