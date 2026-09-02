//! Spin-and-reveal slot: spinning mark, then a picture or a camera viewport.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::{FontSourceTemplate, LineBreak};

use crate::icons::maybraid::SpinningIcon;
use crate::theme::{
	BARLOW_SEMIBOLD, LOADING_ICON_SIZE, LOADING_STACK_GAP, PANEL_BLOCK_FONT_SIZE,
	PANEL_ITEM_FONT_SIZE, TEXT_YELLOW, TEXT_YELLOW_FAINT,
};

/// Seconds the son spins before the slot reveals its payload.
pub const SPIN_REVEAL_SECS: f32 = 1.4;

/// Inner size of the picture / camera slot.
pub const SPIN_REVEAL_SLOT_SIZE: f32 = 280.0;

/// Centered clothing-roll tile.
pub const SPIN_REVEAL_TILE_WIDTH: f32 = 260.0;

/// See [`SPIN_REVEAL_TILE_WIDTH`].
pub const SPIN_REVEAL_TILE_HEIGHT: f32 = 340.0;

/// What the slot shows after the spin.
#[derive(Clone, Debug)]
pub enum SpinRevealPayload {
	/// Asset path under `maybraid/assets` (PNG / image).
	Picture { image: &'static str },
	/// Host fills [`SpinRevealViewport`] (GLB / live camera).
	Camera { path: String },
}

/// Host path for a live camera viewport (GLB preview, render-to-texture, …).
#[derive(Component, Debug, Default, Clone)]
pub struct SpinRevealViewport {
	pub path: String,
}

/// Marker on the spinning overlay that hides the payload until reveal.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct SpinRevealCover;

/// Marker on the revealed payload root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct SpinRevealFace;

/// One spin-and-reveal cell. Cover visibility is owned by the screen plugin.
pub struct SpinRevealSlot {
	pub title: String,
	pub subtitle: String,
	pub payload: SpinRevealPayload,
	pub revealed: bool,
}

impl SpinRevealSlot {
	pub fn picture(
		title: impl Into<String>,
		subtitle: impl Into<String>,
		image: &'static str,
	) -> Self {
		Self {
			title: title.into(),
			subtitle: subtitle.into(),
			payload: SpinRevealPayload::Picture { image },
			revealed: false,
		}
	}

	pub fn camera(
		title: impl Into<String>,
		subtitle: impl Into<String>,
		path: impl Into<String>,
	) -> Self {
		Self {
			title: title.into(),
			subtitle: subtitle.into(),
			payload: SpinRevealPayload::Camera { path: path.into() },
			revealed: false,
		}
	}

	pub fn scene(self) -> impl Scene + 'static {
		let title = self.title;
		let subtitle = self.subtitle;
		let revealed = self.revealed;
		let cover_vis = if revealed { Visibility::Hidden } else { Visibility::Inherited };
		let face_vis = if revealed { Visibility::Inherited } else { Visibility::Hidden };
		let payload: Box<dyn Scene> = match self.payload {
			SpinRevealPayload::Picture { image } => Box::new(picture_scene(image, face_vis)),
			SpinRevealPayload::Camera { path } => Box::new(camera_scene(path, face_vis)),
		};
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(cover_scene(cover_vis)),
			payload,
			Box::new(caption_line(title, PANEL_BLOCK_FONT_SIZE, TEXT_YELLOW)),
			Box::new(caption_line(subtitle, PANEL_ITEM_FONT_SIZE, TEXT_YELLOW_FAINT)),
		];
		bsn! {
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

fn cover_scene(visibility: Visibility) -> impl Scene + 'static {
	let mark: Vec<Box<dyn Scene>> =
		vec![Box::new(SpinningIcon::maybraid_scene(LOADING_ICON_SIZE, TEXT_YELLOW))];
	bsn! {
		SpinRevealCover
		template_value(visibility)
		Node {
			width: px(SPIN_REVEAL_SLOT_SIZE),
			height: px(SPIN_REVEAL_SLOT_SIZE),
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
		}
		Pickable::IGNORE
		Children [ {mark} ]
	}
}

fn picture_scene(image: &'static str, visibility: Visibility) -> impl Scene + 'static {
	bsn! {
		SpinRevealFace
		template_value(visibility)
		ImageNode {
			image: image,
		}
		Node {
			width: px(SPIN_REVEAL_SLOT_SIZE),
			height: px(SPIN_REVEAL_SLOT_SIZE),
		}
		Pickable::IGNORE
	}
}

fn camera_scene(path: String, visibility: Visibility) -> impl Scene + 'static {
	bsn! {
		SpinRevealFace
		template_value(visibility)
		template_value(SpinRevealViewport { path })
		Node {
			width: px(SPIN_REVEAL_SLOT_SIZE),
			height: px(SPIN_REVEAL_SLOT_SIZE),
			border: px(2),
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
		}
		BorderColor::all(TEXT_YELLOW_FAINT)
		BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.85))
		Pickable::IGNORE
	}
}

fn caption_line(text: String, size: f32, color: Color) -> impl Scene + 'static {
	bsn! {
		template_value(Text::new(text))
		TextFont {
			font: FontSourceTemplate::Handle(BARLOW_SEMIBOLD),
			font_size: px(size),
		}
		TextColor(color)
		TextLayout::new(Justify::Center, LineBreak::WordBoundary)
		Pickable::IGNORE
	}
}
