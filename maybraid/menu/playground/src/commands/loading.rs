//! `/loading` subcommand: drive the loading page while it is shown.

use bevy::prelude::*;
use clap::Subcommand;
use menu_screens::{request_loading_explainer, request_loading_progress};

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Loading {
	/// Set the bar fill. `value` is 0 to 1.
	Progress { value: f32 },
	/// Set the explainer line under the bar.
	Explainer {
		#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
		text: Vec<String>,
	},
}

impl Loading {
	pub fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Loading::Progress { value } => {
				request_loading_progress(commands, value);
				*console = format!("loading progress: {value:.2}");
			}
			Loading::Explainer { text } => {
				let line = text.join(" ");
				request_loading_explainer(commands, line.clone());
				*console = format!("loading explainer: {line}");
			}
		}
	}
}
