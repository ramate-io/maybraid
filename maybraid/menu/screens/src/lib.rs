//! Maybraid menu screen plugins.
//!
//! Screens compose [`menu_components`] widgets. The playground is the first
//! consumer; later the game app will add the same plugins.

pub mod home;

pub use home::{request_show_home, HomeMenuChoice, HomeScreen, HomeScreenPlugin, RequestShowHome};
