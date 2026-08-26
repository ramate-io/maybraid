//! Full-width labeled control row.

use bevy::prelude::*;

use crate::theme::PANEL_ROW_GAP;

/// Left-to-right row: label, then `controls`. `justify` packs the pair.
pub fn spawn_labeled_row(
	parent: &mut ChildSpawnerCommands,
	justify: JustifyContent,
	controls: impl FnOnce(&mut ChildSpawnerCommands),
) {
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(12.0),
				row_gap: Val::Px(PANEL_ROW_GAP),
				align_items: AlignItems::Center,
				justify_content: justify,
				flex_wrap: FlexWrap::Wrap,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(controls);
}
