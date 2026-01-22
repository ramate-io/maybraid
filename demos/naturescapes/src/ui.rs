use bevy::prelude::*;
use engine::LoadedChunks;

#[derive(Component)]
pub struct CoordinateDisplay;

pub fn setup_debug_ui(mut commands: Commands) {
	log::info!("Setting up debug UI");

	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(10.0),
				left: Val::Px(10.0),
				padding: UiRect::all(Val::Px(10.0)),
				..default()
			},
			BackgroundColor(Color::hsla(201.0, 0.69, 0.62, 0.7)),
			CoordinateDisplay,
		))
		.with_children(|parent| {
			parent.spawn((
				Text::new("Position: (0.00, 0.00, 0.00)\nChunks: 0"),
				TextFont { font_size: 20.0, ..default() },
				TextColor(Color::WHITE),
			));
		});
}

pub fn update_coordinate_display(
	camera_query: Query<&Transform, (With<Camera3d>, Without<CoordinateDisplay>)>,
	mut text_query: Query<&mut Text>,
	coordinate_display_query: Query<Entity, With<CoordinateDisplay>>,
	children_query: Query<&Children>,
	loaded_chunks: Res<LoadedChunks>,
) {
	if let Ok(transform) = camera_query.single() {
		let pos = transform.translation;
		// Find the coordinate display entity and its children
		if let Ok(display_entity) = coordinate_display_query.single() {
			if let Ok(children) = children_query.get(display_entity) {
				if let Some(&text_entity) = children.first() {
					if let Ok(mut text) = text_query.get_mut(text_entity) {
						text.0 = format!(
							"Position: ({:.2}, {:.2}, {:.2})\nChunks loaded: {}",
							pos.x,
							pos.y,
							pos.z,
							loaded_chunks.chunks.len()
						);
					}
				}
			}
		}
	}
}
