//! Centered line under the loading bar.

use bevy::prelude::*;
use bevy::scene::prelude::{Scene, bsn, template_value};
use bevy::text::{FontSourceTemplate, LineBreak};

use crate::theme::{
	BARLOW_REGULAR, LOADING_BAR_WIDTH, LOADING_EXPLAINER_FONT_SIZE, TEXT_YELLOW_FAINT,
};

/// Marker on the loading explainer [`Text`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct LoadingExplainer;

impl LoadingExplainer {
	pub fn scene(initial: impl Into<String>) -> impl Scene + 'static {
		let initial = initial.into();
		bsn! {
			LoadingExplainer
			template_value(Text::new(initial))
			TextFont {
				font: FontSourceTemplate::Handle(BARLOW_REGULAR),
				font_size: px(LOADING_EXPLAINER_FONT_SIZE),
			}
			TextColor(TEXT_YELLOW_FAINT)
			TextLayout::new(Justify::Center, LineBreak::WordBoundary)
			Node {
				width: px(LOADING_BAR_WIDTH),
			}
			Pickable::IGNORE
		}
	}
}

/// Write `value` onto the [`LoadingExplainer`] under `root`.
pub fn set_loading_explainer(
	root: Entity,
	value: impl Into<String>,
	children: &Query<&Children>,
	lines: &mut Query<&mut Text, With<LoadingExplainer>>,
) {
	let value = value.into();
	set_text_under(root, &value, children, lines);
}

fn set_text_under(
	entity: Entity,
	value: &str,
	children: &Query<&Children>,
	lines: &mut Query<&mut Text, With<LoadingExplainer>>,
) -> bool {
	if let Ok(mut text) = lines.get_mut(entity) {
		text.0 = value.to_string();
		return true;
	}
	let Ok(kids) = children.get(entity) else {
		return false;
	};
	for child in kids {
		if set_text_under(*child, value, children, lines) {
			return true;
		}
	}
	false
}
