//! Bevy renderer for typed character menus.

pub mod event;
pub mod plugin;
pub mod render;
pub mod widgets;

pub use event::CharacterMenuEvent;
pub use plugin::CharacterMenuRendererPlugin;
pub use render::{
	MenuThumbnailContext, RenderContext, RenderMenu, Renderer,
};
pub use widgets::{AssetThumbnailHover, MenuButton, ToggleSectionKey};
