//! Gameplay intents derived from [`maybraid_virtual_pad_controller::VirtualPad`].

use bevy::prelude::*;
use maybraid_controllers_core::Routed;

/// A semantic control addressed to its original actor and source binding.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct ActorIntentEvent {
	pub entity: Entity,
	pub input: Routed<CharacterIntent>,
}

impl ActorIntentEvent {
	/// Deliver a semantic control only while its original actor binding remains valid.
	pub fn queue(input: Routed<CharacterIntent>, commands: &mut Commands) {
		input.queue(commands, |world, input| {
			if let Some(entity) = input.target().actor_entity() {
				world.trigger(Self { entity, input });
			}
		});
	}
}

/// This tick's addressed semantic frame, including an empty neutral frame.
#[derive(Component, Clone, Debug, Default)]
pub struct CharacterIntentFrame(pub(crate) Option<Routed<Vec<CharacterIntent>>>);

impl CharacterIntentFrame {
	pub fn new(input: Routed<Vec<CharacterIntent>>) -> Self {
		Self(Some(input))
	}
	pub fn set(&mut self, input: Routed<Vec<CharacterIntent>>) {
		self.0 = Some(input);
	}
	pub fn get(&self) -> Option<&Routed<Vec<CharacterIntent>>> {
		self.0.as_ref()
	}
	pub fn clear(&mut self) {
		self.0 = None;
	}
}

/// One frame of character control. Analog variants may repeat while held.
#[derive(Clone, Copy, Debug, PartialEq)]
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
///
/// Actor ownership of the selected category lives in `maybraid-equipment`.
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
	/// Shell actions resolve before gameplay children from the same physical frame.
	pub const fn is_shell(self) -> bool {
		matches!(self, Self::InGameMenu | Self::Inventory | Self::ToggleMap)
	}

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
			Self::CycleEquippedQueue(_) => "cycle-equipped-queue",
			Self::SelectInventoryCyclingCategory(_) => "select-inventory-cycling-category",
			Self::InGameMenu => "in-game-menu",
			Self::Inventory => "inventory",
			Self::PowerUseItem => "power-use-item",
			Self::ToggleMap => "toggle-map",
		}
	}
}
