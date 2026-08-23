//! Centered loading stack: spinning mark, thin bar, explainer.

pub mod bar;
pub mod explainer;

pub use bar::{loading_bar_scene, set_loading_progress, sync_loading_bar_fill, LoadingBarFill};
pub use explainer::{set_loading_explainer, LoadingExplainer};

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};

use crate::icons::maybraid::SpinningIcon;
use crate::theme::{LOADING_ICON_SIZE, LOADING_STACK_GAP, TEXT_YELLOW};

/// Marker on the stacked loading chrome.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct LoadingPanel;

/// Spinning Maybraid mark, fillable bar, and explainer line.
pub struct LoadingStack {
	pub progress: f32,
	pub explainer: String,
}

impl LoadingStack {
	pub fn new(progress: f32, explainer: impl Into<String>) -> Self {
		Self { progress, explainer: explainer.into() }
	}

	pub fn scene(self) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(SpinningIcon::maybraid_scene(LOADING_ICON_SIZE, TEXT_YELLOW)),
			Box::new(loading_bar_scene(self.progress)),
			Box::new(LoadingExplainer::scene(self.explainer)),
		];
		bsn! {
			LoadingPanel
			Node {
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::Center,
				row_gap: px(LOADING_STACK_GAP),
			}
			Pickable::IGNORE
			Children [ {children} ]
		}
	}
}
