use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::show::LastVfxShow;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "VFX — / cmd — WASD/Space/Shift — mouse look".into(),
		empty_console_text:
			"Console: `fiery-explosion`, `show dust`, `orbit`, `pause`, `play`, `reset`, `cancel`, `spread`, `help`".into(),
		root_background: Color::srgba(0.08, 0.09, 0.12, 0.86),
		controls_hint: "show dust — solo/mute/unsolo/unmute — pause/step/play — reset/reload — Enter".into(),
	}
}

pub(crate) fn sync_command_status_text(
	last: Res<LastVfxShow>,
	mut status: ResMut<GameCommandStatusText>,
) {
	if last.label.is_empty() {
		if status.0.is_empty() {
			status.0 = "vfx: `fiery-explosion`".into();
		}
		return;
	}
	status.0 = format!("last {}", last.label);
}

#[derive(Component)]
pub(crate) struct LayerInspectorPanel;

#[derive(Component)]
pub(crate) struct LayerInspectorText;

pub(crate) fn setup_layer_inspector(mut commands: Commands) {
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(8.),
				left: Val::Px(8.),
				width: Val::Percent(55.),
				max_height: Val::Percent(40.),
				padding: UiRect::all(Val::Px(8.)),
				overflow: Overflow::scroll_y(),
				..default()
			},
			ScrollPosition::default(),
			BackgroundColor(Color::srgba(0.08, 0.09, 0.12, 0.86)),
			Pickable::default(),
			LayerInspectorPanel,
		))
		.with_children(|panel| {
			panel.spawn((
				Text::new("Layers — show an effect to inspect it"),
				TextFont { font_size: FontSize::Px(12.), ..default() },
				TextColor(Color::WHITE),
				Node { flex_shrink: 0., ..default() },
				Pickable::IGNORE,
				LayerInspectorText,
			));
		});
}

pub(crate) fn sync_layer_inspector(
	inspector: Res<crate::inspector::PreviewInspector>,
	drawer: Res<game_commands::ui::GameCommandDrawerVisible>,
	mut text: Single<&mut Text, With<LayerInspectorText>>,
	mut panel: Single<&mut Visibility, With<LayerInspectorPanel>>,
) {
	**panel = if drawer.0 { Visibility::Inherited } else { Visibility::Hidden };
	text.0 = format!("Layers · solo/mute <index> · mouse wheel to scroll\n{}", inspector.summary());
}

pub(crate) fn scroll_layer_inspector(
	mut event: On<game_commands::ui::ConsoleUiScroll>,
	mut panel: Query<(&mut ScrollPosition, &ComputedNode), With<LayerInspectorPanel>>,
) {
	let Ok((mut position, node)) = panel.get_mut(event.entity) else {
		return;
	};
	let maximum = ((node.content_size().y - node.size().y) * node.inverse_scale_factor()).max(0.);
	position.y = (position.y + event.delta.y).clamp(0., maximum);
	event.propagate(false);
}
