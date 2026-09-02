//! Always-on combat HUD: health bars, world-space counters, directional hit ticks.

use bevy::prelude::*;
use player::{Npc, Player, CAPSULE_LENGTH, CAPSULE_RADIUS};

use crate::damage::{DamageTaken, Health};

const BAR_WIDTH: f32 = 240.0;
const BAR_HEIGHT: f32 = 18.0;
const WORLD_BAR_WIDTH: f32 = 120.0;
const WORLD_BAR_HEIGHT: f32 = 8.0;
const INDICATOR_COUNT: usize = 8;
const INDICATOR_RADIUS: f32 = 148.0;
const INDICATOR_LIFE: f32 = 1.4;
const INDICATOR_GROUP: f32 = 0.5;
const HEAD_LIFT: f32 = CAPSULE_LENGTH * 0.5 + CAPSULE_RADIUS + 0.38;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatantHud {
	Player,
	Npc,
}

#[derive(Component)]
pub(crate) struct HudBarFill;

#[derive(Component)]
pub(crate) struct HudBarLabel;

#[derive(Component)]
pub(crate) struct WorldHealthAnchor;

#[derive(Component)]
pub(crate) struct DamageTick {
	slot: usize,
}

#[derive(Clone, Copy)]
struct LiveTick {
	origin: Vec3,
	born: f32,
}

#[derive(Resource, Default)]
pub(crate) struct DamageTicks([Option<LiveTick>; INDICATOR_COUNT]);

pub(crate) fn spawn_combat_hud(mut commands: Commands) {
	commands
		.spawn((
			Name::new("combat-hud"),
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				..default()
			},
			GlobalZIndex(i32::MAX - 8),
			Pickable::IGNORE,
		))
		.with_children(|root| {
			root.spawn(Node {
				position_type: PositionType::Absolute,
				top: Val::Px(16.0),
				left: Val::Px(16.0),
				right: Val::Px(16.0),
				justify_content: JustifyContent::SpaceBetween,
				align_items: AlignItems::FlexStart,
				column_gap: Val::Px(24.0),
				..default()
			})
			.with_children(|row| {
				spawn_screen_bar(row, CombatantHud::Player, "YOU");
				spawn_screen_bar(row, CombatantHud::Npc, "NPC");
			});
			spawn_world_bar(root, CombatantHud::Npc);
			root.spawn((
				Node {
					position_type: PositionType::Absolute,
					left: Val::Percent(50.0),
					top: Val::Percent(50.0),
					width: Val::Px(0.0),
					height: Val::Px(0.0),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|ring| {
				for slot in 0..INDICATOR_COUNT {
					ring.spawn((
						DamageTick { slot },
						Node {
							position_type: PositionType::Absolute,
							left: Val::Px(-7.0),
							top: Val::Px(-22.0),
							width: Val::Px(14.0),
							height: Val::Px(28.0),
							border: UiRect::all(Val::Px(2.0)),
							border_radius: BorderRadius::all(Val::Px(2.0)),
							..default()
						},
						BackgroundColor(Color::srgba(1.0, 0.22, 0.16, 0.0)),
						BorderColor::all(Color::srgba(1.0, 0.85, 0.7, 0.0)),
						UiTransform::IDENTITY,
						Visibility::Hidden,
						Pickable::IGNORE,
					));
				}
			});
		});
}

fn spawn_screen_bar(parent: &mut ChildSpawnerCommands, kind: CombatantHud, title: &'static str) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(4.0),
				padding: UiRect::all(Val::Px(8.0)),
				min_width: Val::Px(BAR_WIDTH + 16.0),
				border_radius: BorderRadius::all(Val::Px(4.0)),
				..default()
			},
			BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.72)),
		))
		.with_children(|cluster| {
			cluster.spawn((
				kind,
				HudBarLabel,
				Text::new(format!("{title}  {0:.0}/{0:.0}", crate::damage::MAX_HEALTH)),
				TextFont { font_size: FontSize::Px(16.0), ..default() },
				TextColor(Color::srgb(0.95, 0.97, 1.0)),
			));
			cluster
				.spawn((
					Node {
						width: Val::Px(BAR_WIDTH),
						height: Val::Px(BAR_HEIGHT),
						padding: UiRect::all(Val::Px(2.0)),
						border_radius: BorderRadius::all(Val::Px(3.0)),
						..default()
					},
					BackgroundColor(Color::srgba(0.08, 0.09, 0.1, 0.95)),
				))
				.with_children(|track| {
					track.spawn((
						kind,
						HudBarFill,
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							border_radius: BorderRadius::all(Val::Px(2.0)),
							..default()
						},
						BackgroundColor(bar_color(1.0)),
					));
				});
		});
}

