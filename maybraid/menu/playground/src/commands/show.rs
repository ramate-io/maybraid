//! `/show` subcommand: spawn a menu screen.

use crate::character::request_show_character;
use crate::loading_demo::LoadingDemo;
use bevy::prelude::*;
use clap::Subcommand;
use menu_screens::{request_show_home, request_show_loading};

#[derive(Clone, Copy, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Show {
	/// Spawn the Maybraid home screen (bottom-left text menu).
	Home,
	/// Spawn the standard loading page (spinning mark, bar, explainer).
	Loading,
	/// Spawn the Maybraid character-creator panel (right-justified HUD).
	Character,
}

impl Show {
	pub fn react(self, commands: &mut Commands) {
		match self {
			Show::Home => request_show_home(commands),
			Show::Loading => {
				request_show_loading(commands);
				commands.insert_resource(LoadingDemo::default());
			}
			Show::Character => request_show_character(commands),
		}
	}
}
