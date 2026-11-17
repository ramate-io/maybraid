//! Shared rotary-wheel geometry used by inventory selection and the gallery.
//!
//! Motion matches the inventory carousel from [#1567](https://github.com/ramate-io/maybraid/pull/1567):
//! a flattened ring with no track disc. Gallery cards reuse the same easing
//! and caption treatment; they only swap the art slot for a standing portrait.

use bevy::prelude::*;

use super::hud_menu::HudMenu;
use crate::single_select::text_menu::TextMenu;
use crate::theme::{TEXT_DEEP_BLUE, TEXT_YELLOW, TEXT_YELLOW_FAINT};

/// Ring size and easing. Inventory and the gallery share this flattened carousel.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct RotaryLayout {
	pub radius: f32,
	pub front_offset: f32,
	pub min_scale: f32,
	pub scale_span: f32,
	pub ease: f32,
	pub card_width: f32,
	pub card_height: f32,
}

impl RotaryLayout {
	/// Inventory drawer carousel (flattened ring in the pick pane).
	pub const INVENTORY: Self = Self {
		radius: 228.0,
		front_offset: 14.0,
		min_scale: 0.62,
		scale_span: 0.19,
		ease: 14.0,
		card_width: 164.0,
		card_height: 110.0,
	};

	/// Name size on a rotary caption chip.
	pub const LABEL_PX: f32 = 20.0;

	/// Species / detail size under a rotary name.
	pub const SUBTEXT_PX: f32 = 13.0;

	/// Same inventory motion, with a standing portrait slot instead of a 56px icon.
	pub fn with_standing_portraits(self) -> Self {
		Self { card_width: 220.0, card_height: 360.0, radius: 340.0, front_offset: 16.0, ..self }
	}

	/// Portrait cards with fewer than three entries would otherwise stack.
	pub fn ring_count(self, count: usize) -> usize {
		if count == 0 {
			0
		} else if self.card_height > Self::INVENTORY.card_height {
			count.max(3)
		} else {
			count
		}
	}

	pub fn angle(self, ordinal: usize, selected: usize, count: usize) -> f32 {
		let count = self.ring_count(count);
		if count < 2 {
			return 0.0;
		}
		(ordinal as f32 - selected as f32) * std::f32::consts::TAU / count as f32
	}

	pub fn x(self, angle: f32) -> f32 {
		angle.sin() * self.radius
	}

	pub fn y(self, angle: f32) -> f32 {
		(1.0 + angle.cos()) * self.front_offset / 2.0
	}

	pub fn scale(self, angle: f32) -> f32 {
		self.min_scale + (angle.cos() + 1.0) * self.scale_span
	}

	pub fn scale_for(self, ordinal: usize, selected: usize, count: usize) -> f32 {
		self.scale(self.angle(ordinal, selected, count))
	}

	pub fn z_index(self, angle: f32) -> ZIndex {
		ZIndex((100.0 + angle.cos() * 50.0) as i32)
	}

	pub fn ease_alpha(self, dt: f32) -> f32 {
		(1.0 - (-self.ease * dt).exp()).clamp(0.0, 1.0)
	}

	pub fn card_node(self) -> Node {
		Node {
			position_type: PositionType::Absolute,
			left: Val::Percent(50.0),
			top: Val::Percent(50.0),
			width: Val::Px(self.card_width),
			height: Val::Px(self.card_height),
			margin: UiRect {
				left: Val::Px(-self.card_width / 2.0),
				top: Val::Px(-self.card_height / 2.0),
				..default()
			},
			..default()
		}
	}

	pub fn card_transform(self, angle: f32, scale: f32) -> UiTransform {
		UiTransform {
			translation: Val2::px(self.x(angle), self.y(angle)),
			scale: Vec2::splat(scale),
			..default()
		}
	}

	pub fn apply_transform(
		self,
		transform: &mut UiTransform,
		z_index: &mut ZIndex,
		angle: f32,
		scale: f32,
	) {
		*transform = self.card_transform(angle, scale);
		*z_index = self.z_index(angle);
	}

	pub fn host_node(self) -> Node {
		Node {
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			min_height: Val::Px(96.0),
			position_type: PositionType::Relative,
			overflow: Overflow::clip(),
			..default()
		}
	}

