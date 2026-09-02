//! Gameplay intents derived from [`maybraid_input::VirtualPad`].

use bevy::prelude::*;

/// One frame of character control. Analog variants may repeat while held.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub enum CharacterIntent {
	Move(Vec2),
	Look(Vec2),
	Focus(f32),
	UseItem(f32),
	StartSprint,
	StopSprint,
	SwapPov,
	Jump,
	ExitInteraction,
	StartInteraction,
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
