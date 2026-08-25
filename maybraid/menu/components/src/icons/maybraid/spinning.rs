//! Spinning variant of [`Icon`].

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use crate::theme::{LOADING_SPIN_SECS, MAYBRAID_LOGO};

use super::Icon;

/// Rotate the mark about its center.
#[derive(Component, Debug, Clone, Copy)]
pub struct SpinningIcon {
	pub radians_per_sec: f32,
}

impl Default for SpinningIcon {
	fn default() -> Self {
		Self { radians_per_sec: TAU / LOADING_SPIN_SECS }
	}
}

impl SpinningIcon {
	pub fn maybraid(size: f32, color: Color) -> (Icon, Self) {
		(Icon::maybraid(size, color), Self::default())
	}

	pub fn maybraid_scene(size: f32, color: Color) -> impl Scene + 'static {
		let (icon, spinning) = Self::maybraid(size, color);
		Self::scene(icon, spinning)
	}

	pub fn scene(icon: Icon, spinning: Self) -> impl Scene + 'static {
		let size = icon.size;
		let color = icon.color;
		bsn! {
			template_value(icon)
			template_value(spinning)
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
}

pub fn spin_icons(time: Res<Time>, mut icons: Query<(&SpinningIcon, &mut UiTransform)>) {
	let dt = time.delta_secs();
	for (spin, mut transform) in &mut icons {
		transform.rotation *= Rot2::radians(spin.radians_per_sec * dt);
	}
}