	/// Full-screen host that sits over a menu wash.
	pub fn overlay_node(self) -> Node {
		Node {
			position_type: PositionType::Absolute,
			left: Val::Px(0.0),
			top: Val::Px(0.0),
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			overflow: Overflow::clip(),
			..default()
		}
	}
}

/// One card on a [`RotaryLayout`] ring. `menu` is the layout/focus root.
#[derive(Component, Clone, Copy, Debug)]
pub struct RotaryCard {
	pub menu: Entity,
	pub ordinal: usize,
	pub count: usize,
}

impl RotaryCard {
	/// Fill previously used by the flattened rotary track ellipse.
	pub const REMOVED_TRACK_FILL: Color = Color::srgba(0.35, 0.67, 0.88, 0.72);

	pub fn is_removed_track_fill(color: Color) -> bool {
		color == Self::REMOVED_TRACK_FILL
	}

	pub fn is_track_transform(transform: &UiTransform) -> bool {
		transform.scale.y > 0.0 && transform.scale.y < 0.1
	}

	pub fn unresolved(ordinal: usize, count: usize) -> Self {
		Self { menu: Entity::PLACEHOLDER, ordinal, count }
	}

	/// Bind to the nearest ancestor that owns [`RotaryLayout`].
	pub fn bind_menu(
		&mut self,
		entity: Entity,
		child_of: &Query<&ChildOf>,
		layouts: &Query<&RotaryLayout>,
	) -> Option<Entity> {
		if self.menu != Entity::PLACEHOLDER && layouts.contains(self.menu) {
			return Some(self.menu);
		}
		let menu = std::iter::once(entity)
			.chain(child_of.iter_ancestors(entity))
			.find(|ancestor| layouts.contains(*ancestor))?;
		self.menu = menu;
		Some(menu)
	}

	pub fn selected_index(&self, hud: &Query<&HudMenu>, text: &Query<&TextMenu>) -> Option<usize> {
		if self.count == 0 {
			return None;
		}
		let selected = hud
			.get(self.menu)
			.map(|menu| menu.selected)
			.or_else(|_| text.get(self.menu).map(|menu| menu.selected))
			.ok()?;
		Some(selected.min(self.count - 1))
	}
}

/// Animated angle / scale for a [`RotaryCard`].
#[derive(Component, Clone, Copy, Debug)]
pub struct RotaryMotion {
	pub angle: f32,
	pub scale: f32,
}

impl RotaryMotion {
	pub fn at(layout: RotaryLayout, ordinal: usize, selected: usize, count: usize) -> Self {
		let angle = layout.angle(ordinal, selected, count);
		Self { angle, scale: layout.scale(angle) }
	}

	pub fn step_angle(&mut self, target: f32, alpha: f32) {
		let delta = (target - self.angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
			- std::f32::consts::PI;
		self.angle += delta * alpha;
	}

	pub fn step_scale(&mut self, target: f32, alpha: f32) {
		self.scale += (target - self.scale) * alpha;
	}

	pub fn ease_to(
		&mut self,
		layout: RotaryLayout,
		ordinal: usize,
		selected: usize,
		count: usize,
		dt: f32,
	) {
		let alpha = layout.ease_alpha(dt);
		self.step_angle(layout.angle(ordinal, selected, count), alpha);
		self.step_scale(layout.scale_for(ordinal, selected, count), alpha);
	}
}

/// Art slot above a rotary caption. Hosts attach a portrait or item thumbnail.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RotaryPortrait {
	pub ordinal: usize,
}

/// Caption chip whose fill follows the focused ordinal.
#[derive(Component, Clone, Copy, Debug)]
pub struct RotaryCaption {
	pub menu: Entity,
	pub ordinal: usize,
}

impl RotaryCaption {
	pub fn bind_menu(
		&mut self,
		entity: Entity,
		child_of: &Query<&ChildOf>,
		layouts: &Query<&RotaryLayout>,
	) -> Option<Entity> {
		if self.menu != Entity::PLACEHOLDER && layouts.contains(self.menu) {
			return Some(self.menu);
		}
		let menu = std::iter::once(entity)
			.chain(child_of.iter_ancestors(entity))
			.find(|ancestor| layouts.contains(*ancestor))?;
		self.menu = menu;
		Some(menu)
	}

