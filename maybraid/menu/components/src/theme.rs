//! Placeholder type tokens for Maybraid HUD menus.
//!
//! Faces and sizes are a first cut so screens can iterate by eye.

use bevy::prelude::*;

/// Barlow Semi Condensed Black, under `maybraid/assets`.
pub const BARLOW_BLACK: &str = "fonts/barlow/BarlowSemiCondensed-Black.ttf";

/// Barlow Semi Condensed SemiBold, under `maybraid/assets`.
pub const BARLOW_SEMIBOLD: &str = "fonts/barlow/BarlowSemiCondensed-SemiBold.ttf";

/// Largest face on a text menu (title / header).
pub const HEADER_FONT_SIZE: f32 = 96.0;

/// Large option labels, still smaller than [`HEADER_FONT_SIZE`].
pub const ITEM_FONT_SIZE: f32 = 48.0;

/// Idle menu yellow.
pub const TEXT_YELLOW: Color = Color::srgb(1.0, 0.86, 0.22);

/// Hover / keyboard-focus yellow (slightly darker).
pub const TEXT_YELLOW_HOVER: Color = Color::srgb(0.82, 0.68, 0.12);

/// Extra space under a header before the first option.
pub const HEADER_MARGIN_BOTTOM: f32 = 16.0;

/// Gap between stacked options.
pub const ITEM_ROW_GAP: f32 = 6.0;

/// Inset from the bottom-left of the window.
pub const COLUMN_INSET: f32 = 48.0;
