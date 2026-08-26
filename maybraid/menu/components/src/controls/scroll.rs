//! Visible scrollbar for overflowing HUD panes.

use bevy::ecs::event::EntityEvent;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;

use crate::theme::{SCROLLBAR_THUMB, SCROLLBAR_TRACK, SCROLLBAR_WIDTH};

const SCROLL_LINE_PX: f32 = 14.0;
const MIN_THUMB_PX: f32 = 24.0;

/// Scrollable column that a [`HudScrollThumb`] tracks.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct HudScrollViewport;

/// Vertical track; hidden when content fits.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct HudScrollTrack;

/// Thumb whose height and offset follow the viewport.
#[derive(Component, Debug, Clone, Copy)]
pub struct HudScrollThumb {
	pub viewport: Entity,
}

#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct HudScroll {
	pub entity: Entity,
	pub delta: Vec2,
}

/// Row: growing scroll viewport plus a thin track.
pub fn spawn_scroll_pane(
	parent: &mut ChildSpawnerCommands,
	viewport_extra: impl Bundle,
	align: AlignItems,
	row_gap: f32,
) -> Entity {
	let mut viewport = Entity::PLACEHOLDER;
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(8.0),
				min_height: Val::Px(0.0),
				flex_grow: 1.0,
				flex_shrink: 1.0,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			viewport = row
				.spawn((
					HudScrollViewport,
					viewport_extra,
					Node {
						width: Val::Percent(100.0),
						flex_grow: 1.0,
						flex_shrink: 1.0,
						min_height: Val::Px(0.0),
						min_width: Val::Px(0.0),
						flex_direction: FlexDirection::Column,
						align_items: align,
						row_gap: Val::Px(row_gap),
						overflow: Overflow::scroll_y(),
						scrollbar_width: SCROLLBAR_WIDTH,
						..default()
					},
					ScrollPosition::default(),
					Pickable::default(),
				))
				.id();
			row.spawn((
				HudScrollTrack,
				Node {
					width: Val::Px(SCROLLBAR_WIDTH),
					height: Val::Percent(100.0),
					flex_shrink: 0.0,
					position_type: PositionType::Relative,
					..default()
				},
				BackgroundColor(SCROLLBAR_TRACK),
				Visibility::Hidden,
				Pickable::IGNORE,
			))
			.with_children(|track| {
				track.spawn((
					HudScrollThumb { viewport },
					Node {
						position_type: PositionType::Absolute,
						left: Val::Px(0.0),
						width: Val::Px(SCROLLBAR_WIDTH),
						height: Val::Px(MIN_THUMB_PX),
						top: Val::Px(0.0),
						..default()
					},
					BackgroundColor(SCROLLBAR_THUMB),
					Pickable::IGNORE,
				));
			});
		});
	viewport
}

/// Show the track only when the viewport overflows; size the thumb to the visible ratio.
pub fn sync_hud_scrollbars(
	viewports: Query<(&ScrollPosition, &ComputedNode), With<HudScrollViewport>>,
	mut tracks: Query<(&mut Visibility, &ComputedNode, &Children), With<HudScrollTrack>>,
	mut thumbs: Query<(&HudScrollThumb, &mut Node), Without<HudScrollTrack>>,
) {
	for (mut track_visibility, track_computed, children) in &mut tracks {
		let Some((thumb_entity, thumb)) = children
			.iter()
			.find_map(|child| thumbs.get(child).ok().map(|thumb| (child, thumb.0)))
		else {
			continue;
		};
		let Ok((scroll, viewport)) = viewports.get(thumb.viewport) else {
			continue;
		};
		let viewport_h = viewport.size().y * viewport.inverse_scale_factor();
		let content_h = viewport.content_size().y * viewport.inverse_scale_factor();
		let track_h = track_computed.size().y * track_computed.inverse_scale_factor();
		if content_h <= viewport_h + 1.0 || track_h <= 0.0 {
			*track_visibility = Visibility::Hidden;
			continue;
		}
		*track_visibility = Visibility::Inherited;
		let ratio = (viewport_h / content_h).clamp(0.08, 1.0);
		let thumb_h = (track_h * ratio).max(MIN_THUMB_PX).min(track_h);
		let max_scroll = (content_h - viewport_h).max(0.0);
		let max_top = (track_h - thumb_h).max(0.0);
		let top = if max_scroll <= 0.0 { 0.0 } else { (scroll.y / max_scroll) * max_top };
		if let Ok((_, mut thumb_node)) = thumbs.get_mut(thumb_entity) {
			thumb_node.height = Val::Px(thumb_h);
			thumb_node.top = Val::Px(top);
		}
	}
}

pub fn send_hud_scroll_events(
	mut mouse_wheel_reader: MessageReader<MouseWheel>,
	hover_map: Res<HoverMap>,
	mut commands: Commands,
) {
	for mouse_wheel in mouse_wheel_reader.read() {
		let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);
		if mouse_wheel.unit == MouseScrollUnit::Line {
			delta *= SCROLL_LINE_PX;
		}
		for pointer_map in hover_map.values() {
			for entity in pointer_map.keys().copied() {
				commands.trigger(HudScroll { entity, delta });
			}
		}
	}
}

pub fn on_hud_scroll(
	mut scroll: On<HudScroll>,
	mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode), With<HudScrollViewport>>,
) {
	let Ok((mut scroll_position, node, computed)) = query.get_mut(scroll.entity) else {
		return;
	};
	let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();
	let delta = &mut scroll.delta;
	if node.overflow.x == OverflowAxis::Scroll && delta.x != 0. {
		let max =
			if delta.x > 0. { scroll_position.x >= max_offset.x } else { scroll_position.x <= 0. };
		if !max {
			scroll_position.x += delta.x;
			delta.x = 0.;
		}
	}
	if node.overflow.y == OverflowAxis::Scroll && delta.y != 0. {
		let max =
			if delta.y > 0. { scroll_position.y >= max_offset.y } else { scroll_position.y <= 0. };
		if !max {
			scroll_position.y += delta.y;
			delta.y = 0.;
		}
	}
	if *delta == Vec2::ZERO {
		scroll.propagate(false);
	}
}