fn spawn_world_bar(parent: &mut ChildSpawnerCommands, kind: CombatantHud) {
	parent
		.spawn((
			kind,
			WorldHealthAnchor,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Px(WORLD_BAR_WIDTH),
				height: Val::Px(WORLD_BAR_HEIGHT + 14.0),
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::Center,
				row_gap: Val::Px(2.0),
				..default()
			},
			Visibility::Hidden,
			Pickable::IGNORE,
		))
		.with_children(|anchor| {
			anchor.spawn((
				kind,
				HudBarLabel,
				Text::new(""),
				TextFont { font_size: FontSize::Px(12.0), ..default() },
				TextColor(Color::srgb(1.0, 1.0, 1.0)),
			));
			anchor
				.spawn((
					Node {
						width: Val::Percent(100.0),
						height: Val::Px(WORLD_BAR_HEIGHT),
						border_radius: BorderRadius::all(Val::Px(2.0)),
						..default()
					},
					BackgroundColor(Color::srgba(0.05, 0.05, 0.06, 0.85)),
				))
				.with_children(|track| {
					track.spawn((
						kind,
						HudBarFill,
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							border_radius: BorderRadius::all(Val::Px(2.0)),
							..default()
						},
						BackgroundColor(bar_color(1.0)),
					));
				});
		});
}

pub(crate) fn sync_health_hud(
	players: Query<&Health, With<Player>>,
	npcs: Query<&Health, With<Npc>>,
	mut fills: Query<(&CombatantHud, &mut Node, &mut BackgroundColor), With<HudBarFill>>,
	mut labels: Query<(&CombatantHud, &mut Text), With<HudBarLabel>>,
) {
	let player = players.single().ok().copied();
	let npc = npcs.single().ok().copied();
	for (kind, mut node, mut color) in &mut fills {
		let health = readout(*kind, player, npc);
		node.width = Val::Percent(health.map(Health::fraction).unwrap_or(0.0) * 100.0);
		color.0 = bar_color(health.map(Health::fraction).unwrap_or(0.0));
	}
	for (kind, mut text) in &mut labels {
		text.0 = label_text(*kind, readout(*kind, player, npc));
	}
}

pub(crate) fn sync_world_health_bars(
	cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
	npcs: Query<&GlobalTransform, With<Npc>>,
	mut anchors: Query<(&mut Node, &mut Visibility), With<WorldHealthAnchor>>,
) {
	let Ok((camera, camera_transform)) = cameras.single() else {
		return;
	};
	let Some(npc) = npcs.single().ok().copied() else {
		for (_, mut visibility) in &mut anchors {
			*visibility = Visibility::Hidden;
		}
		return;
	};
	let head = npc.translation() + Vec3::Y * HEAD_LIFT;
	let screen = camera.world_to_viewport(camera_transform, head).ok();
	for (mut node, mut visibility) in &mut anchors {
		if let Some(screen) = screen {
			node.left = Val::Px(screen.x - WORLD_BAR_WIDTH * 0.5);
			node.top = Val::Px(screen.y - 22.0);
			*visibility = Visibility::Visible;
		} else {
			*visibility = Visibility::Hidden;
		}
	}
}

pub(crate) fn ingest_damage_indicators(
	time: Res<Time>,
	mut hits: MessageReader<DamageTaken>,
	players: Query<(), With<Player>>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
	mut ticks: ResMut<DamageTicks>,
) {
	let Ok(camera) = cameras.single() else {
		return;
	};
	let now = time.elapsed_secs();
	for hit in hits.read() {
		if players.get(hit.target).is_err() {
			continue;
		}
		assign_tick(&mut ticks.0, camera, hit.origin, now);
	}
}

pub(crate) fn update_damage_indicators(
	time: Res<Time>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
	mut ticks: ResMut<DamageTicks>,
	mut nodes: Query<(
		&DamageTick,
		&mut UiTransform,
		&mut BackgroundColor,
		&mut BorderColor,
		&mut Visibility,
	)>,
) {
	let now = time.elapsed_secs();
	for slot in &mut ticks.0 {
		if slot.is_some_and(|tick| now >= tick.born + INDICATOR_LIFE) {
			*slot = None;
		}
	}
	let Ok(camera) = cameras.single() else {
		for (_, _, _, _, mut visibility) in &mut nodes {
			*visibility = Visibility::Hidden;
		}
		return;
	};
	for (tick, mut transform, mut fill, mut border, mut visibility) in &mut nodes {
		let Some(live) = ticks.0[tick.slot] else {
			*visibility = Visibility::Hidden;
			continue;
		};
		let age = now - live.born;
		let alpha = indicator_alpha(age);
		let yaw = incoming_yaw(camera, live.origin);
		let offset = indicator_offset(yaw, INDICATOR_RADIUS);
		transform.translation = Val2::px(offset.x, offset.y);
		transform.rotation = Rot2::radians(yaw);
		fill.0 = Color::srgba(1.0, 0.22, 0.16, alpha);
		*border = BorderColor::all(Color::srgba(1.0, 0.9, 0.75, alpha * 0.9));
		*visibility = Visibility::Visible;
	}
}

