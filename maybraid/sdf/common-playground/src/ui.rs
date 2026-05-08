use bevy::prelude::*;

use crate::{input::TypedSdfName, PreviewConfig};

use crate::input::TextEntryFocus;
use sdf_common::SdfCommonPrimitive;

#[derive(Component)]
pub struct DebugHudRoot;

pub fn setup_debug_ui(mut commands: Commands) {
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(10.0),
				left: Val::Px(10.0),
				padding: UiRect::all(Val::Px(10.0)),
				..default()
			},
			BackgroundColor(Color::hsla(201.0, 0.69, 0.62, 0.75)),
			DebugHudRoot,
		))
		.with_children(|parent| {
			parent.spawn((
				Text::new(
					"SDF Common Playground\n\nKeys: Tab cycle · 1 / 2 · +/- resolution · / text mode · WASD fly",
				),
				TextFont { font_size: 18.0, ..default() },
				TextColor(Color::WHITE),
			));
		});
}

fn panel_body(config: &PreviewConfig, typed: &TypedSdfName, text_focus: &TextEntryFocus) -> String {
	format!(
		"SDF: {} ({})\nres_2: {}  |  variants: {}\n\n[/] text mode: {} — Type name + Enter: {}\n(Esc clear · aliases: tapered_cylinder, noisy_cylinder, …)",
		config.primitive,
		config.primitive.variant_key(),
		config.res_2,
		SdfCommonPrimitive::all_variant_keys().join(", "),
		if text_focus.0 { "ON" } else { "OFF" },
		if typed.0.is_empty() {
			"_".into()
		} else {
			typed.0.clone()
		},
	)
}

pub fn update_debug_ui(
	camera_query: Query<&Transform, With<Camera3d>>,
	mut text_query: Query<&mut Text>,
	hud_query: Query<Entity, With<DebugHudRoot>>,
	children_query: Query<&Children>,
	config: Res<PreviewConfig>,
	typed: Res<TypedSdfName>,
	text_focus: Res<TextEntryFocus>,
) {
	let Ok(transform) = camera_query.single() else {
		return;
	};
	let pos = transform.translation;

	if let Ok(display_entity) = hud_query.single() {
		if let Ok(children) = children_query.get(display_entity) {
			if let Some(&text_entity) = children.first() {
				if let Ok(mut text) = text_query.get_mut(text_entity) {
					text.0 = format!(
						"{}\n\nCamera: ({:.1}, {:.1}, {:.1})",
						panel_body(&config, &typed, &text_focus),
						pos.x,
						pos.y,
						pos.z
					);
				}
			}
		}
	}
}
