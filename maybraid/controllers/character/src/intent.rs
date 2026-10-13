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
	/// Unused hold chord. Discover maps now read stick flicks, not bumpers.
	SkillMap,
	UseItem(f32),
	StartSprint,
	StopSprint,
	SwapPov,
	Jump,
	/// Tap **B** (release before the hold threshold). Toggles stand ↔ squat; prone steps to squat.
	ChangeSquat,
	/// Hold **B** across the stance threshold. Enters prone from stand or squat.
	ChangeProne,
	ExitInteraction,
	StartInteraction,
	/// Cycle the weapon queue. D-Pad left/right while weapons are the cycle target.
	SwapActive,
	/// Choose which equipped queue the D-Pad cycles: Lubikan, weapons, or skill map
	/// (pad **Y** / keyboard **Y**).
	CycleInventoryTarget,
	/// Cycle the active equipped queue. D-Pad left/right, or **[** / **]**.
	/// Negative steps backward; non-negative advances. World routes this to
	/// weapons, skill maps, or Lubikans from the current cycle target.
	CycleSkillMap(i8),
	InGameMenu,
	Inventory,
	PowerUseItem,
	/// Toggle the world overhead map. Keyboard **M** / Xbox **View** (pad Select).
	ToggleMap,
	/// Open the overhead map. Pad **D-Pad up**.
	OpenMap,
	/// Close the overhead map. Pad **D-Pad down**.
	CloseMap,
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
			Self::ChangeSquat => "change-squat",
			Self::ChangeProne => "change-prone",
			Self::ExitInteraction => "exit-interaction",
			Self::StartInteraction => "start-interaction",
			Self::SwapActive => "swap-active",
			Self::CycleInventoryTarget => "cycle-inventory-target",
			Self::CycleSkillMap(_) => "cycle-skill-map",
			Self::InGameMenu => "in-game-menu",
			Self::Inventory => "inventory",
			Self::PowerUseItem => "power-use-item",
			Self::ToggleMap => "toggle-map",
			Self::OpenMap => "open-map",
			Self::CloseMap => "close-map",
		}
	}
}
