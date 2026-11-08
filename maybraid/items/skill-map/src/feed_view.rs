//! Rockadder mosaic lanes and directional notes for timed skill maps.

use bevy::prelude::*;
use menu_components::HudFonts;

use crate::feed::{FeedDirection, FeedState, FeedStepStatus, FEED_STEPS};
use crate::map::{render_layer, AuthoredMap};
use crate::tile_material::SkillMapTileAssets;
use crate::user::{SkillMapHeld, SkillMapMember};
use crate::viewport::map_view_extent;

const HIT_Y: f32 = -20.0;
const TOP_Y: f32 = 22.0;
const SCROLL_SPEED: f32 = 24.0;
const GOLD: Color = Color::srgb(0.98, 0.73, 0.27);
const HIT: Color = Color::srgb(0.38, 0.96, 0.65);
const MISS: Color = Color::srgb(0.98, 0.32, 0.20);

#[derive(Component)]
pub(crate) struct FeedPrompt {
	index: usize,
	origin: Vec2,
}

#[derive(Component)]
pub(crate) struct FeedStatus;

impl FeedDirection {
	fn lane_x(self) -> f32 {
		let direction = self.vector();
		if direction.x < 0.0 {
			-27.0
		} else if direction.y > 0.0 {
			-9.0
		} else if direction.y < 0.0 {
			9.0
		} else {
			27.0
		}
	}

	fn rotation(self) -> Quat {
		let direction = self.vector();
		Quat::from_rotation_z((-direction.x).atan2(direction.y))
	}
}

impl FeedPrompt {
	fn presentation(&self, state: &FeedState) -> (f32, bool, Color) {
		let due = FeedState::step_time(self.index);
		let status = if state.in_pause() {
			if self.index == 0 && state.next_first_hit() {
				FeedStepStatus::Hit
			} else {
				FeedStepStatus::Pending
			}
		} else {
			state.step_status(self.index)
		};
		// Let the next phrase approach during the interlude, giving its first
		// prompt the same lead-in as every other note.
		let remaining = if state.in_pause() {
			FeedState::cycle_duration() - state.elapsed() + due
		} else {
			due - state.elapsed()
		};
		let y = HIT_Y + remaining * SCROLL_SPEED;
		if status == FeedStepStatus::Hit {
			return (HIT_Y, remaining >= -0.13, HIT);
		}
		let color = if status == FeedStepStatus::Missed { MISS } else { GOLD };
		(y, (HIT_Y - 10.0..=TOP_Y).contains(&y), color)
	}
}

