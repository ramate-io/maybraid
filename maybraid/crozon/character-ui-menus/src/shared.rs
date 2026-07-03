//! Menu sections shared by every species.
//!
//! Hair, clothing, and animation controls are structurally identical across
//! species, so they are defined once here instead of per species. Each
//! implements [`MenuComponent`] to lower itself into [`MenuNode`]s.

use character_ui_menu::{
	AssetChoice, AssetSingleSelect, CameraFocus, ItemRow, MenuComponent, MenuNode, MultiSelect,
	PreviewColor, SwatchChoice, SwatchSingleSelect,
};
use crozon_character_items::{ClothingColor, ClothingMesh, ItemColor};
use crozon_characters::ConceptAnimation;

use crate::event::{AssetValue, CharacterField, MenuEvent, SwatchValue};

/// Hair style and color; species only differ in default camera framing.
#[derive(Clone, Debug, PartialEq)]
pub struct HairMenu {
	pub style: AssetSingleSelect<crozon_characters::species::common::HairMesh>,
	pub color: SwatchSingleSelect<ItemColor>,
}

impl HairMenu {
	pub fn new(
		style: crozon_characters::species::common::HairMesh,
		color: ItemColor,
		focus: CameraFocus,
	) -> Self {
		Self {
			style: AssetSingleSelect::new(style).with_camera_focus(focus),
			color: SwatchSingleSelect::new(color),
		}
	}
}

impl MenuComponent<MenuEvent> for HairMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Hair",
				&self.style,
				PreviewColor::of(self.color.value),
				|value| MenuEvent::SetAsset(CharacterField::Hair, AssetValue::Hair(value)),
			),
			MenuNode::swatch("Hair Color", &self.color, |color| {
				MenuEvent::SetSwatch(CharacterField::HairColor, SwatchValue::Item(color))
			}),
		])
	}
}

/// Clothing layers with a default color plus per-layer overrides.
#[derive(Clone, Debug, PartialEq)]
pub struct ClothingMenu {
	pub layers: MultiSelect<ClothingMesh>,
	pub default_color: SwatchSingleSelect<ItemColor>,
	pub item_colors: Vec<ClothingColor>,
}

impl ClothingMenu {
	pub fn new(
		layers: Vec<ClothingMesh>,
		default_color: ItemColor,
		overrides: Vec<ClothingColor>,
	) -> Self {
		Self {
			layers: MultiSelect::new(layers),
			default_color: SwatchSingleSelect::new(default_color),
			item_colors: overrides,
		}
	}

	pub fn color_for(&self, clothing: ClothingMesh) -> ItemColor {
		ClothingColor::resolve(&self.item_colors, self.default_color.value, clothing)
	}

	pub fn set_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		ClothingColor::set(&mut self.item_colors, clothing, color);
	}
}

impl MenuComponent<MenuEvent> for ClothingMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		let rows = ClothingMesh::VALUES
			.iter()
			.map(|&clothing| {
				let selected = self.layers.contains(clothing);
				let color = self.color_for(clothing);
				ItemRow {
					asset: AssetChoice::new(
						clothing,
						selected,
						MenuEvent::ToggleClothing(clothing),
					),
					preview: PreviewColor::of(color),
					// Color choices only matter for worn layers.
					colors: if selected {
						SwatchChoice::from_values(color, |color| {
							MenuEvent::SetSwatch(
								CharacterField::Clothing(clothing),
								SwatchValue::Item(color),
							)
						})
					} else {
						Vec::new()
					},
				}
			})
			.collect();
		MenuNode::ItemMultiSelect { label: "Clothing", rows }
	}
}

/// Animation clip picker.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationMenu {
	pub clip: AssetSingleSelect<ConceptAnimation>,
}

impl AnimationMenu {
	pub fn new(focus: CameraFocus) -> Self {
		Self { clip: AssetSingleSelect::new(ConceptAnimation::Still).with_camera_focus(focus) }
	}
}

impl MenuComponent<MenuEvent> for AnimationMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::asset_grid("Animation", &self.clip, PreviewColor::WHITE, |value| {
			MenuEvent::SetAsset(CharacterField::Animation, AssetValue::Animation(value))
		})
	}
}
