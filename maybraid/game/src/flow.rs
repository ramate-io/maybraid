//! Title / characters / world routing for the Maybraid executable.

use bevy::prelude::*;
use menu_screens::HomeMenuChoice;

/// Which shell the executable is showing. World gameplay is only live in
/// [`Self::World`] while [`WorldPause`] is [`WorldPause::Playing`].
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameFlow {
	#[default]
	Home,
	Characters,
	World,
}

/// Pause overlay while [`GameFlow::World`] is active. Absent in other shells.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameFlow = GameFlow::World)]
pub enum WorldPause {
	#[default]
	Playing,
	Menu,
}

/// What the executable does with a home-row pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeRoute {
	World { label: &'static str },
	Characters,
	Unimplemented,
}

impl HomeRoute {
	pub fn from_choice(choice: HomeMenuChoice) -> Self {
		match choice {
			HomeMenuChoice::Discovery | HomeMenuChoice::Reliquary => {
				Self::World { label: choice.label() }
			}
			HomeMenuChoice::Characters => Self::Characters,
			HomeMenuChoice::TrainingGround | HomeMenuChoice::Settings => Self::Unimplemented,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn discovery_and_reliquary_enter_world() {
		assert_eq!(
			HomeRoute::from_choice(HomeMenuChoice::Discovery),
			HomeRoute::World { label: "Discovery" }
		);
		assert_eq!(
			HomeRoute::from_choice(HomeMenuChoice::Reliquary),
			HomeRoute::World { label: "Reliquary" }
		);
	}

	#[test]
	fn characters_is_its_own_shell() {
		assert_eq!(HomeRoute::from_choice(HomeMenuChoice::Characters), HomeRoute::Characters);
	}

	#[test]
	fn training_and_settings_stay_on_home() {
		assert_eq!(
			HomeRoute::from_choice(HomeMenuChoice::TrainingGround),
			HomeRoute::Unimplemented
		);
		assert_eq!(HomeRoute::from_choice(HomeMenuChoice::Settings), HomeRoute::Unimplemented);
	}
}
