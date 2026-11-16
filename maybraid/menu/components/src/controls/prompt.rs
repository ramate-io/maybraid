//! Kenney input chips for inventory and gallery actions. Glyphs follow the live
//! device and the real Select / drop / Back / Delete bindings — not hard-coded
//! button letters.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use bevy::text::{FontSourceTemplate, Justify};
use bevy::ui_widgets::Button;

use crate::theme::{
	BARLOW_SEMIBOLD, CORNER_BOTTOM, PANEL_BLOCK_FONT_SIZE, PANEL_GROUP_FONT_SIZE, PROMPT_KEY_BACK,
	PROMPT_KEY_DELETE, PROMPT_KEY_DROP, PROMPT_KEY_EDIT, PROMPT_KEY_SELECT, PROMPT_KEY_UNEQUIP,
	PROMPT_PAD_BACK, PROMPT_PAD_DELETE, PROMPT_PAD_DROP, PROMPT_PAD_EDIT, PROMPT_PAD_SELECT,
	PROMPT_PAD_UNEQUIP, TEXT_YELLOW,
};
use crate::HudFonts;

/// Last device that produced a press. Starts on keyboard so headless shots
/// show key chips until a pad speaks.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputPromptDevice {
	#[default]
	Keyboard,
	Pad,
}

impl InputPromptDevice {
	pub fn select_icon(self) -> &'static str {
		match self {
			Self::Pad => PROMPT_PAD_SELECT,
			Self::Keyboard => PROMPT_KEY_SELECT,
		}
	}

	pub fn back_icon(self) -> &'static str {
		match self {
			Self::Pad => PROMPT_PAD_BACK,
			Self::Keyboard => PROMPT_KEY_BACK,
		}
	}

	pub fn drop_icon(self) -> &'static str {
		match self {
			Self::Pad => PROMPT_PAD_DROP,
			Self::Keyboard => PROMPT_KEY_DROP,
		}
	}

	pub fn edit_icon(self) -> &'static str {
		match self {
			Self::Pad => PROMPT_PAD_EDIT,
			Self::Keyboard => PROMPT_KEY_EDIT,
		}
	}

	pub fn delete_icon(self) -> &'static str {
		match self {
			Self::Pad => PROMPT_PAD_DELETE,
			Self::Keyboard => PROMPT_KEY_DELETE,
		}
	}
}

/// Which inventory action a chip illustrates.
///
/// Equip (A / Enter) equips from the lower pane or enters reorder on the
/// equipped row. Unequip is a separate Y action.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryPromptKind {
	Equip,
	Unequip,
	Drop,
}

impl InventoryPromptKind {
	pub fn label(self) -> &'static str {
		match self {
			Self::Equip => "Equip",
			Self::Unequip => "Unequip",
			Self::Drop => "Drop",
		}
	}

	pub fn icon(self, device: InputPromptDevice) -> &'static str {
		match self {
			Self::Equip => device.select_icon(),
			Self::Unequip => match device {
				InputPromptDevice::Pad => PROMPT_PAD_UNEQUIP,
				InputPromptDevice::Keyboard => PROMPT_KEY_UNEQUIP,
			},
			Self::Drop => device.drop_icon(),
		}
	}
}

const INVENTORY_PROMPT_KINDS: [InventoryPromptKind; 3] =
	[InventoryPromptKind::Equip, InventoryPromptKind::Unequip, InventoryPromptKind::Drop];

/// Image on an inventory action chip so a device change can swap the glyph.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryPromptGlyph(pub InventoryPromptKind);

/// Character-gallery footer actions. Select / Back always apply; Edit / Delete
/// need a saved character under the cursor.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GalleryPromptKind {
	Select,
	Back,
	Edit,
	Delete,
}

impl GalleryPromptKind {
	pub const ALL: [Self; 4] = [Self::Select, Self::Back, Self::Edit, Self::Delete];

