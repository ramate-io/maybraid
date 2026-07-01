use bevy::prelude::*;
use character_ui_menu::{AssetThumbnailDisplay, SectionOpen, ThumbnailRequest};

pub mod asset;
pub mod cycle;
pub mod labeled;
pub mod multi_select;
pub mod root;
pub mod section;
pub mod section_select;
pub mod select_picker;
pub mod single_select;
pub mod slider;
pub mod swatch;
mod util;

pub use asset::{AssetEventMap, AssetGrid, AssetSelect};
pub use multi_select::{
	ClothingSwatchEventMap, ColoredAssetMultiSelect, ColoredMultiSelectMaps, ItemColorMap,
	ItemPreviewColorMap, ToggleEventMap,
};
pub use section_select::{SectionMenuMap, SectionSelect};
pub use select_picker::{SelectEventMap, SelectPicker};
pub use swatch::{SwatchEventMap, SwatchPicker, SwatchSelect};

/// Renderer-owned thumbnail bridge. The playground adapts this to its cache.
pub trait MenuThumbnailContext {
	fn image_for_asset(
		&mut self,
		label: &'static str,
		asset_path: &'static str,
		color: Color,
		camera: character_ui_menu::ThumbnailCamera,
	) -> Option<Handle<Image>>;
}

pub struct RenderContext<'a, T> {
	pub sections: &'a dyn SectionOpen,
	pub thumbnails: &'a mut T,
	pub asset_thumbnails: AssetThumbnailDisplay,
	pub preview_color: Color,
	pub base_preview_color: Color,
	pub accent_preview_color: Color,
	pub prewarm: &'a mut Vec<ThumbnailRequest>,
}

#[derive(Default)]
pub struct Renderer;

pub trait RenderMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	);
}

impl Renderer {
	pub fn render<M, C>(
		&self,
		parent: &mut ChildSpawnerCommands,
		menu: &M,
		context: &mut RenderContext<'_, C>,
	) where
		M: RenderMenu,
		C: MenuThumbnailContext,
	{
		menu.render_with(self, parent, context);
	}
}
