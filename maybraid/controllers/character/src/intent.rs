//! Gameplay intents derived from [`maybraid_input::VirtualPad`].

use bevy::prelude::*;

/// Held / pressed / released for one digital channel.
///
/// [`Self::from_held`] is the only edge detector. Tools must not re-threshold analog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DigitalIntent {
	pub held: bool,
	pub pressed: bool,
	pub released: bool,
}

impl DigitalIntent {
	pub fn from_held(previous_held: bool, held: bool) -> Self {
		Self { held, pressed: held && !previous_held, released: !held && previous_held }
	}
}

/// Canonical use / attack edges at the controller boundary.
///
/// [`crate::produce::resolve_use_attack_state`] folds this frame's
/// [`CharacterIntent`] (pad, remaps, and scripted writes). Suppression is a
/// release because produce emits no [`CharacterIntent::UseItem`] once the pad
/// is gated. Tools read this resource instead of re-reading [`VirtualPad`] or
/// raw mouse.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct UseAttackState {
	pub use_item: DigitalIntent,
	pub use_amount: f32,
	pub attack_left: DigitalIntent,
	pub attack_right: DigitalIntent,
}

impl UseAttackState {
	pub fn advance(
		self,
		use_held: bool,
		use_amount: f32,
		attack_left: bool,
		attack_right: bool,
	) -> Self {
		Self {
			use_item: DigitalIntent::from_held(self.use_item.held, use_held),
			use_amount: if use_held { use_amount } else { 0.0 },
			attack_left: DigitalIntent::from_held(self.attack_left.held, attack_left),
			attack_right: DigitalIntent::from_held(self.attack_right.held, attack_right),
		}
	}
}

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
	/// Rising edge on the left trigger (LT / **F**). Fisticuffs maps this to a left jab.
	/// RMB still writes analog focus and must not emit this (see `collect_with_mouse`).
	AttackLeft,
	/// Rising edge on the right trigger (RT / **G**). Fisticuffs maps this to a right jab.
	/// LMB still writes analog UseItem and must not emit this.
	AttackRight,
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
	/// Cycle the equipped queue for the active inventory category (pad **Y** / keyboard **Y**).
	SwapActive,
	/// Cycle the equipped queue for the active category. **[** / **]** keyboard equivalents.
	/// Negative steps backward; non-negative advances.
	CycleEquippedQueue(i8),
	/// Select which equipped queue **Y** cycles. Pad **D-Pad** left / right / down.
	SelectInventoryCyclingCategory(InventoryCyclingCategory),
	InGameMenu,
	Inventory,
	PowerUseItem,
	/// Toggle the world overhead map. Keyboard **M** / Xbox **View** (pad Select).
	ToggleMap,
}

/// Which equipped inventory queue responds to **Y** / **[** / **]**.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InventoryCyclingCategory {
	#[default]
	Weapons,
	SkillMaps,
	Lubikans,
}

impl InventoryCyclingCategory {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Weapons => "Weapons",
			Self::SkillMaps => "Skill Maps",
			Self::Lubikans => "Lubikans",
		}
	}
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
			Self::AttackLeft => "attack-left",
			Self::AttackRight => "attack-right",
			Self::StartSprint => "start-sprint",
			Self::StopSprint => "stop-sprint",
			Self::SwapPov => "swap-pov",
			Self::Jump => "jump",
			Self::ChangeSquat => "change-squat",
			Self::ChangeProne => "change-prone",
			Self::ExitInteraction => "exit-interaction",
			Self::StartInteraction => "start-interaction",
			Self::SwapActive => "swap-active",
			Self::CycleEquippedQueue(_) => "cycle-equipped-queue",
			Self::SelectInventoryCyclingCategory(_) => "select-inventory-cycling-category",
			Self::InGameMenu => "in-game-menu",
			Self::Inventory => "inventory",
			Self::PowerUseItem => "power-use-item",
			Self::ToggleMap => "toggle-map",
		}
	}
}
