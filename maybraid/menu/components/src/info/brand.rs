//! Upper-left brand plus the current game mode.

use bevy::prelude::*;
use bevy::scene::prelude::{Scene, bsn, template_value};
use bevy::text::{FontSourceTemplate, Justify};

use crate::theme::{BARLOW_BLACK, BRAND_MODE_FONT_SIZE, COLUMN_INSET, TEXT_YELLOW};

/// Default product name in [`BrandModeLine`].
pub const BRAND_NAME: &str = "Maybraid";

/// Marker on the brand / mode [`Text`]. Screens write the string.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct BrandModeTitle;

/// Which top corner holds [`BrandModeLine`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrandModeCorner {
	#[default]
	TopLeft,
	TopRight,
}

/// Upper-corner chrome: `Maybraid - <mode>`.
pub struct BrandModeLine {
	pub brand: String,
	pub mode: String,
	pub corner: BrandModeCorner,
}

impl BrandModeLine {
	pub fn new(mode: impl Into<String>) -> Self {
		Self { brand: BRAND_NAME.into(), mode: mode.into(), corner: BrandModeCorner::TopLeft }
	}

	pub fn at(mut self, corner: BrandModeCorner) -> Self {
		self.corner = corner;
		self
	}

	pub fn display(brand: &str, mode: &str) -> String {
		format!("{brand} - {mode}")
	}

	pub fn scene(self) -> impl Scene + 'static {
		let label = Self::display(&self.brand, &self.mode);
		let justify = match self.corner {
			BrandModeCorner::TopLeft => Justify::Left,
			BrandModeCorner::TopRight => Justify::Right,
		};
		let node = match self.corner {
			BrandModeCorner::TopLeft => Node {
				position_type: PositionType::Absolute,
				top: Val::Px(COLUMN_INSET),
				left: Val::Px(COLUMN_INSET),
				..default()
			},
			BrandModeCorner::TopRight => Node {
				position_type: PositionType::Absolute,
				top: Val::Px(COLUMN_INSET),
				right: Val::Px(COLUMN_INSET),
				..default()
			},
		};
		bsn! {
			BrandModeTitle
			template_value(Text::new(label))
			TextFont {
				font: FontSourceTemplate::Handle(BARLOW_BLACK),
				font_size: px(BRAND_MODE_FONT_SIZE),
			}
			TextColor(TEXT_YELLOW)
			TextLayout::new(justify, bevy::text::LineBreak::NoWrap)
			template_value(node)
			Pickable::IGNORE
		}
	}
}

/// Write `value` onto the [`BrandModeTitle`] under `root` (the screen).
pub fn set_brand_mode_title(
	root: Entity,
	value: impl Into<String>,
	children: &Query<&Children>,
	lines: &mut Query<&mut Text, With<BrandModeTitle>>,
) {
	let value = value.into();
	set_brand_under(root, &value, children, lines);
}

fn set_brand_under(
	entity: Entity,
	value: &str,
	children: &Query<&Children>,
	lines: &mut Query<&mut Text, With<BrandModeTitle>>,
) -> bool {
	if let Ok(mut text) = lines.get_mut(entity) {
		text.0 = value.to_string();
		return true;
	}
	let Ok(kids) = children.get(entity) else {
		return false;
	};
	for child in kids {
		if set_brand_under(*child, value, children, lines) {
			return true;
		}
	}
	false
}

#[cfg(test)]
mod tests {
	use super::{BRAND_NAME, BrandModeLine};

	#[test]
	fn display_joins_brand_and_mode() {
		assert_eq!(BrandModeLine::display(BRAND_NAME, "Discovery"), "Maybraid - Discovery");
	}
}
