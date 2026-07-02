use bevy::prelude::*;
use character_ui_menu::Section;

use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crate::widgets::{text, ToggleSectionKey, ACTIVE, INACTIVE};

impl<T: RenderMenu> RenderMenu for Section<T> {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		let open = context.sections.is_open(self.label);
		parent
			.spawn((
				Node {
					width: Val::Percent(100.0),
					flex_direction: FlexDirection::Column,
					row_gap: Val::Px(crate::widgets::MENU_VERTICAL_GAP),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|section_parent| {
				section_parent.spawn((
					Button,
					Node {
						min_width: Val::Px(28.0),
						height: Val::Px(crate::widgets::BUTTON_HEIGHT),
						padding: UiRect::axes(
							Val::Px(7.0),
							Val::Px(crate::widgets::MENU_BUTTON_PADDING_V),
						),
						justify_content: JustifyContent::Center,
						align_items: AlignItems::Center,
						..default()
					},
					BackgroundColor(if open { ACTIVE } else { INACTIVE }),
					ToggleSectionKey(self.label),
				))
				.with_children(|button| {
					text(
						button,
						&format!("{} {}", if open { "v" } else { ">" }, self.label),
						10.0,
						Color::WHITE,
					);
				});
				if open {
					self.value.render_with(renderer, section_parent, context);
				}
			});
	}
}
