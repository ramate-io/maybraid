//! Menu sections shared by every species.
//!
//! Hair, clothing, and animation controls are structurally identical across
//! species, so they are defined once here instead of per species. Each
//! implements [`MenuComponent`] to lower itself into [`MenuNode`]s.

use character_ui_menu::{
	AssetChoice, AssetSingleSelect, CameraFocus, ItemRow, MenuComponent, MenuNode, MultiSelect,
	PreviewColor, SingleSelect, SwatchChoice, SwatchSingleSelect,
};
use crozon_character_items::{ClothingColor, ClothingMaterial, ClothingMesh, ItemColor};
use crozon_characters::ConceptAnimation;

use crate::{
	cycle_value,
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
};

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
	pub material: SingleSelect<ClothingMaterial>,
}

impl ClothingMenu {
	pub fn new(
		layers: Vec<ClothingMesh>,
		default_color: ItemColor,
		overrides: Vec<ClothingColor>,
		material: ClothingMaterial,
	) -> Self {
		Self {
			layers: MultiSelect::new(layers),
			default_color: SwatchSingleSelect::new(default_color),
			item_colors: overrides,
			material: SingleSelect::new(material),
		}
	}

	pub fn color_for(&self, clothing: ClothingMesh) -> ItemColor {
		ClothingColor::resolve(&self.item_colors, self.default_color.value, clothing)
	}

	pub fn set_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		ClothingColor::set(&mut self.item_colors, clothing, color);
	}
}

/// Clothing toggle / color / material events shared by every species menu.
pub(crate) fn apply_clothing_event(menu: &mut ClothingMenu, event: MenuEvent) -> bool {
	match event {
		MenuEvent::ToggleClothing(clothing) => {
			menu.layers.toggle(clothing);
			true
		}
		MenuEvent::SetSwatch(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
			menu.set_color(clothing, color);
			true
		}
		MenuEvent::SetAsset(
			CharacterField::ClothingMaterial,
			AssetValue::ClothingMaterial(material),
		) => {
			menu.material.value = material;
			true
		}
		MenuEvent::Cycle(CharacterField::ClothingMaterial, delta) => {
			menu.material.value = cycle_value(menu.material.value, delta);
			true
		}
		_ => false,
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
		MenuNode::fragment([
			MenuNode::section_select(
				"Material",
				self.material.value,
				|value| {
					MenuEvent::SetAsset(
						CharacterField::ClothingMaterial,
						AssetValue::ClothingMaterial(value),
					)
				},
				MenuNode::fragment([]),
			),
			MenuNode::ItemMultiSelect { label: "Clothing", rows },
		])
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