	pub fn label(self) -> &'static str {
		match self {
			Self::Select => "Select",
			Self::Back => "Back",
			Self::Edit => "Edit",
			Self::Delete => "Delete",
		}
	}

	pub fn icon(self, device: InputPromptDevice) -> &'static str {
		match self {
			Self::Select => device.select_icon(),
			Self::Back => device.back_icon(),
			Self::Edit => device.edit_icon(),
			Self::Delete => device.delete_icon(),
		}
	}

	pub fn needs_character(self) -> bool {
		matches!(self, Self::Edit | Self::Delete)
	}
}

/// Image on a gallery action chip so a device change can swap the glyph.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GalleryPromptGlyph(pub GalleryPromptKind);

/// Footer row of gallery Select / Back / Edit / Delete chips.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct GalleryActionPrompts;

/// Drop chip row. Hosts hide this when the focused bag item is undroppable.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InventoryDropPromptChip;

/// Footer row under the interactive inventory scroll pane.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct InventoryActionPrompts;

pub fn spawn_inventory_action_prompts(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	assets: &AssetServer,
	device: InputPromptDevice,
) {
	parent
		.spawn((
			InventoryActionPrompts,
			Node {
				width: Val::Percent(100.0),
				min_width: Val::Px(0.0),
				flex_direction: FlexDirection::Row,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				column_gap: Val::Px(18.0),
				padding: UiRect::vertical(Val::Px(2.0)),
				flex_shrink: 0.0,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			for kind in INVENTORY_PROMPT_KINDS {
				spawn_prompt_chip(row, fonts, assets, device, kind);
			}
		});
}

fn spawn_prompt_chip(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	assets: &AssetServer,
	device: InputPromptDevice,
	kind: InventoryPromptKind,
) {
	let mut chip = parent.spawn((
		Node {
			flex_direction: FlexDirection::Row,
			align_items: AlignItems::Center,
			column_gap: Val::Px(8.0),
			..default()
		},
		Pickable::IGNORE,
	));
	if kind == InventoryPromptKind::Drop {
		chip.insert(InventoryDropPromptChip);
	}
	chip.with_children(|chip| {
		chip.spawn((
			InventoryPromptGlyph(kind),
			ImageNode::new(assets.load(kind.icon(device))),
			Node { width: Val::Px(28.0), height: Val::Px(28.0), flex_shrink: 0.0, ..default() },
			Pickable::IGNORE,
		));
		chip.spawn((
			Text::new(kind.label()),
			fonts.item(PANEL_GROUP_FONT_SIZE),
			TextColor(TEXT_YELLOW),
			Pickable::IGNORE,
		));
	});
}

pub fn sync_input_prompt_device(
	keys: Res<ButtonInput<KeyCode>>,
	gamepads: Query<&Gamepad>,
	mut device: ResMut<InputPromptDevice>,
) {
	let key_used = keys.get_just_pressed().next().is_some();
	let pad_used = gamepads.iter().any(gamepad_just_pressed);
	if pad_used {
		*device = InputPromptDevice::Pad;
	} else if key_used {
		*device = InputPromptDevice::Keyboard;
	}
}

fn gamepad_just_pressed(gamepad: &Gamepad) -> bool {
	[
		GamepadButton::South,
		GamepadButton::East,
		GamepadButton::West,
		GamepadButton::North,
		GamepadButton::Start,
		GamepadButton::Select,
	]
	.into_iter()
	.any(|button| gamepad.just_pressed(button))
}

pub fn sync_inventory_prompt_glyphs(
	device: Res<InputPromptDevice>,
	assets: Res<AssetServer>,
	mut inventory: Query<(&InventoryPromptGlyph, &mut ImageNode)>,
	mut gallery: Query<(&GalleryPromptGlyph, &mut ImageNode), Without<InventoryPromptGlyph>>,
) {
	for (glyph, mut image) in &mut inventory {
		set_prompt_icon(&assets, &mut image, glyph.0.icon(*device));
	}
	for (glyph, mut image) in &mut gallery {
		set_prompt_icon(&assets, &mut image, glyph.0.icon(*device));
	}
}

fn set_prompt_icon(assets: &AssetServer, image: &mut ImageNode, path: &'static str) {
	let handle = assets.load(path);
	if image.image != handle {
		image.image = handle;
	}
}

const GALLERY_PROMPT_ICON_PX: f32 = 36.0;

/// Bottom-center gallery keybind row. Pointer clicks the chips; pad / keyboard
/// follow the same bindings.
pub fn gallery_action_prompts_scene(device: InputPromptDevice) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = GalleryPromptKind::ALL
		.into_iter()
		.map(|kind| Box::new(gallery_prompt_chip_scene(kind, device)) as Box<dyn Scene>)
		.collect();
	bsn! {
		GalleryActionPrompts
		Node {
			position_type: PositionType::Absolute,
			left: px(0.0),
			right: px(0.0),
			bottom: px(CORNER_BOTTOM),
			width: percent(100),
			flex_direction: FlexDirection::Row,
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			column_gap: px(28.0),
		}
		Pickable::IGNORE
		Children [ {children} ]
	}
}

