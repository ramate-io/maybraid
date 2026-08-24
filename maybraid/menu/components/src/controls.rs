//! Mid-size HUD controls for inspector-style panels.
//!
//! These are imperative spawn helpers for dynamic trees (character menus).
//! Full-screen home / loading screens keep using BSN [`Scene`]s.

pub mod button;
pub mod fonts;
pub mod row;
pub mod section;
pub mod select_list;
pub mod stepper;
pub mod swatch;
pub mod text;
pub mod tile;

pub use button::spawn_text_button;
pub use fonts::HudFonts;
pub use row::spawn_labeled_row;
pub use section::spawn_section_header;
pub use select_list::spawn_select_row;
pub use stepper::spawn_stepper;
pub use swatch::{color_from_hex, spawn_swatch, spawn_swatch_row};
pub use text::{spawn_block_label, spawn_cursor_slot, spawn_group_label, spawn_hud_text};
pub use tile::{spawn_asset_tile, spawn_tile_grid};
