//! Rockadder mosaic lanes and directional notes for timed skill maps.

use bevy::prelude::*;
use menu_components::{HudFonts, TANZ_CALLOUT_ICON_PX};

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
/// Overlay height used to translate [`TANZ_CALLOUT_ICON_PX`] into map units.
const OVERLAY_REFERENCE_HEIGHT_PX: f32 = 1080.0;
/// Authored chevron height (stem + barbs) before [`FeedArrow::scale`].
const FEED_ARROW_AUTHOR_HEIGHT: f32 = 7.25;

#[derive(Component)]
pub(crate) struct FeedPrompt {
	index: usize,
	origin: Vec2,
}

#[derive(Component)]
pub(crate) struct FeedStatus;

#[derive(Component)]
pub(crate) struct FeedNoteTint;

/// Direction chevron shared by live notes and idle receptors.
pub(crate) struct FeedArrow;

impl FeedArrow {
	const PIECES: [(Vec2, Vec2, f32); 3] = [
		(Vec2::new(0.0, -0.5), Vec2::new(1.4, 6.0), 0.0),
		(Vec2::new(-1.35, 1.5), Vec2::new(1.4, 4.5), -std::f32::consts::FRAC_PI_4),
		(Vec2::new(1.35, 1.5), Vec2::new(1.4, 4.5), std::f32::consts::FRAC_PI_4),
	];

	/// Map-space height matching the Tanz callout icon on a 1080p overlay.
	pub fn height() -> f32 {
		TANZ_CALLOUT_ICON_PX / OVERLAY_REFERENCE_HEIGHT_PX * map_view_extent().y
	}

	pub fn scale() -> f32 {
		Self::height() / FEED_ARROW_AUTHOR_HEIGHT
	}

	pub fn spawn_note(
		parent: &mut ChildSpawnerCommands,
		color: Color,
		layer: &bevy::camera::visibility::RenderLayers,
	) {
		Self::spawn(parent, color, layer, true);
	}

	pub fn spawn_receptor(
		parent: &mut ChildSpawnerCommands,
		color: Color,
		layer: &bevy::camera::visibility::RenderLayers,
	) {
		Self::spawn(parent, color, layer, false);
	}

	fn spawn(
		parent: &mut ChildSpawnerCommands,
		color: Color,
		layer: &bevy::camera::visibility::RenderLayers,
		note: bool,
	) {
		for (position, size, angle) in Self::PIECES {
			let mut glyph = parent.spawn((
				Sprite::from_color(color, size),
				Transform::from_translation(position.extend(0.0))
					.with_rotation(Quat::from_rotation_z(angle)),
				layer.clone(),
			));
			if note {
				glyph.insert(FeedNoteTint);
			}
		}
	}
}

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
			board
				.spawn((
					Transform::from_xyz(direction.lane_x(), HIT_Y, 0.2)
						.with_rotation(direction.rotation())
						.with_scale(Vec3::splat(FeedArrow::scale())),
					Visibility::Inherited,
					layer.clone(),
				))
				.with_children(|receptor| {
					FeedArrow::spawn_receptor(receptor, Color::srgb(0.57, 0.43, 0.24), &layer);
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
			Transform::from_xyz(origin.x + direction.lane_x(), origin.y + y, 0.5)
				.with_rotation(direction.rotation())
				.with_scale(Vec3::splat(FeedArrow::scale())),
			if visible { Visibility::Inherited } else { Visibility::Hidden },
			layer.clone(),
		));
		if let Some(member) = member {
			note.insert(member);
		}
		let note_id = note.id();
		note.with_children(|face| {
			FeedArrow::spawn_note(face, color, &layer);
		});
		if let Some(parent) = parent {
			commands.entity(parent).add_child(note_id);
		}
	}
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
	mut prompts: Query<(&FeedPrompt, &SkillMapMember, &mut Transform, &mut Visibility, &Children)>,
	mut glyphs: Query<&mut Sprite, With<FeedNoteTint>>,
	mut labels: Query<(&SkillMapMember, &mut Text, &mut TextColor), With<FeedStatus>>,
) {
	for (prompt, member, mut transform, mut visibility, children) in &mut prompts {
		let Ok(state) = states.get(member.session) else {
			continue;
		};
		let (y, visible, color) = prompt.presentation(state);
		transform.translation.y = prompt.origin.y + y;
		*visibility = if visible { Visibility::Inherited } else { Visibility::Hidden };
		for child in children {
			if let Ok(mut sprite) = glyphs.get_mut(*child) {
				sprite.color = color;
			}
		}
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
	use crate::feed_view::{spawn_feed_scene, FeedArrow, FeedPrompt, HIT, HIT_Y};
	use crate::map::authored_map_from_spec;
	use crate::tile_material::SkillMapTileAssets;
	use crate::tiles::SkillMapTile;
	use crate::viewport::map_view_extent;
	use menu_components::TANZ_CALLOUT_ICON_PX;

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
		assert_eq!(
			world.query::<&Mesh2d>().iter(&world).count(),
			1,
			"rhythm maps keep the mosaic ground, not per-note tiles"
		);
		let mut slot = false;
		let mut sprites = 0;
		for sprite in world.query::<&Sprite>().iter(&world) {
			sprites += 1;
			slot |= matches!(
				sprite.custom_size,
				Some(size) if size == Vec2::new(16.0, 57.0)
					|| size == Vec2::splat(12.0)
					|| size == Vec2::splat(10.5)
					|| size == Vec2::splat(8.8)
			);
		}
		assert!(!slot, "slot panels must not spawn behind rhythm arrows");
		assert_eq!(sprites, 37, "hit line + 4 receptor chevrons + 8 notes");
		world.despawn(host);
		assert!(!world.entities().contains(host));
		assert_eq!(world.query::<&FeedPrompt>().iter(&world).count(), 0);
		assert_eq!(world.query::<&Transform>().iter(&world).count(), 0);
		assert_eq!(world.query::<&Sprite>().iter(&world).count(), 0);
		assert_eq!(world.query::<&Mesh2d>().iter(&world).count(), 0);
	}

	#[test]
	fn feed_arrows_match_the_tanz_callout_icon() {
		let expected = TANZ_CALLOUT_ICON_PX / super::OVERLAY_REFERENCE_HEIGHT_PX
			* map_view_extent().y
			/ super::FEED_ARROW_AUTHOR_HEIGHT;
		assert!((FeedArrow::scale() - expected).abs() < 1e-4);
		assert!(
			FeedArrow::scale() < 1.0,
			"overlay arrows must be smaller than the authored chevron"
		);
		assert_eq!(TANZ_CALLOUT_ICON_PX, 40.0);
		assert!((FeedArrow::height() - 40.0 / 1080.0 * map_view_extent().y).abs() < 1e-4);
	}
}
