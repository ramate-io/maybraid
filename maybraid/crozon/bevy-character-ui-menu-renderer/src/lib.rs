//! Bevy renderer for typed Crozon character menus.

pub mod plugin;
pub mod render;

pub use plugin::CharacterUiMenuRendererPlugin;
pub use render::{
	render_asset_select, render_multi_select, render_section, render_single_cycle, render_slider,
	render_swatch_select, MenuButton, MenuThumbnailContext,
};
