//! Gameplay intents derived from [`maybraid_input::VirtualPad`].

use bevy::prelude::*;

/// One frame of character control. Analog variants may repeat while held.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub enum CharacterIntent {
	Move(Vec2),
	Look(Vec2),
	Focus(f32),
	/// Cheek-weld ADS with iron-sight FOV. Left bumper / middle mouse.
	Ads(f32),
	/// Both bumpers (Xbox LB+RB) or keyboard **C+V**. Opens Discover skill maps.
	SkillMap,
	UseItem(f32),
	StartSprint,
	StopSprint,
	SwapPov,
	Jump,
	ExitInteraction,
	StartInteraction,
	/// Cycle the weapon queue (pad **Y** / keyboard **Y**).
	SwapActive,
	InGameMenu,
	Inventory,
	PowerUseItem,
}

impl CharacterIntent {
	pub fn label(self) -> &'static str {
		match self {
			Self::Move(_) => "move",
			Self::Look(_) => "look",
			Self::Focus(_) => "focus",
			Self::Ads(_) => "ads",
			Self::SkillMap => "skill-map",
			Self::UseItem(_) => "use-item",
			Self::StartSprint => "start-sprint",
			Self::StopSprint => "stop-sprint",
			Self::SwapPov => "swap-pov",
			Self::Jump => "jump",
			Self::ExitInteraction => "exit-interaction",
			Self::StartInteraction => "start-interaction",
			Self::SwapActive => "swap-active",
			Self::InGameMenu => "in-game-menu",
			Self::Inventory => "inventory",
			Self::PowerUseItem => "power-use-item",
		}
	}
}
