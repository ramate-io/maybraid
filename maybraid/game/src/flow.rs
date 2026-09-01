//! Title / characters / world routing for the Maybraid executable.

use bevy::prelude::*;
use menu_screens::HomeMenuChoice;

/// Which shell the executable is showing. World gameplay is only live in [`Self::World`].
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameFlow {
	#[default]
	Home,
	Characters,
	World,
}

/// Home-row destinations that the executable implements.
pub fn home_destination(choice: HomeMenuChoice) -> Option<GameFlow> {
	match choice {
		HomeMenuChoice::Discovery | HomeMenuChoice::Reliquary => Some(GameFlow::World),
		HomeMenuChoice::Characters => Some(GameFlow::Characters),
		HomeMenuChoice::TrainingGround | HomeMenuChoice::Settings => None,
	}
}

/// In-game brand label for Discovery / Reliquary. `None` when the choice is not a world mode.
pub fn world_mode_label(choice: HomeMenuChoice) -> Option<&'static str> {
	match choice {
		HomeMenuChoice::Discovery => Some("Discovery"),
		HomeMenuChoice::Reliquary => Some("Reliquary"),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn discovery_and_reliquary_enter_world() {
		assert_eq!(home_destination(HomeMenuChoice::Discovery), Some(GameFlow::World));
		assert_eq!(home_destination(HomeMenuChoice::Reliquary), Some(GameFlow::World));
		assert_eq!(world_mode_label(HomeMenuChoice::Discovery), Some("Discovery"));
		assert_eq!(world_mode_label(HomeMenuChoice::Reliquary), Some("Reliquary"));
	}

	#[test]
	fn characters_is_its_own_shell() {
		assert_eq!(home_destination(HomeMenuChoice::Characters), Some(GameFlow::Characters));
		assert_eq!(world_mode_label(HomeMenuChoice::Characters), None);
	}

	#[test]
	fn training_and_settings_stay_on_home() {
		assert_eq!(home_destination(HomeMenuChoice::TrainingGround), None);
		assert_eq!(home_destination(HomeMenuChoice::Settings), None);
	}
}