	pub fn is_selected(&self, hud: &Query<&HudMenu>, text: &Query<&TextMenu>) -> bool {
		hud.get(self.menu)
			.map(|menu| menu.selected == self.ordinal)
			.or_else(|_| text.get(self.menu).map(|menu| menu.selected == self.ordinal))
			.unwrap_or(false)
	}

	pub fn paint(self, selected: bool) -> (Color, Color) {
		if selected {
			(TEXT_YELLOW, TEXT_DEEP_BLUE)
		} else {
			(Color::NONE, TEXT_YELLOW_FAINT)
		}
	}
}

/// Nested column that holds caption text under a [`RotaryCaption`].
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RotaryCaptionGroup;

/// Caption text whose color follows [`RotaryCaption::paint`].
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RotaryCaptionText;

/// Ease rotary cards to their wrapped positions. No nodes are rebuilt.
pub fn animate_rotary_cards(
	time: Res<Time>,
	child_of: Query<&ChildOf>,
	layouts: Query<&RotaryLayout>,
	hud: Query<&HudMenu>,
	text: Query<&TextMenu>,
	mut cards: Query<(Entity, &mut RotaryCard, &mut RotaryMotion, &mut UiTransform, &mut ZIndex)>,
	mut labels: Query<(Entity, &mut RotaryCaption, &Children, &mut BackgroundColor)>,
	caption_groups: Query<&Children, With<RotaryCaptionGroup>>,
	mut captions: Query<&mut TextColor, With<RotaryCaptionText>>,
) {
	let dt = time.delta_secs();
	for (entity, mut card, mut motion, mut transform, mut z_index) in &mut cards {
		let Some(menu) = card.bind_menu(entity, &child_of, &layouts) else {
			continue;
		};
		let Ok(layout) = layouts.get(menu) else {
			continue;
		};
		let Some(selected) = card.selected_index(&hud, &text) else {
			continue;
		};
		motion.ease_to(*layout, card.ordinal, selected, card.count, dt);
		layout.apply_transform(&mut transform, &mut z_index, motion.angle, motion.scale);
	}
	for (entity, mut label, children, mut background) in &mut labels {
		let Some(_) = label.bind_menu(entity, &child_of, &layouts) else {
			continue;
		};
		let selected = label.is_selected(&hud, &text);
		let (fill, face) = label.paint(selected);
		background.0 = fill;
		for child in children.iter() {
			if let Ok(text_children) = caption_groups.get(child) {
				for text_child in text_children.iter() {
					if let Ok(mut color) = captions.get_mut(text_child) {
						color.0 = face;
					}
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::RotaryLayout;
	use super::RotaryMotion;

	#[test]
	fn inventory_motion_matches_the_legacy_carousel() {
		let layout = RotaryLayout::INVENTORY;
		assert!((layout.angle(0, 0, 4) - 0.0).abs() < f32::EPSILON);
		assert!((layout.x(0.0) - 0.0).abs() < f32::EPSILON);
		assert!((layout.y(0.0) - 14.0).abs() < f32::EPSILON);
		assert!((layout.scale_for(0, 0, 4) - 1.0).abs() < 0.001);
	}

	#[test]
	fn motion_takes_the_short_wrap() {
		let layout = RotaryLayout::INVENTORY;
		let mut motion = RotaryMotion { angle: 0.1, scale: 1.0 };
		motion.step_angle(std::f32::consts::TAU - 0.1, 1.0);
		assert!(motion.angle < 0.0, "should wrap backward, got {}", motion.angle);
		assert!((layout.angle(0, 1, 3) + layout.angle(1, 0, 3)).abs() < 0.001);
	}

	#[test]
	fn standing_portraits_keep_inventory_easing() {
		let inventory = RotaryLayout::INVENTORY;
		let portrait = inventory.with_standing_portraits();
		assert_eq!(portrait.min_scale, inventory.min_scale);
		assert_eq!(portrait.scale_span, inventory.scale_span);
		assert_eq!(portrait.ease, inventory.ease);
		assert!(portrait.card_height > inventory.card_height);
		assert!(
			portrait.x(portrait.angle(1, 0, 2)).abs() > 100.0,
			"two portrait cards must not stack at the front"
		);
		assert!(
			inventory.x(inventory.angle(1, 0, 2)).abs() < 1.0,
			"inventory still stacks a two-item ring at the front"
		);
	}
}
