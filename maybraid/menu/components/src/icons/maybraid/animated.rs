//! Blinking variant of [`Icon`].

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use crate::theme::{
	ICON_BLINK_ALPHA_MAX, ICON_BLINK_ALPHA_MIN, ICON_BLINK_SECS, MAYBRAID_LOGO, TEXT_YELLOW,
};

use super::Icon;

/// Pulse the mark alpha so it reads as a live light.
#[derive(Component, Debug, Clone, Copy)]
pub struct AnimatedIcon {
	pub color: Color,
}

impl Default for AnimatedIcon {
	fn default() -> Self {
		Self { color: TEXT_YELLOW }
	}
}

impl AnimatedIcon {
	pub fn maybraid(size: f32, color: Color) -> (Icon, Self) {
		(Icon::maybraid(size, color), Self { color })
	}

	pub fn maybraid_scene(size: f32, color: Color) -> impl Scene + 'static {
		let (icon, animated) = Self::maybraid(size, color);
		Self::scene(icon, animated)
	}

	pub fn maybraid_scene_with_visibility(
		size: f32,
		color: Color,
		visibility: Visibility,
	) -> impl Scene + 'static {
		let (icon, animated) = Self::maybraid(size, color);
		Self::scene_with_visibility(icon, animated, visibility)
	}

	pub fn scene(icon: Icon, animated: Self) -> impl Scene + 'static {
		Self::scene_with_visibility(icon, animated, Visibility::Inherited)
	}

	pub fn scene_with_visibility(
		icon: Icon,
		animated: Self,
		visibility: Visibility,
	) -> impl Scene + 'static {
		let size = icon.size;
		let color = icon.color;
		bsn! {
			template_value(icon)
			template_value(animated)
			template_value(visibility)
			ImageNode {
				image: MAYBRAID_LOGO,
				color: color,
			}
			Node {
				width: px(size),
				height: px(size),
				flex_shrink: 0.0,
			}
			Pickable::IGNORE
		}
	}

	/// Imperative spawn for sinks that walk a dynamic tree.
	pub fn spawn(
		icon: Icon,
		animated: Self,
		parent: &mut ChildSpawnerCommands,
		image: Handle<Image>,
		visibility: Visibility,
	) {
		parent.spawn((
			icon,
			animated,
			visibility,
			ImageNode { image, color: icon.color, ..default() },
			Node {
				width: Val::Px(icon.size),
				height: Val::Px(icon.size),
				flex_shrink: 0.0,
				..default()
			},
			Pickable::IGNORE,
		));
	}
}

pub fn blink_animated_icons(time: Res<Time>, mut icons: Query<(&AnimatedIcon, &mut ImageNode)>) {
	let phase = (time.elapsed_secs() * TAU / ICON_BLINK_SECS).sin().mul_add(0.5, 0.5);
	let alpha = ICON_BLINK_ALPHA_MIN + (ICON_BLINK_ALPHA_MAX - ICON_BLINK_ALPHA_MIN) * phase;
	for (animated, mut image) in &mut icons {
		image.color = animated.color.with_alpha(alpha);
	}
}
