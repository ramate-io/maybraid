//! Placeholder type tokens for Maybraid HUD menus.
//!
//! Faces and sizes are a first cut so screens can iterate by eye.

use bevy::prelude::*;

/// Barlow Semi Condensed Black, under `maybraid/assets`.
pub const BARLOW_BLACK: &str = "fonts/barlow/BarlowSemiCondensed-Black.ttf";

/// Barlow Semi Condensed SemiBold, under `maybraid/assets`.
pub const BARLOW_SEMIBOLD: &str = "fonts/barlow/BarlowSemiCondensed-SemiBold.ttf";

/// Barlow Semi Condensed Regular, under `maybraid/assets`.
pub const BARLOW_REGULAR: &str = "fonts/barlow/BarlowSemiCondensed-Regular.ttf";

/// Largest face on a text menu (title / header).
pub const HEADER_FONT_SIZE: f32 = 96.0;

/// Large option labels, still smaller than [`HEADER_FONT_SIZE`].
pub const ITEM_FONT_SIZE: f32 = 48.0;

/// Idle menu yellow.
pub const TEXT_YELLOW: Color = Color::srgb(1.0, 0.86, 0.22);

/// Hover / keyboard-focus yellow (slightly darker).
pub const TEXT_YELLOW_HOVER: Color = Color::srgb(0.82, 0.68, 0.12);

/// Description strip: same yellow, lower alpha.
pub const TEXT_YELLOW_FAINT: Color = Color::srgba(1.0, 0.86, 0.22, 0.42);

/// Description line under the menu column.
pub const DESCRIPTION_FONT_SIZE: f32 = 22.0;

/// Inset from the bottom of the window for the description strip.
pub const DESCRIPTION_BOTTOM: f32 = 24.0;

/// Extra space under a header before the first option.
pub const HEADER_MARGIN_BOTTOM: f32 = 16.0;

/// Gap between stacked options.
pub const ITEM_ROW_GAP: f32 = 6.0;

/// Inset from the left of the window.
pub const COLUMN_INSET: f32 = 48.0;

/// Menu column sits above the description strip.
pub const COLUMN_BOTTOM: f32 = 88.0;
