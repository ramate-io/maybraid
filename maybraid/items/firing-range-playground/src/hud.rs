//! Always-on combat HUD: health bars, world-space counters, directional hit ticks.

use bevy::prelude::*;
use game_commands::GameCommandDrawerVisible;
use player::{Npc, Player, CAPSULE_LENGTH, CAPSULE_RADIUS};

use crate::damage::{DamageApplied, Health};
use crate::session::{CombatantKit, RangeSession};

const BAR_WIDTH: f32 = 240.0;
const BAR_HEIGHT: f32 = 18.0;
const WORLD_BAR_WIDTH: f32 = 120.0;
const WORLD_BAR_HEIGHT: f32 = 8.0;
const INDICATOR_COUNT: usize = 8;
const INDICATOR_RADIUS: f32 = 148.0;
const INDICATOR_LIFE: f32 = 1.4;
const INDICATOR_GROUP: f32 = 0.5;
const HEAD_LIFT: f32 = CAPSULE_LENGTH * 0.5 + CAPSULE_RADIUS + 0.38;
const GUN_CARD_WIDTH: f32 = 248.0;
const DRAWER_CLEARANCE: f32 = 290.0;
const POPUP_LIFE: f32 = 0.9;
const POPUP_RISE: f32 = 48.0;
const HIT_POINTS: u8 = 1;
const HEAD_POINTS: u8 = 2;
const DOWN_POINTS: u8 = 5;

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
pub(crate) struct CombatHudRoot;

#[derive(Component)]
pub(crate) struct GunStatsPanel;

#[derive(Component)]
pub(crate) struct GunStatsText;

#[derive(Component)]
pub(crate) struct WorldHudFill;

#[derive(Component)]
pub(crate) struct WorldHudLabel;

#[derive(Component)]
pub(crate) struct WorldHealthAnchor {
	target: Entity,
}

#[derive(Component)]
pub(crate) struct DamageTick {
	slot: usize,
}

#[derive(Component)]
pub(crate) struct CombatPopup {
	world: Vec3,
	born: f32,
	points: u8,
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
			CombatHudRoot,
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
			spawn_gun_stats_card(root);
		});
}

