//! Thin fillable loading track.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use crate::theme::{LOADING_BAR_HEIGHT, LOADING_BAR_WIDTH, TEXT_YELLOW, TEXT_YELLOW_FAINT};

/// Fill of the loading track. [`progress`] is 0..=1.
#[derive(Component, Debug, Clone, Copy)]
pub struct LoadingBarFill {
	pub progress: f32,
}

impl Default for LoadingBarFill {
	fn default() -> Self {
		Self { progress: 0.0 }
	}
}

impl LoadingBarFill {
	pub fn new(progress: f32) -> Self {
		Self { progress: progress.clamp(0.0, 1.0) }
	}
}

/// Track plus fill. Width of the fill follows [`LoadingBarFill::progress`].
pub fn loading_bar_scene(progress: f32) -> impl Scene + 'static {
	let fill = LoadingBarFill::new(progress);
	let width_percent = fill.progress * 100.0;
	bsn! {
		Node {
			width: px(LOADING_BAR_WIDTH),
			height: px(LOADING_BAR_HEIGHT),
			flex_shrink: 0.0,
			overflow: Overflow::clip(),
		}
		BackgroundColor(TEXT_YELLOW_FAINT)
		Pickable::IGNORE
		Children [(
			template_value(fill)
			Node {
				width: percent(width_percent),
				height: percent(100),
			}
			BackgroundColor(TEXT_YELLOW)
			Pickable::IGNORE
		)]
	}
}

pub fn sync_loading_bar_fill(
	mut fills: Query<(&LoadingBarFill, &mut Node), Changed<LoadingBarFill>>,
) {
	for (fill, mut node) in &mut fills {
		node.width = percent(fill.progress.clamp(0.0, 1.0) * 100.0);
	}
}

/// Write `progress` (0..=1) onto the [`LoadingBarFill`] under `root`.
pub fn set_loading_progress(
	root: Entity,
	progress: f32,
	children: &Query<&Children>,
	fills: &mut Query<&mut LoadingBarFill>,
) {
	let progress = progress.clamp(0.0, 1.0);
	set_fill_under(root, progress, children, fills);
}

fn set_fill_under(
	entity: Entity,
	progress: f32,
	children: &Query<&Children>,
	fills: &mut Query<&mut LoadingBarFill>,
) -> bool {
	if let Ok(mut fill) = fills.get_mut(entity) {
		fill.progress = progress;
		return true;
	}
	let Ok(kids) = children.get(entity) else {
		return false;
	};
	for child in kids {
		if set_fill_under(*child, progress, children, fills) {
			return true;
		}
	}
	false
}

#[cfg(test)]
mod tests {
	use super::LoadingBarFill;

	#[test]
	fn new_clamps() {
		assert_eq!(LoadingBarFill::new(-0.2).progress, 0.0);
		assert_eq!(LoadingBarFill::new(1.4).progress, 1.0);
	}
}