/// Shared scene for live play and catalog thumbnails. No collidable map tiles
/// or live session state are created here; previews remain inert.
pub(crate) fn spawn_feed_scene(
	commands: &mut Commands,
	spec: AuthoredMap,
	member: Option<SkillMapMember>,
	assets: &SkillMapTileAssets,
	origin: Vec2,
	parent: Option<Entity>,
) {
	let layer = render_layer(spec.id);
	let mut scene = commands.spawn((
		Name::new("skill-map-feed-lanes"),
		Transform::from_translation(origin.extend(0.0)),
		Visibility::Inherited,
	));
	if let Some(member) = member {
		scene.insert(member);
	}
	let scene_id = scene.id();
	scene.with_children(|board| {
		board.spawn((
			Mesh2d(assets.mesh.clone()),
			MeshMaterial2d(assets.land_rock.clone()),
			Transform::from_scale((map_view_extent() / 16.0).extend(1.0)),
			layer.clone(),
		));
		for direction in
			[FeedDirection::Left, FeedDirection::Up, FeedDirection::Down, FeedDirection::Right]
		{
			let x = direction.lane_x();
			board.spawn((
				Sprite::from_color(Color::srgba(0.04, 0.025, 0.015, 0.80), Vec2::new(16.0, 57.0)),
				Transform::from_xyz(x, -2.0, 0.1),
				layer.clone(),
			));
			board.spawn((
				Sprite::from_color(Color::srgba(0.93, 0.65, 0.25, 0.24), Vec2::new(0.4, 57.0)),
				Transform::from_xyz(x - 8.5, -2.0, 0.15),
				layer.clone(),
			));
			board
				.spawn((
					Sprite::from_color(Color::srgb(0.48, 0.31, 0.12), Vec2::splat(12.0)),
					Transform::from_xyz(x, HIT_Y, 0.2),
					layer.clone(),
				))
				.with_children(|receptor| {
					receptor.spawn((
						Sprite::from_color(Color::srgb(0.10, 0.065, 0.035), Vec2::splat(10.5)),
						Transform::from_xyz(0.0, 0.0, 0.01),
						layer.clone(),
					));
					spawn_arrow(receptor, direction, Color::srgb(0.57, 0.43, 0.24), &layer);
				});
		}
		board.spawn((
			Sprite::from_color(Color::srgba(1.0, 0.78, 0.36, 0.45), Vec2::new(72.0, 0.6)),
			Transform::from_xyz(0.0, HIT_Y, 0.3),
			layer.clone(),
		));
	});
	if let Some(parent) = parent {
		commands.entity(parent).add_child(scene_id);
	}

	let idle = FeedState::default();
	for (index, direction) in FEED_STEPS.iter().copied().enumerate() {
		let prompt = FeedPrompt { index, origin };
		let (y, visible, color) = prompt.presentation(&idle);
		let mut note = commands.spawn((
			Name::new(format!("skill-map-feed-step-{}", index + 1)),
			prompt,
			Sprite::from_color(color, Vec2::splat(12.0)),
			Transform::from_xyz(origin.x + direction.lane_x(), origin.y + y, 0.5),
			if visible { Visibility::Inherited } else { Visibility::Hidden },
			layer.clone(),
		));
		if let Some(member) = member {
			note.insert(member);
		}
		let note_id = note.id();
		note.with_children(|face| {
			face.spawn((
				Mesh2d(assets.mesh.clone()),
				MeshMaterial2d(assets.rockadder.clone()),
				Transform::from_xyz(0.0, 0.0, 0.01).with_scale(Vec3::new(0.64, 0.64, 1.0)),
				layer.clone(),
			));
			// Keep the directional cue legible over Rockadder's bright moving facets.
			face.spawn((
				Sprite::from_color(Color::srgb(0.075, 0.045, 0.02), Vec2::splat(8.8)),
				Transform::from_xyz(0.0, 0.0, 0.02),
				layer.clone(),
			));
			spawn_arrow(face, direction, Color::srgb(1.0, 0.96, 0.78), &layer);
		});
		if let Some(parent) = parent {
			commands.entity(parent).add_child(note_id);
		}
	}
}

fn spawn_arrow(
	parent: &mut ChildSpawnerCommands,
	direction: FeedDirection,
	color: Color,
	layer: &bevy::camera::visibility::RenderLayers,
) {
	parent
		.spawn((
			Transform::from_xyz(0.0, 0.0, 0.03).with_rotation(direction.rotation()),
			Visibility::Inherited,
		))
		.with_children(|arrow| {
			for (position, size, angle) in [
				(Vec2::new(0.0, -0.5), Vec2::new(1.4, 6.0), 0.0),
				(Vec2::new(-1.35, 1.5), Vec2::new(1.4, 4.5), -std::f32::consts::FRAC_PI_4),
				(Vec2::new(1.35, 1.5), Vec2::new(1.4, 4.5), std::f32::consts::FRAC_PI_4),
			] {
				arrow.spawn((
					Sprite::from_color(color, size),
					Transform::from_translation(position.extend(0.0))
						.with_rotation(Quat::from_rotation_z(angle)),
					layer.clone(),
				));
			}
		});
}

pub(crate) fn spawn_feed_status(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	member: SkillMapMember,
) {
	parent.spawn((
		FeedStatus,
		member,
		Text::new("Flick LEFT to start"),
		fonts.item(12.0),
		TextColor(GOLD),
		TextLayout::justify(Justify::Center),
		Node {
			position_type: PositionType::Absolute,
			top: Val::Px(39.0),
			left: Val::Px(6.0),
			right: Val::Px(6.0),
			..default()
		},
		Pickable::IGNORE,
	));
}

