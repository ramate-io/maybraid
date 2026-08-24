//! Maybraid HUD renderer for typed character menus.
//!
//! Menus lower into the renderer-agnostic `MenuNode` IR (see
//! `character-ui-menu`); this crate paints those nodes with mid-size HUD
//! widgets from `menu-components`. Placement is host-owned: the sink only
//! left- or right-justifies content inside the parent the host provides.

pub mod event;
pub mod justify;
pub mod overlay;
pub mod plugin;
pub mod sink;
pub mod widgets;

pub use event::CharacterMenuEvent;
pub use justify::MenuJustify;
pub use overlay::{
	find_overlay_node, flatten_nodes, is_picker_only, is_select_node, overlay_closes_on_pick,
	overlay_summary_value, primary_select, render_overlay_body, spawn_overlay_shell,
};
pub use plugin::MaybraidCharacterMenuRendererPlugin;
pub use sink::{MaybraidMenuSink, MenuSink, MenuThumbnailContext, NoThumbnails, RenderContext};
pub use widgets::{
	CloseOverlaySelect, MenuButton, OpenSelectKey, OverlaySelectRoot, OverlaySelectViewport,
	ToggleSectionKey,
};
