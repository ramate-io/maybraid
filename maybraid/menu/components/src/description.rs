//! Faint description strip along the bottom of the screen.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::{FontSourceTemplate, LineBreak};

use crate::theme::{
	BARLOW_REGULAR, COLUMN_INSET, DESCRIPTION_BOTTOM, DESCRIPTION_FONT_SIZE, TEXT_YELLOW_FAINT,
};

/// Marker on the bottom description [`Text`]. Screens write the string.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextMenuDescription;

impl TextMenuDescription {
	pub fn scene(initial: impl Into<String>) -> impl Scene + 'static {
		let initial = initial.into();
		bsn! {
			TextMenuDescription
			template_value(Text::new(initial))
			TextFont {
				font: FontSourceTemplate::Handle(BARLOW_REGULAR),
				font_size: px(DESCRIPTION_FONT_SIZE),
			}
			TextColor(TEXT_YELLOW_FAINT)
			TextLayout::new(Justify::Left, LineBreak::WordBoundary)
			Node {
				position_type: PositionType::Absolute,
				left: px(COLUMN_INSET),
				right: px(COLUMN_INSET),
				bottom: px(DESCRIPTION_BOTTOM),
			}
			Pickable::IGNORE
		}
	}
}