pub(crate) fn sync_feed_view(
	states: Query<&FeedState>,
	users: Query<&SkillMapHeld>,
	mut prompts: Query<(
		&FeedPrompt,
		&SkillMapMember,
		&mut Transform,
		&mut Visibility,
		&mut Sprite,
	)>,
	mut labels: Query<(&SkillMapMember, &mut Text, &mut TextColor), With<FeedStatus>>,
) {
	for (prompt, member, mut transform, mut visibility, mut sprite) in &mut prompts {
		let Ok(state) = states.get(member.session) else {
			continue;
		};
		let (y, visible, color) = prompt.presentation(state);
		transform.translation.y = prompt.origin.y + y;
		*visibility = if visible { Visibility::Inherited } else { Visibility::Hidden };
		sprite.color = color;
	}
	for (member, mut text, mut color) in &mut labels {
		let Ok(state) = states.get(member.session) else {
			continue;
		};
		let held = users.get(member.user).is_ok_and(|held| held.0);
		let (label, tint) = if !state.started() {
			("Flick LEFT to start".to_owned(), GOLD)
		} else if !held {
			(format!("Paused · {} / 8", state.hits()), GOLD)
		} else if state.in_pause() {
			(format!("{} / 8 · Next phrase…", state.hits()), GOLD)
		} else {
			let last = (0..FEED_STEPS.len())
				.rev()
				.find(|&index| state.step_status(index) != FeedStepStatus::Pending);
			let missed =
				last.is_some_and(|index| state.step_status(index) == FeedStepStatus::Missed);
			(
				format!("{} / 8 · {}", state.hits(), if missed { "Miss" } else { "Hit" }),
				if missed { MISS } else { HIT },
			)
		};
		if text.0 != label {
			text.0 = label;
		}
		color.0 = tint;
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use character_items::{SkillMapKind, SkillMapSpec};

	use crate::feed::{FeedState, FEED_WINDOW_SECS};
	use crate::feed_view::{spawn_feed_scene, FeedPrompt, HIT, HIT_Y};
	use crate::map::authored_map_from_spec;
	use crate::tile_material::SkillMapTileAssets;
	use crate::tiles::SkillMapTile;

	#[test]
	fn early_hit_flashes_at_receptor_including_repeat() {
		let mut feed = FeedState::default();
		assert!(feed.strike(Vec2::NEG_X));
		feed.advance(FeedState::step_time(1) - FEED_WINDOW_SECS * 0.5);
		assert!(feed.strike(Vec2::Y));
		let up = FeedPrompt { index: 1, origin: Vec2::ZERO };
		assert_eq!(up.presentation(&feed), (HIT_Y, true, HIT));
		feed.advance(FeedState::cycle_duration() - feed.elapsed() - FEED_WINDOW_SECS * 0.5);
		assert!(feed.strike(Vec2::NEG_X));
		let left = FeedPrompt { index: 0, origin: Vec2::ZERO };
		assert_eq!(left.presentation(&feed), (HIT_Y, true, HIT));
	}

	#[test]
	fn preview_is_inert_and_its_entire_scene_follows_parent_cleanup() {
		let mut world = World::new();
		let host = world.spawn_empty().id();
		let assets = SkillMapTileAssets {
			mesh: default(),
			cursor_mesh: default(),
			bead_mesh: default(),
			land_fire: default(),
			land_wave: default(),
			land_rock: default(),
			land_cosmo: default(),
			water: default(),
			fireball: default(),
			dumbwave: default(),
			rockadder: default(),
			cosimo: default(),
			cursor: default(),
		};
		let spec = authored_map_from_spec(SkillMapSpec::new(SkillMapKind::RockadderFeed, 13));
		spawn_feed_scene(&mut world.commands(), spec, None, &assets, Vec2::ZERO, Some(host));
		world.flush();
		assert_eq!(world.query::<&FeedPrompt>().iter(&world).count(), 8);
		assert_eq!(world.query::<&SkillMapTile>().iter(&world).count(), 0);
		assert_eq!(world.query::<&FeedState>().iter(&world).count(), 0);
		world.despawn(host);
		assert!(!world.entities().contains(host));
		assert_eq!(world.query::<&FeedPrompt>().iter(&world).count(), 0);
		assert_eq!(world.query::<&Transform>().iter(&world).count(), 0);
		assert_eq!(world.query::<&Sprite>().iter(&world).count(), 0);
		assert_eq!(world.query::<&Mesh2d>().iter(&world).count(), 0);
	}
}
