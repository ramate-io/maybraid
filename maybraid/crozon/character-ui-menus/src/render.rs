use bevy::prelude::*;
use bevy_character_ui_menu_renderer::{
	MenuThumbnailContext, RenderContext, RenderMenu, Renderer,
};
use character_ui_menu::{AssetOption, LabelOption, ListValues, Root, SingleSelect, SwatchOption};

use crate::{
	character::{CharacterMenu, ConceptSpecies},
	event::{MenuEvent, SwatchValue},
	fields::{ColoredMultiSelectColors, AssetField, AssetFieldValue, ColoredMultiSelectField, CycleField, SliderField, SwatchField, SwatchFieldValue},
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
		renderer.render_cycle_row(
			parent,
			self.label,
			self.select.value.label(),
			MenuEvent::Cycle(self.field, -1),
			MenuEvent::Cycle(self.field, 1),
			context,
		);
	}
}

impl RenderMenu for SliderField {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		renderer.render_slider_row(
			parent,
			self.label,
			self.slider.value,
			self.slider.step,
			MenuEvent::SliderDelta(self.field, -self.slider.step),
			MenuEvent::SliderDelta(self.field, self.slider.step),
			context,
		);
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
		renderer.render_swatch_row(
			parent,
			self.label,
			self.swatch.value,
			|value| MenuEvent::SetSwatch(self.field, T::to_swatch_value(value)),
			context,
		);
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
		let color = context.preview_color;
		renderer.render_asset_grid(
			parent,
			self.label,
			self.select.value,
			|value| MenuEvent::SetAsset(self.field, T::to_asset_value(value)),
			|_| color,
			context,
		);
	}
}

impl RenderMenu for ColoredMultiSelectField<crozon_characters::species::common::ClothingMesh, crozon_characters::species::braidman::BraidmanColor> {
	fn render_with<Ctx: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, Ctx>,
	) {
		renderer.render_colored_multi_select(
			parent,
			self.label,
			&self.layers.selected,
			|value| self.color_for(value),
			|value| MenuEvent::ToggleClothing(value),
			|value, color| {
				MenuEvent::SetSwatch(
					crate::event::CharacterField::Clothing(value),
					SwatchValue::Braidman(color),
				)
			},
			|value| self.color_for(value).color(),
			context,
		);
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
	field: crate::event::CharacterField,
	select: SingleSelect<T>,
) -> CycleField<T> {
	CycleField { label, field, select }
}

pub(crate) fn slider_field(
	label: &'static str,
	field: crate::event::CharacterField,
	slider: character_ui_menu::Slider,
) -> SliderField {
	SliderField { label, field, slider }
}

pub(crate) fn swatch_field<T>(
	label: &'static str,
	field: crate::event::CharacterField,
	swatch: character_ui_menu::SwatchSingleSelect<T>,
) -> SwatchField<T> {
	SwatchField { label, field, swatch }
}

pub(crate) fn asset_field<T>(
	label: &'static str,
	field: crate::event::CharacterField,
	select: character_ui_menu::AssetSingleSelect<T>,
) -> AssetField<T> {
	AssetField { label, field, select }
}
