use bevy::prelude::*;

use crate::input::CommandConsoleOutput;
use crate::input::TextEntryFocus;
use crate::input::TypedCommandLine;
use crate::preview::PreviewConfig;
#[derive(Component)]
pub struct DebugHudRoot;

#[derive(Component)]
pub(crate) struct HudStatusLine;

#[derive(Component)]
pub(crate) struct HudConsoleBlock;

pub fn setup_debug_ui(mut commands: Commands) {
	let status_size = 12.0;
	let console_size = 11.0;

	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				bottom: Val::Px(6.0),
				left: Val::Px(8.0),
				right: Val::Px(8.0),
				padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(4.0),
				align_items: AlignItems::Stretch,
				..default()
			},
			BackgroundColor(Color::hsla(201.0, 0.69, 0.62, 0.82)),
			DebugHudRoot,
		))
		.with_children(|parent| {
			parent.spawn((
				Text::new("SDF playground · Tab/1/2 · +/- res · / cmd · WASD"),
				TextFont { font_size: status_size, ..default() },
				TextColor(Color::WHITE),
				HudStatusLine,
			));
			parent.spawn((
				Node {
					max_height: Val::Px(200.0),
					overflow: Overflow::clip(),
					..default()
				},
				BackgroundColor(Color::NONE),
			))
			.with_children(|row| {
				row.spawn((
					Text::new(""),
					TextFont { font_size: console_size, ..default() },
					TextColor(Color::srgba(0.95, 0.98, 1.0, 1.0)),
					HudConsoleBlock,
				));
			});
		});
}

fn panel_status(config: &PreviewConfig, line: &TypedCommandLine, text_focus: &TextEntryFocus) -> String {
	format!(
		"{} ({})  res_2={}  |  [/] {}  |  buf: {}",
		config.primitive,
		config.primitive.variant_key(),
		config.res_2,
		if text_focus.0 { "cmd ON" } else { "cmd off" },
		if line.0.is_empty() {
			"_".into()
		} else {
			line.0.clone()
		},
	)
}

pub(crate) fn update_debug_ui(
	camera_query: Query<&Transform, With<Camera3d>>,
	mut hud_text: ParamSet<(
		Query<&mut Text, With<HudStatusLine>>,
		Query<&mut Text, With<HudConsoleBlock>>,
	)>,
	config: Res<PreviewConfig>,
	typed: Res<TypedCommandLine>,
	text_focus: Res<TextEntryFocus>,
	console: Res<CommandConsoleOutput>,
) {
	let Ok(transform) = camera_query.single() else {
		return;
	};
	let pos = transform.translation;

	if let Ok(mut status) = hud_text.p0().single_mut() {
		status.0 = format!(
			"{}\nCam {:.1}, {:.1}, {:.1}   ·   help + Enter → HUD",
			panel_status(&config, &typed, &text_focus),
			pos.x,
			pos.y,
			pos.z
		);
	}
	if let Ok(mut block) = hud_text.p1().single_mut() {
		block.0 = if console.0.is_empty() {
			"Console: (errors & `help` output)".into()
		} else {
			console.0.clone()
		};
	}
}
