//! Bevy renderer for typed character menus.

pub mod event;
pub mod plugin;
pub mod render;
pub mod widgets;

pub use event::CharacterMenuEvent;
pub use plugin::CharacterMenuRendererPlugin;
pub use render::{
	AssetEventMap, AssetGrid, AssetSelect, ClothingSwatchEventMap, ColoredAssetMultiSelect,
	ColoredMultiSelectMaps, ItemColorMap, ItemPreviewColorMap, MenuThumbnailContext, RenderContext,
	RenderMenu, Renderer, SectionMenuMap, SectionSelect, SelectEventMap, SelectPicker,
	SwatchEventMap, SwatchPicker, SwatchSelect, ToggleEventMap,
};
pub use widgets::{AssetThumbnailHover, MenuButton, ToggleSectionKey};
