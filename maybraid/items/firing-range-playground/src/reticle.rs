//! Center-screen crosshair.

use bevy::prelude::*;

pub(crate) fn spawn_reticle(mut commands: Commands) {
	commands
		.spawn((
			Name::new("reticle"),
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				position_type: PositionType::Absolute,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|root| {
			root.spawn((
				Node { width: Val::Px(14.0), height: Val::Px(2.0), ..default() },
				BackgroundColor(Color::srgba(0.95, 0.95, 0.9, 0.85)),
			));
			root.spawn((
				Node {
					width: Val::Px(2.0),
					height: Val::Px(14.0),
					position_type: PositionType::Absolute,
					..default()
				},
				BackgroundColor(Color::srgba(0.95, 0.95, 0.9, 0.85)),
			));
		});
}
