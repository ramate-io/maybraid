//! Session mode shown as `Maybraid - <mode>` on the in-game menu.

use bevy::prelude::*;
use menu_components::{BRAND_NAME, BrandModeLine};

/// Current game mode. The in-game menu reads this for the upper-left title.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct GameMode {
	pub label: String,
}

impl Default for GameMode {
	fn default() -> Self {
		Self { label: String::from("Discovery") }
	}
}

impl GameMode {
	pub fn new(label: impl Into<String>) -> Self {
		Self { label: label.into() }
	}

	pub fn title(&self) -> String {
		BrandModeLine::display(BRAND_NAME, &self.label)
	}
}

#[cfg(test)]
mod tests {
	use super::GameMode;

	#[test]
	fn default_title_uses_discovery() {
		assert_eq!(GameMode::default().title(), "Maybraid - Discovery");
	}
}
