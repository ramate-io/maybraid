use character_ui_menu::{AssetSingleSelect, MultiSelect, SingleSelect, Slider, SwatchSingleSelect};

use crate::event::{AssetValue, CharacterField, SwatchValue};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewColorSource {
	Body,
	Skin,
	Eye,
	Mouth,
	Hair,
	Horn,
	White,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CycleField<T> {
	pub label: &'static str,
	pub field: CharacterField,
	pub select: SingleSelect<T>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssetField<T> {
	pub label: &'static str,
	pub field: CharacterField,
	pub select: AssetSingleSelect<T>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwatchField<T> {
	pub label: &'static str,
	pub field: CharacterField,
	pub swatch: SwatchSingleSelect<T>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderField {
	pub label: &'static str,
	pub field: CharacterField,
	pub slider: Slider,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColoredMultiSelectField<T, C> {
	pub label: &'static str,
	pub layers: MultiSelect<T>,
	pub default_color: SwatchSingleSelect<C>,
	pub item_colors: Vec<crozon_characters::species::braidman::ClothingColor>,
}

pub trait AssetFieldValue: Copy {
	fn to_asset_value(value: Self) -> AssetValue;
}

pub trait SwatchFieldValue: Copy {
	fn to_swatch_value(value: Self) -> SwatchValue;
}

pub trait ColoredMultiSelectColors<T, C> {
	fn color_for(&self, item: T) -> C;
}

impl ColoredMultiSelectColors<crozon_characters::species::common::ClothingMesh, crozon_characters::species::braidman::BraidmanColor>
	for ColoredMultiSelectField<
		crozon_characters::species::common::ClothingMesh,
		crozon_characters::species::braidman::BraidmanColor,
	>
{
	fn color_for(
		&self,
		clothing: crozon_characters::species::common::ClothingMesh,
	) -> crozon_characters::species::braidman::BraidmanColor {
		self.item_colors
			.iter()
			.find(|choice| choice.clothing == clothing)
			.map(|choice| choice.color)
			.unwrap_or(self.default_color.value)
	}
}