fn gallery_prompt_chip_scene(
	kind: GalleryPromptKind,
	device: InputPromptDevice,
) -> impl Scene + 'static {
	let label = String::from(kind.label());
	let icon = kind.icon(device);
	let visibility = Visibility::Inherited;
	let children: Vec<Box<dyn Scene>> = vec![
		Box::new(bsn! {
			~{GalleryPromptGlyph(kind)}
			ImageNode {
				image: icon,
				color: TEXT_YELLOW,
			}
			Node {
				width: px(GALLERY_PROMPT_ICON_PX),
				height: px(GALLERY_PROMPT_ICON_PX),
				flex_shrink: 0.0,
			}
			Pickable::IGNORE
		}),
		Box::new(bsn! {
			~{Text::new(label)}
			TextFont {
				font: FontSourceTemplate::Handle(BARLOW_SEMIBOLD),
				font_size: px(PANEL_BLOCK_FONT_SIZE),
			}
			TextColor(TEXT_YELLOW)
			TextLayout::new(Justify::Left, bevy::text::LineBreak::NoWrap)
			Pickable::IGNORE
		}),
	];
	bsn! {
		Button
		~{bevy::picking::hover::Hovered::default()}
		~{kind}
		~{visibility}
		Node {
			flex_direction: FlexDirection::Row,
			align_items: AlignItems::Center,
			column_gap: px(8.0),
			padding: UiRect::axes(px(4.0), px(2.0)),
		}
		BackgroundColor(Color::NONE)
		Children [ {children} ]
	}
}

#[cfg(test)]
mod tests {
	use super::{
		GalleryPromptKind, InputPromptDevice, InventoryPromptKind, INVENTORY_PROMPT_KINDS,
	};
	use crate::theme::{
		PROMPT_KEY_BACK, PROMPT_KEY_DELETE, PROMPT_KEY_DROP, PROMPT_KEY_EDIT, PROMPT_KEY_SELECT,
		PROMPT_KEY_UNEQUIP, PROMPT_PAD_BACK, PROMPT_PAD_DELETE, PROMPT_PAD_DROP, PROMPT_PAD_EDIT,
		PROMPT_PAD_SELECT, PROMPT_PAD_UNEQUIP,
	};
	use crate::{
		GALLERY_DELETE_KEYS, GALLERY_DELETE_PAD, GALLERY_EDIT_KEYS, GALLERY_EDIT_PAD,
		INVENTORY_DROP_KEYS, INVENTORY_DROP_PAD, INVENTORY_UNEQUIP_KEYS, INVENTORY_UNEQUIP_PAD,
		MENU_BACK_KEYS, MENU_BACK_PAD, MENU_SELECT_KEYS, MENU_SELECT_PAD,
	};
	use bevy::prelude::KeyCode;
	use maybraid_virtual_pad_controller::PadButton;

	#[test]
	fn select_glyphs_follow_the_menu_select_binding() {
		assert_eq!(MENU_SELECT_PAD, [PadButton::A, PadButton::Start]);
		assert_eq!(MENU_SELECT_KEYS, [KeyCode::Enter]);
		assert_eq!(InputPromptDevice::Pad.select_icon(), PROMPT_PAD_SELECT);
		assert_eq!(InputPromptDevice::Keyboard.select_icon(), PROMPT_KEY_SELECT);
		assert_eq!(InventoryPromptKind::Equip.label(), "Equip");
		assert_eq!(InventoryPromptKind::Equip.icon(InputPromptDevice::Pad), PROMPT_PAD_SELECT);
		assert_eq!(InventoryPromptKind::Equip.icon(InputPromptDevice::Keyboard), PROMPT_KEY_SELECT);
		assert_eq!(
			INVENTORY_PROMPT_KINDS,
			[InventoryPromptKind::Equip, InventoryPromptKind::Unequip, InventoryPromptKind::Drop]
		);
	}

