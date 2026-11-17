//! Shared rotary-wheel geometry used by inventory selection and the gallery.

use bevy::prelude::*;

use super::hud_menu::HudMenu;
use crate::single_select::text_menu::TextMenu;
use crate::theme::{TEXT_DEEP_BLUE, TEXT_YELLOW, TEXT_YELLOW_FAINT};

/// Ring size and easing. Inventory uses a flattened carousel; the gallery
/// uses a larger wheel over the live character.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct RotaryLayout {
	pub radius: f32,
	pub front_offset: f32,
	pub min_scale: f32,
	pub scale_span: f32,
	pub ease: f32,
	pub card_width: f32,
	pub card_height: f32,
	pub track_alpha: f32,
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
		track_alpha: 0.72,
	};

	/// Full-screen character wheel laid over the gallery preview.
	pub const GALLERY: Self = Self {
		radius: 360.0,
		front_offset: 130.0,
		min_scale: 0.58,
		scale_span: 0.22,
		ease: 14.0,
		card_width: 280.0,
		card_height: 108.0,
		track_alpha: 0.18,
	};

	/// Name size on a gallery wheel card.
	pub const GALLERY_LABEL_PX: f32 = 32.0;

	/// Species / caption size under a gallery wheel name.
	pub const GALLERY_SUBTEXT_PX: f32 = 18.0;

	pub fn angle(self, ordinal: usize, selected: usize, count: usize) -> f32 {
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

	pub fn track_diameter(self) -> f32 {
		self.radius * 2.0 + 104.0
	}

	pub fn track_node(self) -> Node {
		let diameter = self.track_diameter();
		Node {
			position_type: PositionType::Absolute,
			left: Val::Percent(50.0),
			top: Val::Percent(50.0),
			width: Val::Px(diameter),
			height: Val::Auto,
			aspect_ratio: Some(1.0),
			margin: UiRect {
				left: Val::Px(-diameter / 2.0),
				top: Val::Px(-diameter / 2.0),
				..default()
			},
			border: UiRect::all(Val::Px(1.0)),
			border_radius: BorderRadius::MAX,
			..default()
		}
	}

	pub fn track_transform(self) -> UiTransform {
		let diameter = self.track_diameter();
		UiTransform {
			translation: Val2::px(0.0, self.card_height / 2.0 + 3.0),
			scale: Vec2::new(1.0, (self.front_offset * 2.0 + 2.0) / diameter),
			..default()
		}
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

	/// Full-screen host that sits over a live character preview.
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

	pub fn track_fill(self) -> Color {
		Color::srgba(0.35, 0.67, 0.88, self.track_alpha)
	}

	pub fn track_border(self) -> Color {
		Color::srgb(0.74, 0.87, 0.96)
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
		if *layout == RotaryLayout::GALLERY {
			// Gallery names follow the 3D lineup in menu-playground; keep easing
			// motion so those labels can reuse scale / depth.
			continue;
		}
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
	fn inventory_track_matches_the_legacy_carousel() {
		let layout = RotaryLayout::INVENTORY;
		assert!((layout.track_diameter() - 560.0).abs() < f32::EPSILON);
		assert!((layout.card_height / 2.0 + 3.0 - 58.0).abs() < f32::EPSILON);
		assert!((layout.angle(0, 0, 4) - 0.0).abs() < f32::EPSILON);
		assert!((layout.x(0.0) - 0.0).abs() < f32::EPSILON);
		assert!((layout.y(0.0) - 14.0).abs() < f32::EPSILON);
		assert!((layout.scale_for(0, 0, 4) - 1.0).abs() < 0.001);
	}

	#[test]
	fn motion_takes_the_short_wrap() {
		let layout = RotaryLayout::GALLERY;
		let mut motion = RotaryMotion { angle: 0.1, scale: 1.0 };
		motion.step_angle(std::f32::consts::TAU - 0.1, 1.0);
		assert!(motion.angle < 0.0, "should wrap backward, got {}", motion.angle);
		assert!((layout.angle(0, 1, 3) + layout.angle(1, 0, 3)).abs() < 0.001);
	}

	#[test]
	fn gallery_wheel_is_larger_than_inventory() {
		assert!(RotaryLayout::GALLERY.radius > RotaryLayout::INVENTORY.radius);
		assert!(RotaryLayout::GALLERY.front_offset > RotaryLayout::INVENTORY.front_offset);
	}
}
