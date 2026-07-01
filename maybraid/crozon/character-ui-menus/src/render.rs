use bevy::prelude::*;
use bevy_character_ui_menu_renderer::{
	AssetEventMap, ClothingSwatchEventMap, ColoredAssetMultiSelect, ColoredMultiSelectMaps,
	ItemColorMap, ItemPreviewColorMap, LabeledAssetGrid, LabeledCycle, LabeledSlider, LabeledSwatch,
	MenuThumbnailContext, RenderContext, RenderMenu, Renderer, SwatchEventMap, ToggleEventMap,
};
use character_ui_menu::{AssetOption, LabelOption, ListValues, Root, SwatchOption};
use crozon_characters::species::{
	braidman::BraidmanColor,
	common::ClothingMesh,
};

use crate::{
	character::{CharacterMenu, ConceptSpecies},
	event::{CharacterField, MenuEvent, SwatchValue},
	fields::{
		AssetField, AssetFieldValue, ColoredMultiSelectColors, ColoredMultiSelectField, CycleField,
		SliderField, SwatchField, SwatchFieldValue,
	},
};

pub mod braidman;
pub mod brodler;
mod values;

impl<T> RenderMenu for CycleField<T>
where
	T: Copy + LabelOption + ListValues,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		LabeledCycle {
			label: self.label,
			value_label: self.select.value.label(),
			minus: MenuEvent::Cycle(self.field, -1),
			plus: MenuEvent::Cycle(self.field, 1),
		}
		.render_with(renderer, parent, context);
	}
}

impl RenderMenu for SliderField {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		LabeledSlider {
			label: self.label,
			slider: self.slider,
			decrease: MenuEvent::SliderDelta(self.field, -self.slider.step),
			increase: MenuEvent::SliderDelta(self.field, self.slider.step),
		}
		.render_with(renderer, parent, context);
	}
}

struct SwatchEvents {
	field: CharacterField,
}

impl<T> SwatchEventMap<MenuEvent, T> for SwatchEvents
where
	T: SwatchFieldValue + Copy,
{
	fn swatch_event(&self, value: T) -> MenuEvent {
		MenuEvent::SetSwatch(self.field, T::to_swatch_value(value))
	}
}

impl<T> RenderMenu for SwatchField<T>
where
	T: SwatchFieldValue + Copy + LabelOption + ListValues + SwatchOption,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		let map = SwatchEvents { field: self.field };
		LabeledSwatch::new(self.label, self.swatch.value, &map).render_with(renderer, parent, context);
	}
}

struct AssetEvents {
	field: CharacterField,
}

impl<T> AssetEventMap<MenuEvent, T> for AssetEvents
where
	T: AssetFieldValue + Copy,
{
	fn select_event(&self, value: T) -> MenuEvent {
		MenuEvent::SetAsset(self.field, T::to_asset_value(value))
	}
}

impl<T> RenderMenu for AssetField<T>
where
	T: AssetFieldValue + Copy + LabelOption + ListValues + AssetOption,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		let map = AssetEvents { field: self.field };
		LabeledAssetGrid::new(self.label, self.select.value, &map)
			.render_with(renderer, parent, context);
	}
}

struct ClothingFieldMaps<'a> {
	field: &'a ColoredMultiSelectField<ClothingMesh, BraidmanColor>,
}

impl ToggleEventMap<MenuEvent, ClothingMesh> for ClothingFieldMaps<'_> {
	fn toggle_event(&self, value: ClothingMesh) -> MenuEvent {
		MenuEvent::ToggleClothing(value)
	}
}

impl ClothingSwatchEventMap<MenuEvent, ClothingMesh, BraidmanColor> for ClothingFieldMaps<'_> {
	fn color_event(&self, item: ClothingMesh, color: BraidmanColor) -> MenuEvent {
		MenuEvent::SetSwatch(CharacterField::Clothing(item), SwatchValue::Braidman(color))
	}
}

impl ItemColorMap<ClothingMesh, BraidmanColor> for ClothingFieldMaps<'_> {
	fn color_for(&self, item: ClothingMesh) -> BraidmanColor {
		self.field.color_for(item)
	}
}

impl ItemPreviewColorMap<ClothingMesh> for ClothingFieldMaps<'_> {
	fn preview_color(&self, item: ClothingMesh) -> Color {
		self.field.color_for(item).color()
	}
}

impl ColoredMultiSelectMaps<MenuEvent, ClothingMesh, BraidmanColor> for ClothingFieldMaps<'_> {}

impl RenderMenu for ColoredMultiSelectField<ClothingMesh, BraidmanColor> {
	fn render_with<Ctx: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, Ctx>,
	) {
		let maps = ClothingFieldMaps { field: self };
		ColoredAssetMultiSelect::new(self.label, &self.layers.selected, &maps)
			.render_with(renderer, parent, context);
	}
}

impl RenderMenu for CharacterMenu {
	fn render_with<Ctx: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, Ctx>,
	) {
		match self.species.value {
			ConceptSpecies::Braidman => {
				Root::new(self.braidman.clone()).render_with(renderer, parent, context)
			}
			ConceptSpecies::Brodler => {
				Root::new(self.brodler.clone()).render_with(renderer, parent, context)
			}
		}
	}
}

pub(crate) fn cycle_field<T>(
	label: &'static str,
	field: CharacterField,
	select: character_ui_menu::SingleSelect<T>,
) -> CycleField<T> {
	CycleField { label, field, select }
}

pub(crate) fn slider_field(
	label: &'static str,
	field: CharacterField,
	slider: character_ui_menu::Slider,
) -> SliderField {
	SliderField { label, field, slider }
}

pub(crate) fn swatch_field<T>(
	label: &'static str,
	field: CharacterField,
	swatch: character_ui_menu::SwatchSingleSelect<T>,
) -> SwatchField<T> {
	SwatchField { label, field, swatch }
}

pub(crate) fn asset_field<T>(
	label: &'static str,
	field: CharacterField,
	select: character_ui_menu::AssetSingleSelect<T>,
) -> AssetField<T> {
	AssetField { label, field, select }
}
