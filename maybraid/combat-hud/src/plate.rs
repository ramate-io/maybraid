//! Frosted corner-plate chrome shared by combat HUD panels.

use bevy::prelude::*;
use menu_components::{hud_glass_fill, hud_glass_stroke, HUD_GLASS_BORDER, HUD_GLASS_RADIUS};

pub(crate) const HUD_CORNER_INSET: f32 = 16.0;

const PLATE_PAD_X: f32 = 8.0;
const PLATE_PAD_Y: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HudPlateCorner {
	BottomLeft,
	TopRight,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HudPlateLayout {
	pub width: f32,
	pub row_gap: f32,
	pub corner: HudPlateCorner,
	/// Extra lift for a stacked plate (charge above vitals).
	pub bottom_lift: f32,
}

impl HudPlateLayout {
	fn node(self) -> Node {
		let mut node = Node {
			position_type: PositionType::Absolute,
			width: Val::Px(self.width),
			flex_direction: FlexDirection::Column,
			align_items: AlignItems::Center,
			row_gap: Val::Px(self.row_gap),
			padding: UiRect::axes(Val::Px(PLATE_PAD_X), Val::Px(PLATE_PAD_Y)),
			border: UiRect::all(Val::Px(HUD_GLASS_BORDER)),
			border_radius: BorderRadius::all(Val::Px(HUD_GLASS_RADIUS)),
			..default()
		};
		match self.corner {
			HudPlateCorner::BottomLeft => {
				node.left = Val::Px(HUD_CORNER_INSET);
				node.bottom = Val::Px(HUD_CORNER_INSET + self.bottom_lift);
			}
			HudPlateCorner::TopRight => {
				node.right = Val::Px(HUD_CORNER_INSET);
				node.top = Val::Px(HUD_CORNER_INSET);
			}
		}
		node
	}
}

/// Spawns a hidden, pick-ignored HUD plate with shared fill, stroke, and padding.
pub(crate) fn spawn_hud_plate(
	parent: &mut ChildSpawnerCommands,
	name: &'static str,
	marker: impl Bundle,
	layout: HudPlateLayout,
	children: impl FnOnce(&mut ChildSpawnerCommands),
) -> Entity {
	parent
		.spawn((
			Name::new(name),
			marker,
			layout.node(),
			hud_glass_fill(),
			hud_glass_stroke(false),
			Visibility::Hidden,
			Pickable::IGNORE,
		))
		.with_children(children)
		.id()
}