	#[test]
	fn unequip_glyphs_follow_the_inventory_unequip_binding() {
		assert_eq!(INVENTORY_UNEQUIP_PAD, PadButton::Y);
		assert_eq!(INVENTORY_UNEQUIP_KEYS, [KeyCode::KeyY]);
		assert_eq!(InventoryPromptKind::Unequip.label(), "Unequip");
		assert_eq!(InventoryPromptKind::Unequip.icon(InputPromptDevice::Pad), PROMPT_PAD_UNEQUIP);
		assert_eq!(
			InventoryPromptKind::Unequip.icon(InputPromptDevice::Keyboard),
			PROMPT_KEY_UNEQUIP
		);
	}

	#[test]
	fn drop_glyphs_follow_the_inventory_drop_binding() {
		assert_eq!(INVENTORY_DROP_PAD, PadButton::X);
		assert_eq!(INVENTORY_DROP_KEYS, [KeyCode::KeyE, KeyCode::KeyX]);
		assert_eq!(InputPromptDevice::Pad.drop_icon(), PROMPT_PAD_DROP);
		assert_eq!(InputPromptDevice::Keyboard.drop_icon(), PROMPT_KEY_DROP);
		assert_eq!(InventoryPromptKind::Drop.icon(InputPromptDevice::Pad), PROMPT_PAD_DROP);
	}

	#[test]
	fn gallery_glyphs_follow_select_back_edit_and_delete_bindings() {
		assert_eq!(MENU_SELECT_PAD, [PadButton::A, PadButton::Start]);
		assert_eq!(MENU_SELECT_KEYS, [KeyCode::Enter]);
		assert_eq!(MENU_BACK_PAD, PadButton::B);
		assert_eq!(MENU_BACK_KEYS, [KeyCode::Escape]);
		assert_eq!(GALLERY_EDIT_PAD, PadButton::X);
		assert_eq!(GALLERY_EDIT_KEYS, [KeyCode::KeyE, KeyCode::KeyX]);
		assert_eq!(GALLERY_DELETE_PAD, PadButton::Select);
		assert_eq!(GALLERY_DELETE_KEYS, [KeyCode::Delete]);
		assert_eq!(
			GalleryPromptKind::ALL,
			[
				GalleryPromptKind::Select,
				GalleryPromptKind::Back,
				GalleryPromptKind::Edit,
				GalleryPromptKind::Delete
			]
		);
		assert_eq!(GalleryPromptKind::Select.icon(InputPromptDevice::Pad), PROMPT_PAD_SELECT);
		assert_eq!(GalleryPromptKind::Select.icon(InputPromptDevice::Keyboard), PROMPT_KEY_SELECT);
		assert_eq!(GalleryPromptKind::Back.icon(InputPromptDevice::Pad), PROMPT_PAD_BACK);
		assert_eq!(GalleryPromptKind::Back.icon(InputPromptDevice::Keyboard), PROMPT_KEY_BACK);
		assert_eq!(GalleryPromptKind::Edit.icon(InputPromptDevice::Pad), PROMPT_PAD_EDIT);
		assert_eq!(GalleryPromptKind::Edit.icon(InputPromptDevice::Keyboard), PROMPT_KEY_EDIT);
		assert_eq!(GalleryPromptKind::Delete.icon(InputPromptDevice::Pad), PROMPT_PAD_DELETE);
		assert_eq!(GalleryPromptKind::Delete.icon(InputPromptDevice::Keyboard), PROMPT_KEY_DELETE);
		assert!(GalleryPromptKind::Edit.needs_character());
		assert!(GalleryPromptKind::Delete.needs_character());
		assert!(!GalleryPromptKind::Select.needs_character());
		assert!(!GalleryPromptKind::Back.needs_character());
	}
}
