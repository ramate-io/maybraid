use bevy::prelude::*;
use bevy_character_ui_menu_renderer::{
	AssetEventMap, AssetSelect, ClothingSwatchEventMap, ColoredAssetMultiSelect, ColoredMultiSelectMaps,
	ItemColorMap, ItemPreviewColorMap, MenuThumbnailContext, RenderContext, RenderMenu, Renderer,
	SectionMenuMap, SectionSelect, SelectEventMap, SwatchEventMap, SwatchSelect, ToggleEventMap,
};
use character_ui_menu::{
	AssetOption, BlockLabeled, Cycle, Labeled, LabelOption, ListValues, Root, SingleSelect, Slider,
	SliderStep, SwatchOption,
};
use crozon_characters::species::{
	braidman::BraidmanColor,
	common::ClothingMesh,
};

use crate::{
	character::{CharacterMenu, ConceptSpecies},
	event::{CharacterField, MenuEvent, SwatchValue},
	fields::{AssetFieldValue, ColoredMultiSelectColors, SwatchFieldValue},
};

pub mod braidman;
pub mod brodler;
pub mod dui;
pub mod mygr;
pub mod wumbus;
pub mod lero;
pub mod spibmom;
mod values;

#[derive(Clone, Copy)]
struct SpeciesSelectEvents;

impl SelectEventMap<MenuEvent, ConceptSpecies> for SpeciesSelectEvents {
	fn select_event(&self, species: ConceptSpecies) -> MenuEvent {
		MenuEvent::SetSpecies(species)
	}
}

struct CharacterSpeciesMenus<'a> {
	menu: &'a CharacterMenu,
}

impl SectionMenuMap<ConceptSpecies> for CharacterSpeciesMenus<'_> {
	fn render_menu_for<C: MenuThumbnailContext>(
		&self,
		species: ConceptSpecies,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		match species {
			ConceptSpecies::Braidman => {
				Root::new(self.menu.braidman.clone()).render_with(renderer, parent, context)
			}
			ConceptSpecies::Brodler => {
				Root::new(self.menu.brodler.clone()).render_with(renderer, parent, context)
			}
			ConceptSpecies::Mygr => {
				Root::new(self.menu.mygr.clone()).render_with(renderer, parent, context)
			}
			ConceptSpecies::Dui => {
				Root::new(self.menu.dui.clone()).render_with(renderer, parent, context)
			}
			ConceptSpecies::Wumbus => {
				Root::new(self.menu.wumbus.clone()).render_with(renderer, parent, context)
			}
			ConceptSpecies::Lero => {
				Root::new(self.menu.lero.clone()).render_with(renderer, parent, context)
			}
			ConceptSpecies::Spibmom => {
				Root::new(self.menu.spibmom.clone()).render_with(renderer, parent, context)
			}
		}
	}
}

#[derive(Clone, Copy)]
pub(crate) struct FieldAssetEvents(CharacterField);

impl<T> AssetEventMap<MenuEvent, T> for FieldAssetEvents
where
	T: AssetFieldValue + Copy,
{
	fn select_event(&self, value: T) -> MenuEvent {
		MenuEvent::SetAsset(self.0, T::to_asset_value(value))
	}
}

#[derive(Clone, Copy)]
pub(crate) struct FieldSwatchEvents(CharacterField);

impl<T> SwatchEventMap<MenuEvent, T> for FieldSwatchEvents
where
	T: SwatchFieldValue + Copy,
{
	fn swatch_event(&self, value: T) -> MenuEvent {
		MenuEvent::SetSwatch(self.0, T::to_swatch_value(value))
	}
}

struct ClothingFieldMaps<'a> {
	field: &'a crate::fields::ColoredMultiSelectField<ClothingMesh, BraidmanColor>,
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

impl RenderMenu for CharacterMenu {
	fn render_with<Ctx: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, Ctx>,
	) {
		let species_menus = CharacterSpeciesMenus { menu: self };
		SectionSelect::new("Species", self.species, SpeciesSelectEvents, &species_menus)
			.render_with(renderer, parent, context);
	}
}

pub(crate) fn labeled_cycle<T>(
	label: &'static str,
	field: CharacterField,
	select: SingleSelect<T>,
) -> Labeled<Cycle<MenuEvent, T>>
where
	T: Copy + LabelOption,
{
	Labeled::new(
		label,
		Cycle::new(
			select,
			MenuEvent::Cycle(field, -1),
			MenuEvent::Cycle(field, 1),
		),
	)
}

pub(crate) fn labeled_slider(
	label: &'static str,
	field: CharacterField,
	slider: Slider,
) -> Labeled<SliderStep<MenuEvent>> {
	Labeled::new(
		label,
		SliderStep::new(
			slider,
			MenuEvent::SliderDelta(field, -slider.step),
			MenuEvent::SliderDelta(field, slider.step),
		),
	)
}

pub(crate) fn labeled_swatch<T>(
	label: &'static str,
	field: CharacterField,
	swatch: character_ui_menu::SwatchSingleSelect<T>,
) -> Labeled<SwatchSelect<MenuEvent, T, FieldSwatchEvents>>
where
	T: SwatchFieldValue + Copy + LabelOption + ListValues + SwatchOption,
{
	Labeled::new(label, SwatchSelect::new(swatch, FieldSwatchEvents(field)))
}

pub(crate) fn block_asset<T>(
	label: &'static str,
	field: CharacterField,
	select: character_ui_menu::AssetSingleSelect<T>,
) -> BlockLabeled<AssetSelect<MenuEvent, T, FieldAssetEvents>>
where
	T: AssetFieldValue + Copy + LabelOption + ListValues + AssetOption,
{
	BlockLabeled::new(label, AssetSelect::new(select, FieldAssetEvents(field)))
}

pub(crate) fn render_colored_clothing<C: MenuThumbnailContext>(
	field: &crate::fields::ColoredMultiSelectField<ClothingMesh, BraidmanColor>,
	renderer: &Renderer,
	parent: &mut ChildSpawnerCommands,
	context: &mut RenderContext<'_, C>,
) {
	let maps = ClothingFieldMaps { field };
	ColoredAssetMultiSelect::new(field.label, &field.layers.selected, &maps)
		.render_with(renderer, parent, context);
}