fn readout(kind: CombatantHud, player: Option<Health>, npc: Option<Health>) -> Option<Health> {
	match kind {
		CombatantHud::Player => player,
		CombatantHud::Npc => npc,
	}
}

fn label_text(kind: CombatantHud, health: Option<Health>) -> String {
	let name = match kind {
		CombatantHud::Player => "YOU",
		CombatantHud::Npc => "NPC",
	};
	match health {
		Some(health) if !health.is_dead() => {
			format!("{name}  {:.0}/{:.0}", health.current, health.max)
		}
		_ => format!("{name}  DOWN"),
	}
}

fn bar_color(fraction: f32) -> Color {
	if fraction <= 0.0 {
		Color::srgb(0.28, 0.08, 0.08)
	} else if fraction < 0.35 {
		Color::srgb(0.86, 0.18, 0.14)
	} else if fraction < 0.7 {
		Color::srgb(0.92, 0.72, 0.18)
	} else {
		Color::srgb(0.28, 0.82, 0.38)
	}
}

fn assign_tick(
	slots: &mut [Option<LiveTick>; INDICATOR_COUNT],
	camera: &GlobalTransform,
	origin: Vec3,
	now: f32,
) {
	let yaw = incoming_yaw(camera, origin);
	let mut best_empty = None;
	let mut oldest = 0;
	let mut oldest_born = f32::MAX;
	for (index, slot) in slots.iter_mut().enumerate() {
		match slot {
			Some(live)
				if angle_delta(incoming_yaw(camera, live.origin), yaw) <= INDICATOR_GROUP =>
			{
				live.origin = origin;
				live.born = now;
				return;
			}
			Some(live) if live.born < oldest_born => {
				oldest_born = live.born;
				oldest = index;
			}
			None if best_empty.is_none() => best_empty = Some(index),
			_ => {}
		}
	}
	let index = best_empty.unwrap_or(oldest);
	slots[index] = Some(LiveTick { origin, born: now });
}

/// Camera-local yaw: 0 is forward / screen-up, positive is clockwise (right).
pub(crate) fn incoming_yaw(camera: &GlobalTransform, origin: Vec3) -> f32 {
	let local = camera.rotation().inverse() * (origin - camera.translation());
	local.x.atan2(-local.z)
}

pub(crate) fn indicator_offset(yaw: f32, radius: f32) -> Vec2 {
	Vec2::new(yaw.sin() * radius, -yaw.cos() * radius)
}

fn indicator_alpha(age: f32) -> f32 {
	if age <= 0.2 {
		1.0
	} else {
		(1.0 - (age - 0.2) / (INDICATOR_LIFE - 0.2)).clamp(0.0, 1.0)
	}
}

fn angle_delta(a: f32, b: f32) -> f32 {
	let mut delta = (a - b).abs();
	if delta > std::f32::consts::PI {
		delta = std::f32::consts::TAU - delta;
	}
	delta
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn forward_hit_sits_above_center() {
		let camera = GlobalTransform::from(
			Transform::from_translation(Vec3::ZERO).looking_to(Dir3::NEG_Z, Dir3::Y),
		);
		let yaw = incoming_yaw(&camera, Vec3::new(0.0, 0.0, -8.0));
		assert!(yaw.abs() < 0.05, "{yaw}");
		let offset = indicator_offset(yaw, 100.0);
		assert!(offset.y < -90.0, "{offset:?}");
		assert!(offset.x.abs() < 8.0, "{offset:?}");
	}

	#[test]
	fn right_hit_sits_on_screen_right() {
		let camera = GlobalTransform::from(
			Transform::from_translation(Vec3::ZERO).looking_to(Dir3::NEG_Z, Dir3::Y),
		);
		let yaw = incoming_yaw(&camera, Vec3::new(8.0, 0.0, 0.0));
		assert!((yaw - std::f32::consts::FRAC_PI_2).abs() < 0.05, "{yaw}");
		let offset = indicator_offset(yaw, 100.0);
		assert!(offset.x > 90.0, "{offset:?}");
		assert!(offset.y.abs() < 8.0, "{offset:?}");
	}
}
