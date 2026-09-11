//! Standalone objective chip. Sits on HUD chrome, not inside a camera image.

use bevy::prelude::*;

use crate::theme::{
	OBJECTIVE_MARKER_BORDER, OBJECTIVE_MARKER_FONT_SIZE, OBJECTIVE_MARKER_PAD_X,
	OBJECTIVE_MARKER_PAD_Y, OBJECTIVE_MARKER_RADIUS, TEXT_YELLOW,
};
use crate::HudFonts;

/// Preset faces for [`MenuObjective`]. Yellow is the in-game HUD chip.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MenuObjectiveKind {
	#[default]
	Yellow,
}

impl MenuObjectiveKind {
	pub fn color(self) -> Color {
		match self {
			Self::Yellow => TEXT_YELLOW,
		}
	}
}

/// Bordered label that stays readable on busy art.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct MenuObjective {
	pub label: String,
	pub color: Color,
}

impl MenuObjective {
	pub fn yellow(label: impl Into<String>) -> Self {
		Self { label: label.into(), color: MenuObjectiveKind::Yellow.color() }
	}
}

/// Spawn the chip as a child. Caller sets layout (absolute overlay, row, …).
pub fn spawn_menu_objective(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	objective: MenuObjective,
	extra: impl Bundle,
) {
	let color = objective.color;
	let fill = color.with_alpha(0.14);
	let label = objective.label.clone();
	parent
		.spawn((
			objective,
			Node {
				padding: UiRect::axes(
					Val::Px(OBJECTIVE_MARKER_PAD_X),
					Val::Px(OBJECTIVE_MARKER_PAD_Y),
				),
				border: UiRect::all(Val::Px(OBJECTIVE_MARKER_BORDER)),
				border_radius: BorderRadius::all(Val::Px(OBJECTIVE_MARKER_RADIUS)),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				flex_shrink: 0.0,
				..default()
			},
			BorderColor::all(color),
			BackgroundColor(fill),
			Pickable::IGNORE,
			extra,
		))
		.with_children(|chip| {
			chip.spawn((
				Text::new(label),
				fonts.item(OBJECTIVE_MARKER_FONT_SIZE),
				TextColor(color),
				Pickable::IGNORE,
			));
		});
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn yellow_uses_the_menu_face() {
		let chip = MenuObjective::yellow("Fireball 00A1");
		assert_eq!(chip.label, "Fireball 00A1");
		assert_eq!(chip.color, TEXT_YELLOW);
		assert_eq!(MenuObjectiveKind::Yellow.color(), TEXT_YELLOW);
	}
}