fn spawn_gun_stats_card(parent: &mut ChildSpawnerCommands) {
	parent
		.spawn((
			Name::new("gun-stats"),
			GunStatsPanel,
			Node {
				position_type: PositionType::Absolute,
				right: Val::Px(16.0),
				bottom: Val::Px(16.0),
				min_width: Val::Px(GUN_CARD_WIDTH),
				flex_direction: FlexDirection::Column,
				padding: UiRect::all(Val::Px(10.0)),
				border_radius: BorderRadius::all(Val::Px(4.0)),
				..default()
			},
			BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.72)),
			Pickable::IGNORE,
		))
		.with_children(|card| {
			card.spawn((
				GunStatsText,
				Text::new(""),
				TextFont { font_size: FontSize::Px(13.0), ..default() },
				TextColor(Color::srgb(0.92, 0.95, 0.98)),
			));
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

fn spawn_world_bar(parent: Entity, target: Entity, commands: &mut Commands) {
	commands.entity(parent).with_children(|root| {
		root.spawn((
			WorldHealthAnchor { target },
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
				WorldHudLabel,
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
						WorldHudFill,
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
	});
}

pub(crate) fn ensure_world_health_bars(
	mut commands: Commands,
	hud: Query<Entity, With<CombatHudRoot>>,
	npcs: Query<Entity, With<Npc>>,
	anchors: Query<(Entity, &WorldHealthAnchor)>,
) {
	let Ok(hud) = hud.single() else {
		return;
	};
	let live: Vec<Entity> = npcs.iter().collect();
	let existing: Vec<(Entity, Entity)> =
		anchors.iter().map(|(entity, anchor)| (entity, anchor.target)).collect();
	for (entity, target) in &existing {
		if !live.contains(target) {
			commands.entity(*entity).try_despawn();
		}
	}
	for npc in live {
		if existing.iter().any(|(_, target)| *target == npc) {
			continue;
		}
		spawn_world_bar(hud, npc, &mut commands);
	}
}

pub(crate) fn sync_health_hud(
	session: Res<RangeSession>,
	players: Query<&Health, With<Player>>,
	npcs: Query<&Health, With<Npc>>,
	mut fills: Query<(&CombatantHud, &mut Node, &mut BackgroundColor), With<HudBarFill>>,
	mut labels: Query<(&CombatantHud, &mut Text), With<HudBarLabel>>,
) {
	let player = players.single().ok().copied();
	let alive = npcs.iter().count();
	let total = session.npc_count.max(1);
	let npc = npcs.single().ok().copied();
	let field_fraction = (alive as f32 / total as f32).clamp(0.0, 1.0);
	for (kind, mut node, mut color) in &mut fills {
		let fraction = match *kind {
			CombatantHud::Player => player.map(Health::fraction).unwrap_or(0.0),
			CombatantHud::Npc if session.is_free_for_all() => field_fraction,
			CombatantHud::Npc => npc.map(Health::fraction).unwrap_or(0.0),
		};
		node.width = Val::Percent(fraction * 100.0);
		color.0 = bar_color(fraction);
	}
	for (kind, mut text) in &mut labels {
		text.0 = match *kind {
			CombatantHud::Player => label_text(CombatantHud::Player, player),
			CombatantHud::Npc if session.is_free_for_all() => {
				format!("FIELD  {alive}/{total}")
			}
			CombatantHud::Npc => label_text(CombatantHud::Npc, npc),
		};
	}
}

pub(crate) fn sync_gun_stats(
	drawer: Res<GameCommandDrawerVisible>,
	players: Query<Option<&CombatantKit>, With<Player>>,
	mut panels: Query<(&mut Node, &mut Visibility), With<GunStatsPanel>>,
	mut labels: Query<&mut Text, With<GunStatsText>>,
) {
	let Ok((mut node, mut visibility)) = panels.single_mut() else {
		return;
	};
	let Ok(mut text) = labels.single_mut() else {
		return;
	};
	node.bottom = if drawer.0 { Val::Px(DRAWER_CLEARANCE) } else { Val::Px(16.0) };
	let Ok(kit) = players.single() else {
		*visibility = Visibility::Hidden;
		return;
	};
	*visibility = Visibility::Visible;
	text.0 = gun_card_text(kit);
}

fn gun_card_text(kit: Option<&CombatantKit>) -> String {
	match kit {
		Some(kit) => format_gun_card(
			kit.firearm.body.label(),
			&kit.stats.catalog_detail(),
			&live_stat_rows(kit),
		),
		None => format_gun_card("bullpup", "Bolt · 25 DPC", &duel_stat_rows()),
	}
}

fn live_stat_rows(kit: &CombatantKit) -> Vec<(String, String)> {
	let live = kit.live.payload.amount;
	let catalog = f32::from(kit.stats.damage);
	kit.stats
		.stat_rows()
		.into_iter()
		.map(|(label, value)| {
			if label == "DPC" && (live - catalog).abs() >= 0.5 {
				(label, format!("{live:.0} ({catalog:.0})"))
			} else {
				(label, value)
			}
		})
		.collect()
}

fn duel_stat_rows() -> Vec<(String, String)> {
	vec![
		(String::from("Projectile"), String::from("Bolt")),
		(String::from("Speed"), String::from("180/s")),
		(String::from("Penetration"), String::from("0.85")),
		(String::from("Range"), String::from("36 m")),
		(String::from("Fire"), String::from("Auto")),
		(String::from("DPC"), String::from("25")),
	]
}

fn format_gun_card(title: &str, detail: &str, rows: &[(String, String)]) -> String {
	let mut lines = vec![title.to_ascii_uppercase(), String::from(detail), String::new()];
	for (label, value) in rows {
		lines.push(format!("{label:<13}{value}"));
	}
	lines.join("\n")
}

type WorldBarFills<'w, 's> = Query<
	'w,
	's,
	(&'static mut Node, &'static mut BackgroundColor),
	(With<WorldHudFill>, Without<WorldHealthAnchor>),
>;

pub(crate) fn sync_world_health_bars(
	cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
	npcs: Query<(&GlobalTransform, &Health), With<Npc>>,
	mut anchors: Query<(Entity, &WorldHealthAnchor, &mut Node, &mut Visibility)>,
	children: Query<&Children>,
	mut fills: WorldBarFills,
	mut labels: Query<&mut Text, With<WorldHudLabel>>,
) {
	let Ok((camera, camera_transform)) = cameras.single() else {
		return;
	};
	for (anchor_entity, anchor, mut node, mut visibility) in &mut anchors {
		let Ok((transform, health)) = npcs.get(anchor.target) else {
			*visibility = Visibility::Hidden;
			continue;
		};
		let head = transform.translation() + Vec3::Y * HEAD_LIFT;
		if let Ok(screen) = camera.world_to_viewport(camera_transform, head) {
			node.left = Val::Px(screen.x - WORLD_BAR_WIDTH * 0.5);
			node.top = Val::Px(screen.y - 22.0);
			*visibility = Visibility::Visible;
		} else {
			*visibility = Visibility::Hidden;
			continue;
		}
		let Ok(kids) = children.get(anchor_entity) else {
			continue;
		};
		for child in kids {
			if let Ok(mut text) = labels.get_mut(*child) {
				text.0 = format!("{:.0}", health.current);
			}
			let Ok(grand) = children.get(*child) else {
				continue;
			};
			for grandchild in grand {
				let Ok((mut fill, mut color)) = fills.get_mut(*grandchild) else {
					continue;
				};
				fill.width = Val::Percent(health.fraction() * 100.0);
				color.0 = bar_color(health.fraction());
			}
		}
	}
}

pub(crate) fn ingest_damage_indicators(
	time: Res<Time>,
	mut hits: MessageReader<DamageApplied>,
	players: Query<(), With<Player>>,
	transforms: Query<&GlobalTransform>,
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
		let origin = hit
			.source
			.and_then(|source| transforms.get(source).ok())
			.map(GlobalTransform::translation)
			.unwrap_or(hit.point);
		assign_tick(&mut ticks.0, camera, origin, now);
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

pub(crate) fn ingest_combat_popups(
	time: Res<Time>,
	mut hits: MessageReader<DamageApplied>,
	players: Query<Entity, With<Player>>,
	targets: Query<&GlobalTransform>,
	hud: Query<Entity, With<CombatHudRoot>>,
	mut commands: Commands,
) {
	let Ok(player) = players.single() else {
		return;
	};
	let Ok(hud) = hud.single() else {
		return;
	};
	let now = time.elapsed_secs();
	for hit in hits.read() {
		if hit.source != Some(player) {
			continue;
		}
		let points = targets
			.get(hit.target)
			.ok()
			.filter(|transform| is_headshot(transform, hit.point))
			.map_or(HIT_POINTS, |_| HEAD_POINTS);
		spawn_combat_popup(&mut commands, hud, hit.point, now, points);
		if hit.remaining <= 0.0 {
			spawn_combat_popup(&mut commands, hud, hit.point + Vec3::Y * 0.16, now, DOWN_POINTS);
		}
	}
}

fn spawn_combat_popup(commands: &mut Commands, hud: Entity, world: Vec3, born: f32, points: u8) {
	let (fill, size) = popup_style(points);
	commands.entity(hud).with_children(|root| {
		root.spawn((
			Name::new("combat-popup"),
			CombatPopup { world, born, points },
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				..default()
			},
			Text::new(popup_label(points)),
			TextFont { font_size: FontSize::Px(size), ..default() },
			TextColor(fill),
			Pickable::IGNORE,
		));
	});
}

pub(crate) fn update_combat_popups(
	time: Res<Time>,
	cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
	mut popups: Query<(Entity, &CombatPopup, &mut Node, &mut TextColor, &mut Visibility)>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	let Ok((camera, camera_transform)) = cameras.single() else {
		for (_, _, _, _, mut visibility) in &mut popups {
			*visibility = Visibility::Hidden;
		}
		return;
	};
	for (entity, popup, mut node, mut color, mut visibility) in &mut popups {
		let age = now - popup.born;
		if age >= POPUP_LIFE {
			commands.entity(entity).try_despawn();
			continue;
		}
		let Ok(screen) = camera.world_to_viewport(camera_transform, popup.world) else {
			*visibility = Visibility::Hidden;
			continue;
		};
		let rise = age / POPUP_LIFE * POPUP_RISE;
		node.left = Val::Px(screen.x - 16.0);
		node.top = Val::Px(screen.y - 18.0 - rise);
		let alpha = popup_alpha(age);
		color.0 = popup_style(popup.points).0.with_alpha(alpha);
		*visibility = Visibility::Visible;
	}
}

fn popup_label(points: u8) -> String {
	format!("+{points}")
}

fn popup_style(points: u8) -> (Color, f32) {
	if points >= DOWN_POINTS {
		(Color::srgb(1.0, 0.82, 0.28), 28.0)
	} else if points >= HEAD_POINTS {
		(Color::srgb(0.42, 0.86, 1.0), 24.0)
	} else {
		(Color::srgb(0.92, 0.98, 1.0), 22.0)
	}
}

/// Upper spherical cap of the character capsule.
fn is_headshot(target: &GlobalTransform, point: Vec3) -> bool {
	let local = target.affine().inverse().transform_point3(point);
	local.y >= CAPSULE_LENGTH * 0.5
}

fn popup_alpha(age: f32) -> f32 {
	(1.0 - age / POPUP_LIFE).clamp(0.0, 1.0)
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

	#[test]
	fn duel_card_lists_the_default_bolt() {
		let text = gun_card_text(None);
		assert!(text.starts_with("BULLPUP"), "{text}");
		assert!(text.contains("Bolt · 25 DPC"), "{text}");
		assert!(text.contains("180/s"), "{text}");
		assert!(text.contains("36 m"), "{text}");
	}

	#[test]
	fn hit_is_one_head_is_two_and_down_is_five() {
		assert_eq!(HIT_POINTS, 1);
		assert_eq!(HEAD_POINTS, 2);
		assert_eq!(DOWN_POINTS, 5);
		assert_eq!(popup_label(HIT_POINTS), "+1");
		assert_eq!(popup_label(HEAD_POINTS), "+2");
		assert_eq!(popup_label(DOWN_POINTS), "+5");
		let head = popup_style(HEAD_POINTS).0.to_srgba();
		assert!(head.blue > head.red && head.blue > head.green * 0.9);
	}

	#[test]
	fn upper_hemisphere_is_a_headshot() {
		let target = GlobalTransform::from_translation(Vec3::new(4.0, 1.0, -2.0));
		assert!(is_headshot(&target, Vec3::new(4.0, 1.0 + CAPSULE_LENGTH * 0.5 + 0.2, -2.0)));
		assert!(!is_headshot(&target, Vec3::new(4.0, 1.0, -2.0)));
		assert!(!is_headshot(&target, Vec3::new(4.0, 1.0 - CAPSULE_LENGTH * 0.5, -2.0)));
	}
}
