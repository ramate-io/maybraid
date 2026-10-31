//! Full-screen inventory presentation. Generic overlays and the read-only
//! Starter Kit continue to use the shared `MenuNode` sink.

use bevy::prelude::*;
use character_ui_menu::{
	EquipmentChoice, GridCatalogChoice, InventoryDrawerMode, InventoryRegion, InventorySection,
	MenuNode,
};
use menu_components::{
	menu_region_wash, spawn_header_line, spawn_hud_text, spawn_inventory_action_prompts,
	spawn_text_button, HoverTile, HudFonts, HudMenu, HudMenuItem, HudOverlayMenu, HudScrollManual,
	HudScrollViewport, InputPromptDevice, InventoryHudMenu, TileWrap, PANEL_HEADER_FONT_SIZE,
	PANEL_ITEM_FONT_SIZE, PROMPT_PAD_NEXT_SECTION, PROMPT_PAD_PREVIOUS_SECTION,
	PROMPT_PAD_TOGGLE_DRAWER, TEXT_DEEP_BLUE, TEXT_LIME, TEXT_YELLOW, TEXT_YELLOW_FAINT,
};

use crate::sink::MenuThumbnailContext;
use crate::widgets::{
	CloseOverlaySelect, EquippedOrderControls, InventoryControl, InventoryItemKey,
	InventoryMenuSession, MenuButton,
};

/// Region identity stamped on the independent Drawer and Equipped focus lists.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryScreenArea(pub InventoryRegion);

/// Catalog identity used by inventory navigation. Ordinals follow catalog order.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryTile {
	pub section: InventorySection,
	pub region: InventoryRegion,
	pub ordinal: usize,
}

/// AIOP slot identity, including empty capacity placeholders.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryAIOPSlot {
	pub section: InventorySection,
	pub ordinal: usize,
	pub capacity: usize,
}

/// Rotary item position. Cards remain alive while focus moves.
#[derive(Component, Clone, Copy, Debug)]
pub struct InventoryCarouselCard {
	pub menu: Entity,
	pub ordinal: usize,
	pub count: usize,
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct InventoryCarouselLabel {
	menu: Entity,
	ordinal: usize,
}

#[derive(Component)]
pub(crate) struct InventoryCarouselCaption;

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct InventoryCarouselMotion {
	angle: f32,
	scale: f32,
}

/// Host attachment point for the existing live character preview camera.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct InventoryCharacterPreviewSlot;

/// Handles returned to the host. Menu indices are Drawer at 0, Equipped at 1.
#[derive(Clone, Copy, Debug)]
pub struct InventoryScreenSlots {
	pub preview: Entity,
	pub menus: [Entity; 2],
	pub item_count: [usize; 2],
	pub focused_index: [Option<usize>; 2],
}

pub const INVENTORY_GALLERY_COLUMNS: usize = 5;
pub const INVENTORY_GALLERY_ROW_HEIGHT: f32 = 74.0;
const GALLERY_ROW_GAP: f32 = 4.0;
pub const INVENTORY_GALLERY_VISIBLE_ROWS: usize = 2;
const GALLERY_VIEWPORT_HEIGHT: f32 = INVENTORY_GALLERY_ROW_HEIGHT
	* INVENTORY_GALLERY_VISIBLE_ROWS as f32
	+ GALLERY_ROW_GAP * (INVENTORY_GALLERY_VISIBLE_ROWS - 1) as f32;
pub const INVENTORY_GALLERY_ROW_STEP: f32 = INVENTORY_GALLERY_ROW_HEIGHT + GALLERY_ROW_GAP;
const GALLERY_PANEL_HEIGHT: f32 = 196.0;

/// Rebuild the inventory shell and return its preview slot and two focus roots.
pub fn fill_inventory_screen<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
	commands: &mut Commands,
	viewport: Entity,
	fonts: &HudFonts,
	assets: &AssetServer,
	device: InputPromptDevice,
	node: &MenuNode<E>,
	interactive: bool,
	session: &InventoryMenuSession,
	thumbnails: &mut C,
) -> InventoryScreenSlots {
	let mut slots = InventoryScreenSlots {
		preview: Entity::PLACEHOLDER,
		menus: [Entity::PLACEHOLDER; 2],
		item_count: [0; 2],
		focused_index: [None; 2],
	};
	let Some((choices, equipment, capacity)) = section_data(node, session.section) else {
		return slots;
	};
	let Ok(mut root) = commands.get_entity(viewport) else {
		return slots;
	};
	root.despawn_related::<Children>();
	root.insert(crate::widgets::OverlaySelectViewport);
	let drawer_memory =
		session.section_memory[session.section.index()][InventoryRegion::Drawer.index()];
	let equipped_memory =
		session.section_memory[session.section.index()][InventoryRegion::Equipped.index()];
	let drawer_focus = drawer_memory
		.focused_item
		.filter(|key| key.region == InventoryRegion::Drawer)
		.and_then(|key| choices.iter().position(|choice| choice.item_key == Some(key.index)))
		.unwrap_or(0);
	let equipped_focus = equipped_memory
		.focused_item
		.filter(|key| key.region == InventoryRegion::Equipped)
		.and_then(|key| equipment.iter().position(|choice| choice.item_key == key.index))
		.unwrap_or(0);
	let aiop_focus_capacity = if interactive { capacity } else { equipment.len().min(capacity) };
	root.with_children(|host_viewport| {
		let mut shell = host_viewport.spawn((
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				min_width: Val::Px(0.0),
				min_height: Val::Px(0.0),
				flex_direction: FlexDirection::Column,
				position_type: PositionType::Relative,
				row_gap: Val::Px(8.0),
				padding: UiRect::all(Val::Px(12.0)),
				overflow: Overflow::clip(),
				..default()
			},
			Pickable::IGNORE,
		));
		shell.with_children(|screen| {
			let mut main = screen.spawn(Node {
				width: Val::Percent(100.0),
				min_height: Val::Px(0.0),
				flex_grow: 1.0,
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(12.0),
				..default()
			});
			main.with_children(|main| {
				let mut aiop = main.spawn((
					Node {
						width: Val::Percent(26.0),
						min_width: Val::Px(150.0),
						min_height: Val::Px(0.0),
						flex_direction: FlexDirection::Column,
						row_gap: Val::Px(8.0),
						padding: UiRect::all(Val::Px(12.0)),
						border_radius: BorderRadius::all(Val::Px(16.0)),
						overflow: Overflow::clip(),
						..default()
					},
					BackgroundColor(menu_region_wash()),
					Pickable::IGNORE,
				));
				slots.item_count[1] = aiop_focus_capacity;
				slots.focused_index[1] = bounded_focus(equipped_focus, slots.item_count[1]);
				aiop.with_children(|aiop| {
					spawn_header_line(
						aiop,
						fonts,
						"Equipped Items",
						Some(&format!("{}/{}", equipment.len().min(capacity), capacity)),
						PANEL_ITEM_FONT_SIZE,
						TEXT_YELLOW,
					);
					let mut list_root = aiop.spawn((
						InventoryScreenArea(InventoryRegion::Equipped),
						InventoryHudMenu,
						HudMenu {
							selected: equipped_focus.min(equipment.len().saturating_sub(1)),
							item_count: aiop_focus_capacity,
						},
						HudScrollViewport,
						ScrollPosition::default(),
						Node {
							width: Val::Percent(100.0),
							min_height: Val::Px(0.0),
							flex_grow: 1.0,
							flex_direction: FlexDirection::Column,
							row_gap: Val::Px(6.0),
							overflow: Overflow::scroll_y(),
							..default()
						},
						Pickable::default(),
					));
					if session.region == InventoryRegion::Equipped {
						list_root.insert(HudOverlayMenu);
					}
					slots.menus[1] = list_root.id();
					list_root.with_children(|list| {
						for ordinal in 0..capacity {
							if let Some(choice) = equipment.get(ordinal) {
								spawn_equipped_slot(
									list,
									fonts,
									choice,
									session.section,
									ordinal,
									aiop_focus_capacity,
									interactive,
									thumbnails,
									slots.menus[1],
								);
							} else {
								// Keep empty AIOP capacity reachable so its plus action can jump to the drawer.
								spawn_empty_slot(
									list,
									fonts,
									session.section,
									ordinal,
									capacity,
									slots.menus[1],
									interactive,
								);
							}
						}
					});
				});

				let mut center = main.spawn(Node {
					width: Val::Percent(74.0),
					min_width: Val::Px(0.0),
					min_height: Val::Px(0.0),
					flex_grow: 1.0,
					flex_direction: FlexDirection::Column,
					row_gap: Val::Px(8.0),
					..default()
				});
				center.with_children(|right| {
					spawn_inventory_header(right, fonts, assets, session.section, session.drawer);
					let preview_node = if interactive {
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(62.0),
							min_height: Val::Px(80.0),
							flex_grow: 1.0,
							position_type: PositionType::Relative,
							..default()
						}
					} else {
						// Starter Kit has no live character to preview. Reclaim this
						// space so the fixed two-row drawer and Continue footer fit.
						Node { display: Display::None, ..default() }
					};
					slots.preview = right
						.spawn((InventoryCharacterPreviewSlot, preview_node, Pickable::IGNORE))
						.id();
					let mut drawer = right.spawn((
						Node {
							width: Val::Percent(100.0),
							height: Val::Px(match session.drawer {
								InventoryDrawerMode::Rotary => 230.0,
								InventoryDrawerMode::Gallery => GALLERY_PANEL_HEIGHT,
							}),
							min_height: Val::Px(match session.drawer {
								InventoryDrawerMode::Rotary => 230.0,
								InventoryDrawerMode::Gallery => GALLERY_PANEL_HEIGHT,
							}),
							flex_shrink: 0.0,
							flex_direction: FlexDirection::Column,
							row_gap: Val::Px(4.0),
							padding: UiRect::all(Val::Px(6.0)),
							border_radius: BorderRadius::all(Val::Px(16.0)),
							overflow: Overflow::clip(),
							..default()
						},
						BackgroundColor(menu_components::menu_region_wash()),
						Pickable::IGNORE,
					));
					slots.item_count[0] = choices.len();
					slots.focused_index[0] = bounded_focus(drawer_focus, choices.len());
					drawer.with_children(|drawer| {
						spawn_header_line(
							drawer,
							fonts,
							"Pick Items",
							Some(&format!("{} owned", choices.len())),
							PANEL_ITEM_FONT_SIZE,
							TEXT_YELLOW,
						);
						slots.menus[0] = match session.drawer {
							InventoryDrawerMode::Rotary => spawn_rotary_carousel(
								drawer,
								fonts,
								&choices,
								session.section,
								drawer_focus,
								interactive,
								thumbnails,
								session.region == InventoryRegion::Drawer,
							),
							InventoryDrawerMode::Gallery => spawn_gallery(
								drawer,
								fonts,
								&choices,
								session.section,
								drawer_focus,
								interactive,
								thumbnails,
								session.region == InventoryRegion::Drawer,
								drawer_memory.gallery_scroll,
							),
						};
					});
					spawn_inventory_action_footer(right, fonts, assets, device, interactive);
				});
			});
			spawn_inventory_back_control(screen, fonts);
		});
	});
	slots
}

fn spawn_inventory_header(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	assets: &AssetServer,
	section: InventorySection,
	drawer: InventoryDrawerMode,
) {
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				height: Val::Px(66.0),
				min_height: Val::Px(66.0),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(2.0),
				align_items: AlignItems::Center,
				justify_content: JustifyContent::Center,
				flex_shrink: 0.0,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|header| {
			header
				.spawn((
					Node {
						width: Val::Percent(100.0),
						height: Val::Px(40.0),
						flex_direction: FlexDirection::Row,
						align_items: AlignItems::Center,
						justify_content: JustifyContent::Center,
						column_gap: Val::Px(18.0),
						..default()
					},
					Pickable::IGNORE,
				))
				.with_children(|title_row| {
					spawn_inventory_icon_button(
						title_row,
						assets,
						PROMPT_PAD_PREVIOUS_SECTION,
						InventoryControl(
							character_ui_menu::InventoryControlAction::PreviousSection,
						),
					);
					spawn_header_line(
						title_row,
						fonts,
						section.label(),
						None,
						PANEL_HEADER_FONT_SIZE,
						TEXT_YELLOW,
					);
					spawn_inventory_icon_button(
						title_row,
						assets,
						PROMPT_PAD_NEXT_SECTION,
						InventoryControl(character_ui_menu::InventoryControlAction::NextSection),
					);
				});
			header
				.spawn((
					Button,
					InventoryControl(character_ui_menu::InventoryControlAction::ToggleDrawer),
					Node {
						flex_direction: FlexDirection::Row,
						align_items: AlignItems::Center,
						justify_content: JustifyContent::Center,
						column_gap: Val::Px(6.0),
						..default()
					},
					BackgroundColor(Color::NONE),
					Pickable::default(),
				))
				.with_children(|mode| {
					mode.spawn((
						ImageNode::new(assets.load(PROMPT_PAD_TOGGLE_DRAWER)),
						Node { width: Val::Px(21.0), height: Val::Px(21.0), ..default() },
						Pickable::IGNORE,
					));
					let label = match drawer {
						InventoryDrawerMode::Rotary => "Switch to Gallery",
						InventoryDrawerMode::Gallery => "Switch to Rotary",
					};
					mode.spawn((
						Text::new(label),
						fonts.item(15.0),
						TextColor(TEXT_YELLOW_FAINT),
						Pickable::IGNORE,
					));
				});
		});
}

fn spawn_inventory_icon_button(
	parent: &mut ChildSpawnerCommands,
	assets: &AssetServer,
	icon: &'static str,
	action: InventoryControl,
) {
	parent
		.spawn((
			Button,
			action,
			Node { width: Val::Px(40.0), height: Val::Px(36.0), ..default() },
			BackgroundColor(Color::NONE),
			Pickable::default(),
		))
		.with_children(|button| {
			button.spawn((
				ImageNode::new(assets.load(icon)),
				Node { width: Val::Px(32.0), height: Val::Px(32.0), ..default() },
				Pickable::IGNORE,
			));
		});
}

fn spawn_inventory_action_footer(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	assets: &AssetServer,
	device: InputPromptDevice,
	interactive: bool,
) {
	if !interactive {
		return;
	}
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				min_height: Val::Px(28.0),
				flex_direction: FlexDirection::Row,
				align_items: AlignItems::Center,
				justify_content: JustifyContent::Center,
				flex_shrink: 0.0,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|footer| spawn_inventory_action_prompts(footer, fonts, assets, device));
}

fn spawn_inventory_back_control(parent: &mut ChildSpawnerCommands, fonts: &HudFonts) {
	parent
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				bottom: Val::Px(0.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|back| {
			spawn_text_button(back, fonts, "Back", CloseOverlaySelect);
		});
}

fn spawn_rotary_carousel<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	choices: &[GridCatalogChoice<E>],
	section: InventorySection,
	focused: usize,
	interactive: bool,
	thumbnails: &mut C,
	active: bool,
) -> Entity {
	if choices.is_empty() {
		return spawn_empty_drawer(parent, fonts, active);
	}
	let mut carousel_entity = parent.spawn((
		InventoryScreenArea(InventoryRegion::Drawer),
		InventoryHudMenu,
		HudMenu { selected: focused.min(choices.len() - 1), item_count: choices.len() },
		Node {
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			min_height: Val::Px(96.0),
			position_type: PositionType::Relative,
			overflow: Overflow::clip(),
			..default()
		},
		Pickable::IGNORE,
	));
	if active {
		carousel_entity.insert(HudOverlayMenu);
	}
	let carousel_id = carousel_entity.id();
	carousel_entity.with_children(|carousel| {
		carousel.spawn((
			Node {
				position_type: PositionType::Absolute,
				left: Val::Percent(50.0),
				top: Val::Percent(50.0),
				width: Val::Px(560.0),
				height: Val::Auto,
				aspect_ratio: Some(1.0),
				margin: UiRect { left: Val::Px(-280.0), top: Val::Px(-280.0), ..default() },
				border: UiRect::all(Val::Px(1.0)),
				border_radius: BorderRadius::MAX,
				..default()
			},
			UiTransform {
				translation: Val2::px(0.0, 58.0),
				scale: Vec2::new(1.0, 30.0 / 560.0),
				..default()
			},
			BackgroundColor(Color::srgba(0.35, 0.67, 0.88, 0.72)),
			BorderColor::all(Color::srgb(0.74, 0.87, 0.96)),
			ZIndex(1),
			Pickable::IGNORE,
		));
		for (ordinal, choice) in choices.iter().enumerate() {
			let key = choice.item_key.map(|index| InventoryItemKey {
				index,
				equipped: choice.selected,
				region: InventoryRegion::Drawer,
			});
			let image = catalog_image(choice, thumbnails);
			let mut card = carousel.spawn((
				InventoryCarouselCard { menu: carousel_id, ordinal, count: choices.len() },
				InventoryCarouselMotion {
					angle: rotary_angle(ordinal, focused, choices.len()),
					scale: rotary_scale(ordinal, focused, choices.len()),
				},
				Node {
					position_type: PositionType::Absolute,
					left: Val::Percent(50.0),
					top: Val::Percent(50.0),
					width: Val::Px(164.0),
					height: Val::Px(122.0),
					margin: UiRect { left: Val::Px(-82.0), top: Val::Px(-61.0), ..default() },
					..default()
				},
				UiTransform {
					translation: Val2::px(
						rotary_x(ordinal, focused, choices.len()),
						rotary_y(ordinal, focused, choices.len()),
					),
					scale: Vec2::splat(rotary_scale(ordinal, focused, choices.len())),
					..default()
				},
				ZIndex((100.0 + rotary_angle(ordinal, focused, choices.len()).cos() * 50.0) as i32),
				Pickable::IGNORE,
			));
			card.with_children(|card| {
				let base = (
					InventoryTile { section, region: InventoryRegion::Drawer, ordinal },
					HudMenuItem { index: ordinal, menu: carousel_id },
				);
				spawn_rotary_tile(
					card,
					fonts,
					choice,
					image,
					base,
					key,
					choice.event.filter(|_| interactive),
					carousel_id,
					ordinal,
				);
			});
		}
	});
	carousel_id
}

fn spawn_gallery<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	choices: &[GridCatalogChoice<E>],
	section: InventorySection,
	focused: usize,
	interactive: bool,
	thumbnails: &mut C,
	active: bool,
	saved_scroll_rows: usize,
) -> Entity {
	if choices.is_empty() {
		return spawn_empty_drawer(parent, fonts, active);
	}
	let mut scroll = parent.spawn((
		InventoryScreenArea(InventoryRegion::Drawer),
		InventoryHudMenu,
		HudMenu { selected: focused.min(choices.len() - 1), item_count: choices.len() },
		HudScrollManual,
		HudScrollViewport,
		ScrollPosition(Vec2::new(0.0, saved_scroll_rows as f32 * INVENTORY_GALLERY_ROW_STEP)),
		Node {
			width: Val::Percent(100.0),
			height: Val::Px(GALLERY_VIEWPORT_HEIGHT),
			min_height: Val::Px(GALLERY_VIEWPORT_HEIGHT),
			flex_shrink: 0.0,
			overflow: Overflow::scroll_y(),
			scrollbar_width: 6.0,
			..default()
		},
		Pickable::default(),
	));
	if active {
		scroll.insert(HudOverlayMenu);
	}
	let menu = scroll.id();
	let rows = choices.len().div_ceil(INVENTORY_GALLERY_COLUMNS);
	let content_height = gallery_content_height(rows);
	scroll.with_children(|viewport| {
		viewport
			.spawn(Node {
				width: Val::Percent(100.0),
				min_height: Val::Px(content_height),
				flex_shrink: 0.0,
				display: Display::Grid,
				grid_template_columns: vec![GridTrack::flex(1.0); INVENTORY_GALLERY_COLUMNS],
				grid_auto_rows: vec![GridTrack::px(INVENTORY_GALLERY_ROW_HEIGHT)],
				column_gap: Val::Px(8.0),
				row_gap: Val::Px(GALLERY_ROW_GAP),
				..default()
			})
			.with_children(|grid| {
				for (ordinal, choice) in choices.iter().enumerate() {
					let key = choice.item_key.map(|index| InventoryItemKey {
						index,
						equipped: choice.selected,
						region: InventoryRegion::Drawer,
					});
					let image = catalog_image(choice, thumbnails);
					let base = (
						InventoryTile { section, region: InventoryRegion::Drawer, ordinal },
						HudMenuItem { index: ordinal, menu },
					);
					spawn_compact_gallery_tile(
						grid,
						fonts,
						choice,
						image,
						base,
						key,
						choice.event.filter(|_| interactive),
					);
				}
			});
	});
	menu
}

/// A drawn check badge avoids relying on a font glyph missing from some HUD
/// typefaces.
fn spawn_equipped_check(parent: &mut ChildSpawnerCommands) {
	parent
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(4.0),
				right: Val::Px(4.0),
				width: Val::Px(18.0),
				height: Val::Px(18.0),
				border_radius: BorderRadius::MAX,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(TEXT_LIME),
			ZIndex(10),
			Pickable::IGNORE,
		))
		.with_children(|badge| {
			badge.spawn((
				Node {
					position_type: PositionType::Absolute,
					width: Val::Px(3.0),
					height: Val::Px(7.0),
					left: Val::Px(5.0),
					top: Val::Px(5.0),
					..default()
				},
				BackgroundColor(Color::WHITE),
				UiTransform { rotation: Rot2::radians(-0.65), ..default() },
				Pickable::IGNORE,
			));
			badge.spawn((
				Node {
					position_type: PositionType::Absolute,
					width: Val::Px(3.0),
					height: Val::Px(10.0),
					left: Val::Px(9.0),
					top: Val::Px(3.0),
					..default()
				},
				BackgroundColor(Color::WHITE),
				UiTransform { rotation: Rot2::radians(0.65), ..default() },
				Pickable::IGNORE,
			));
		});
}

fn spawn_rotary_tile<E: Copy + Send + Sync + 'static>(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	choice: &GridCatalogChoice<E>,
	image: (Option<Handle<Image>>, Option<Entity>, Color),
	base: impl Bundle,
	key: Option<InventoryItemKey>,
	event: Option<E>,
	menu: Entity,
	ordinal: usize,
) {
	let (thumbnail, viewport, tint) = image;
	match (key, event) {
		(Some(key), Some(event)) => spawn_rotary_button(
			parent,
			fonts,
			choice,
			thumbnail,
			viewport,
			tint,
			menu,
			ordinal,
			(base, key, MenuButton(event)),
		),
		(Some(key), None) => spawn_rotary_button(
			parent,
			fonts,
			choice,
			thumbnail,
			viewport,
			tint,
			menu,
			ordinal,
			(base, key),
		),
		(None, Some(event)) => spawn_rotary_button(
			parent,
			fonts,
			choice,
			thumbnail,
			viewport,
			tint,
			menu,
			ordinal,
			(base, MenuButton(event)),
		),
		(None, None) => spawn_rotary_button(
			parent, fonts, choice, thumbnail, viewport, tint, menu, ordinal, base,
		),
	}
}

fn spawn_rotary_button<E: Copy + Send + Sync + 'static>(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	choice: &GridCatalogChoice<E>,
	thumbnail: Option<Handle<Image>>,
	viewport: Option<Entity>,
	tint: Color,
	menu: Entity,
	ordinal: usize,
	extra: impl Bundle,
) {
	let face = if choice.selected { TEXT_YELLOW } else { TEXT_YELLOW_FAINT };
	parent
		.spawn((
			Button,
			extra,
			Node {
				width: Val::Percent(100.0),
				height: Val::Px(122.0),
				padding: UiRect::all(Val::Px(6.0)),
				flex_direction: FlexDirection::Column,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				row_gap: Val::Px(3.0),
				..default()
			},
		))
		.with_children(|tile| {
			let mut art = tile.spawn((
				Node {
					width: Val::Px(68.0),
					height: Val::Px(68.0),
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
					position_type: PositionType::Relative,
					..default()
				},
				Pickable::IGNORE,
			));
			if let Some(camera) = viewport {
				art.with_children(|art| {
					art.spawn((
						bevy::ui::widget::ViewportNode::new(camera),
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							..default()
						},
						Pickable::IGNORE,
					));
				});
			} else if let Some(image) = thumbnail {
				art.with_children(|art| {
					art.spawn((
						ImageNode { image, color: tint, ..default() },
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							..default()
						},
						Pickable::IGNORE,
					));
				});
			} else if choice.path.is_empty() {
				art.insert(BackgroundColor(inventory_color(choice.preview)));
			}
			tile.spawn((
				InventoryCarouselLabel { menu, ordinal },
				Node {
					width: Val::Percent(100.0),
					padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
					border_radius: BorderRadius::all(Val::Px(4.0)),
					flex_direction: FlexDirection::Column,
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
					row_gap: Val::Px(1.0),
					..default()
				},
				BackgroundColor(Color::NONE),
				Pickable::IGNORE,
			))
			.with_children(|label| {
				label.spawn((
					InventoryCarouselCaption,
					TileWrap,
					Text::new(choice.label.clone()),
					fonts.item(14.0),
					TextColor(face),
					TextLayout::new(
						bevy::text::Justify::Center,
						bevy::text::LineBreak::WordOrCharacter,
					),
					bevy::text::TextBounds::UNBOUNDED,
					Node { width: Val::Percent(100.0), ..default() },
					Pickable::IGNORE,
				));
				if !choice.detail.is_empty() {
					label.spawn((
						InventoryCarouselCaption,
						TileWrap,
						Text::new(choice.detail.clone()),
						fonts.item(11.0),
						TextColor(TEXT_YELLOW_FAINT),
						TextLayout::new(
							bevy::text::Justify::Center,
							bevy::text::LineBreak::WordOrCharacter,
						),
						bevy::text::TextBounds::UNBOUNDED,
						Node { width: Val::Percent(100.0), ..default() },
						Pickable::IGNORE,
					));
				}
			});
			if choice.selected {
				spawn_equipped_check(tile);
			}
		});
}

fn spawn_compact_gallery_tile<E: Copy + Send + Sync + 'static>(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	choice: &GridCatalogChoice<E>,
	image: (Option<Handle<Image>>, Option<Entity>, Color),
	base: impl Bundle,
	key: Option<InventoryItemKey>,
	event: Option<E>,
) {
	let (thumbnail, viewport, tint) = image;
	match (key, event) {
		(Some(key), Some(event)) => spawn_compact_gallery_button(
			parent,
			fonts,
			choice,
			thumbnail,
			viewport,
			tint,
			(base, key, MenuButton(event)),
		),
		(Some(key), None) => spawn_compact_gallery_button(
			parent,
			fonts,
			choice,
			thumbnail,
			viewport,
			tint,
			(base, key),
		),
		(None, Some(event)) => spawn_compact_gallery_button(
			parent,
			fonts,
			choice,
			thumbnail,
			viewport,
			tint,
			(base, MenuButton(event)),
		),
		(None, None) => {
			spawn_compact_gallery_button(parent, fonts, choice, thumbnail, viewport, tint, base)
		}
	}
}

fn spawn_compact_gallery_button<E: Copy + Send + Sync + 'static>(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	choice: &GridCatalogChoice<E>,
	thumbnail: Option<Handle<Image>>,
	viewport: Option<Entity>,
	tint: Color,
	extra: impl Bundle,
) {
	let face = if choice.selected { TEXT_YELLOW } else { TEXT_YELLOW_FAINT };
	parent
		.spawn((
			Button,
			HoverTile { equipped: choice.selected, preserve_fill: false, face },
			extra,
			Node {
				width: Val::Percent(100.0),
				height: Val::Px(INVENTORY_GALLERY_ROW_HEIGHT),
				min_height: Val::Px(INVENTORY_GALLERY_ROW_HEIGHT),
				flex_shrink: 0.0,
				min_width: Val::Px(0.0),
				padding: UiRect::all(Val::Px(3.0)),
				flex_direction: FlexDirection::Row,
				align_items: AlignItems::Center,
				column_gap: Val::Px(6.0),
				border: UiRect::all(Val::Px(1.0)),
				..default()
			},
			BorderColor::all(if choice.selected { TEXT_YELLOW } else { Color::NONE }),
			BackgroundColor(Color::NONE),
			Outline::new(Val::Px(2.0), Val::Px(2.0), Color::NONE),
		))
		.with_children(|tile| {
			let mut image_slot = tile.spawn((
				Node {
					width: Val::Px(36.0),
					height: Val::Px(36.0),
					min_width: Val::Px(36.0),
					min_height: Val::Px(36.0),
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
					position_type: PositionType::Relative,
					..default()
				},
				Pickable::IGNORE,
			));
			if let Some(camera) = viewport {
				image_slot.with_children(|slot| {
					slot.spawn((
						bevy::ui::widget::ViewportNode::new(camera),
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							..default()
						},
						Pickable::IGNORE,
					));
				});
			} else if let Some(image) = thumbnail {
				image_slot.with_children(|slot| {
					slot.spawn((
						ImageNode { image, color: tint, ..default() },
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							..default()
						},
						Pickable::IGNORE,
					));
				});
			} else if choice.path.is_empty() {
				image_slot.insert(BackgroundColor(inventory_color(choice.preview)));
			}
			tile.spawn((
				TileWrap,
				Text::new(choice.label.clone()),
				fonts.item(11.0),
				TextColor(face),
				TextLayout::new(bevy::text::Justify::Left, bevy::text::LineBreak::WordOrCharacter),
				bevy::text::TextBounds::UNBOUNDED,
				Node { flex_grow: 1.0, min_width: Val::Px(0.0), ..default() },
				Pickable::IGNORE,
			));
			if choice.selected {
				spawn_equipped_check(tile);
			}
		});
}

fn spawn_equipped_slot<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	choice: &EquipmentChoice<E>,
	section: InventorySection,
	ordinal: usize,
	capacity: usize,
	interactive: bool,
	thumbnails: &mut C,
	menu: Entity,
) {
	let tile = &choice.tile;
	let key = InventoryItemKey {
		index: choice.item_key,
		equipped: true,
		region: InventoryRegion::Equipped,
	};
	let image = catalog_image(tile, thumbnails);
	let base = (
		InventoryAIOPSlot { section, ordinal, capacity },
		InventoryTile { section, region: InventoryRegion::Equipped, ordinal },
		HudMenuItem { index: ordinal, menu },
		key,
		EquippedOrderControls {
			unequip: choice.unequip,
			move_left: choice.move_left,
			move_right: choice.move_right,
		},
	);
	let (thumbnail, viewport, tint) = image;
	let face = if tile.selected { TEXT_YELLOW } else { TEXT_YELLOW_FAINT };
	let mut row = parent.spawn((
		Button,
		HoverTile { equipped: tile.selected, preserve_fill: false, face },
		base,
		Node {
			width: Val::Percent(100.0),
			height: Val::Px(58.0),
			min_height: Val::Px(58.0),
			padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
			flex_direction: FlexDirection::Row,
			align_items: AlignItems::Center,
			column_gap: Val::Px(8.0),
			border: UiRect::all(Val::Px(1.0)),
			overflow: Overflow::clip(),
			..default()
		},
		BorderColor::all(if tile.selected { TEXT_YELLOW } else { Color::NONE }),
		BackgroundColor(Color::NONE),
		Outline::new(Val::Px(2.0), Val::Px(2.0), Color::NONE),
	));
	if interactive {
		if let Some(event) = tile.event {
			row.insert(MenuButton(event));
		}
	}
	row.with_children(|row| {
		let mut image_slot = row.spawn((
			Node {
				width: Val::Px(46.0),
				height: Val::Px(46.0),
				min_width: Val::Px(46.0),
				min_height: Val::Px(46.0),
				position_type: PositionType::Relative,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			Pickable::IGNORE,
		));
		if let Some(camera) = viewport {
			image_slot.with_children(|slot| {
				slot.spawn((
					bevy::ui::widget::ViewportNode::new(camera),
					Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
					Pickable::IGNORE,
				));
			});
		} else if let Some(image) = thumbnail {
			image_slot.with_children(|slot| {
				slot.spawn((
					ImageNode { image, color: tint, ..default() },
					Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
					Pickable::IGNORE,
				));
			});
		} else if tile.path.is_empty() {
			image_slot.insert(BackgroundColor(inventory_color(tile.preview)));
		}
		let label = if tile.detail.is_empty() {
			tile.label.clone()
		} else {
			format!("{}  ·  {}", tile.label, tile.detail)
		};
		row.spawn((
			TileWrap,
			Text::new(label),
			fonts.item(12.0),
			TextColor(face),
			TextLayout::new(bevy::text::Justify::Left, bevy::text::LineBreak::WordOrCharacter),
			bevy::text::TextBounds::UNBOUNDED,
			Node { flex_grow: 1.0, min_width: Val::Px(0.0), ..default() },
			Pickable::IGNORE,
		));
		if let Some(rank) = tile.rank {
			row.spawn((
				TileWrap,
				Text::new(rank.to_string()),
				fonts.item(12.0),
				TextColor(face),
				Pickable::IGNORE,
			));
		}
		spawn_equipped_check(row);
	});
}

fn spawn_empty_slot(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	section: InventorySection,
	ordinal: usize,
	capacity: usize,
	menu: Entity,
	interactive: bool,
) {
	let mut slot = parent.spawn((
		Node {
			width: Val::Percent(100.0),
			min_height: Val::Px(48.0),
			flex_direction: FlexDirection::Row,
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			border: UiRect::all(Val::Px(1.0)),
			border_radius: BorderRadius::all(Val::Px(8.0)),
			..default()
		},
		BorderColor::all(TEXT_YELLOW_FAINT),
		BackgroundColor(Color::NONE),
		Pickable::default(),
	));
	if interactive {
		slot.insert((
			Button,
			InventoryAIOPSlot { section, ordinal, capacity },
			HudMenuItem { index: ordinal, menu },
			InventoryControl(character_ui_menu::InventoryControlAction::FocusDrawer),
		));
	}
	slot.with_children(|slot| {
		spawn_hud_text(
			slot,
			fonts.item(PANEL_HEADER_FONT_SIZE),
			if interactive { "+" } else { "—" },
			TEXT_YELLOW_FAINT,
			bevy::text::Justify::Center,
		);
	});
}

fn spawn_empty_storage(parent: &mut ChildSpawnerCommands, fonts: &HudFonts) {
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_grow: 1.0,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|empty| {
			spawn_hud_text(
				empty,
				fonts.item(PANEL_ITEM_FONT_SIZE),
				"No owned items",
				TEXT_YELLOW_FAINT,
				bevy::text::Justify::Center,
			);
		});
}

fn spawn_empty_drawer(parent: &mut ChildSpawnerCommands, fonts: &HudFonts, active: bool) -> Entity {
	let mut area = parent.spawn((
		InventoryScreenArea(InventoryRegion::Drawer),
		InventoryHudMenu,
		HudMenu::new(0),
		Node {
			width: Val::Percent(100.0),
			flex_grow: 1.0,
			min_height: Val::Px(0.0),
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			..default()
		},
		Pickable::IGNORE,
	));
	if active {
		area.insert(HudOverlayMenu);
	}
	let entity = area.id();
	area.with_children(|empty| spawn_empty_storage(empty, fonts));
	entity
}

fn section_data<E: Copy>(
	node: &MenuNode<E>,
	section: InventorySection,
) -> Option<(Vec<GridCatalogChoice<E>>, Vec<EquipmentChoice<E>>, usize)> {
	fn walk<E: Copy>(
		nodes: &[MenuNode<E>],
		section: InventorySection,
		choices: &mut Vec<GridCatalogChoice<E>>,
		equipment: &mut Vec<EquipmentChoice<E>>,
		capacity: &mut Option<usize>,
		found: &mut bool,
	) {
		for node in nodes {
			match node {
				MenuNode::Section { label, children } if *label == section.label() => {
					*found = true;
					for child in children {
						match child {
							MenuNode::GridCatalog { max_selected, choices: values, .. } => {
								choices.extend(values.iter().cloned());
								*capacity = Some(*max_selected);
							}
							MenuNode::OrderedEquipment { rows } => {
								*capacity = Some(rows.iter().map(|row| row.capacity).sum());
								equipment.extend(
									rows.iter().flat_map(|row| row.choices.iter().cloned()),
								);
							}
							MenuNode::Fragment(nested) => {
								walk(nested, section, choices, equipment, capacity, found)
							}
							_ => {}
						}
					}
				}
				MenuNode::Fragment(children) | MenuNode::Section { children, .. } => {
					walk(children, section, choices, equipment, capacity, found)
				}
				_ => {}
			}
		}
	}
	let mut choices = Vec::new();
	let mut equipment = Vec::new();
	let mut capacity = None;
	let mut found = false;
	walk(
		std::slice::from_ref(node),
		section,
		&mut choices,
		&mut equipment,
		&mut capacity,
		&mut found,
	);
	if !found {
		return None;
	}
	if section == InventorySection::Lubikan && choices.is_empty() {
		choices.extend(equipment.iter().map(|choice| {
			let mut tile = choice.tile.clone();
			tile.selected = true;
			tile.item_key = None;
			tile.event = None;
			tile
		}));
	}
	let capacity = capacity.unwrap_or(match section {
		InventorySection::Clothing => 6,
		InventorySection::Tools | InventorySection::Skills => 3,
		InventorySection::Tanzes => 4,
		InventorySection::Lubikan => 1,
	});
	let capacity = if section == InventorySection::Lubikan { equipment.len() } else { capacity };
	Some((choices, equipment, capacity))
}

fn catalog_image<E, C: MenuThumbnailContext>(
	choice: &GridCatalogChoice<E>,
	thumbnails: &mut C,
) -> (Option<Handle<Image>>, Option<Entity>, Color) {
	let viewport = choice.image_key.and_then(|key| thumbnails.viewport_for_key(key));
	let thumbnail = if viewport.is_some() {
		None
	} else {
		choice.image_key.and_then(|key| thumbnails.image_for_key(key)).or_else(|| {
			(!choice.path.is_empty())
				.then(|| {
					thumbnails.image_for_asset(
						choice.path,
						choice.path,
						inventory_color(choice.preview),
						choice.thumbnail_camera,
					)
				})
				.flatten()
		})
	};
	let tint = choice.image_key.map(|key| thumbnails.tint_for_key(key)).unwrap_or(Color::WHITE);
	(thumbnail, viewport, tint)
}

fn inventory_color(color: character_ui_menu::PreviewColor) -> Color {
	Color::srgba(color.red, color.green, color.blue, color.alpha)
}

fn bounded_focus(focused: usize, count: usize) -> Option<usize> {
	(count > 0).then(|| focused.min(count - 1))
}

fn gallery_content_height(rows: usize) -> f32 {
	rows as f32 * INVENTORY_GALLERY_ROW_HEIGHT + rows.saturating_sub(1) as f32 * GALLERY_ROW_GAP
}

fn rotary_angle(ordinal: usize, selected: usize, count: usize) -> f32 {
	if count < 2 {
		return 0.0;
	}
	(ordinal as f32 - selected as f32) * std::f32::consts::TAU / count as f32
}

fn rotary_x(ordinal: usize, selected: usize, count: usize) -> f32 {
	rotary_angle(ordinal, selected, count).sin() * 228.0
}

fn rotary_y(ordinal: usize, selected: usize, count: usize) -> f32 {
	(1.0 + rotary_angle(ordinal, selected, count).cos()) * 11.0
}

fn rotary_scale(ordinal: usize, selected: usize, count: usize) -> f32 {
	0.62 + (rotary_angle(ordinal, selected, count).cos() + 1.0) * 0.19
}

/// Ease rotary cards to their wrapped positions. No thumbnails or nodes are
/// rebuilt while focus moves.
pub(crate) fn animate_inventory_carousel(
	time: Res<Time>,
	mut cards: Query<(
		&InventoryCarouselCard,
		&mut InventoryCarouselMotion,
		&mut UiTransform,
		&mut ZIndex,
	)>,
	menus: Query<&HudMenu>,
	mut labels: Query<(&InventoryCarouselLabel, &Children, &mut BackgroundColor)>,
	mut captions: Query<&mut TextColor, With<InventoryCarouselCaption>>,
) {
	let alpha = (1.0 - (-14.0 * time.delta_secs()).exp()).clamp(0.0, 1.0);
	for (card, mut motion, mut transform, mut z_index) in &mut cards {
		let Ok(menu) = menus.get(card.menu) else { continue };
		if card.count == 0 {
			continue;
		}
		let selected = menu.selected.min(card.count - 1);
		let target_angle = rotary_angle(card.ordinal, selected, card.count);
		let delta = (target_angle - motion.angle + std::f32::consts::PI)
			.rem_euclid(std::f32::consts::TAU)
			- std::f32::consts::PI;
		motion.angle += delta * alpha;
		let x = motion.angle.sin() * 228.0;
		let y = (1.0 + motion.angle.cos()) * 11.0;
		transform.translation = Val2::px(x, y);
		let scale = rotary_scale(card.ordinal, selected, card.count);
		motion.scale += (scale - motion.scale) * alpha;
		transform.scale = Vec2::splat(motion.scale);
		*z_index = ZIndex((100.0 + motion.angle.cos() * 50.0) as i32);
	}
	for (label, children, mut background) in &mut labels {
		let selected = menus.get(label.menu).is_ok_and(|menu| menu.selected == label.ordinal);
		background.0 = if selected { TEXT_YELLOW } else { Color::NONE };
		for child in children.iter() {
			if let Ok(mut color) = captions.get_mut(child) {
				color.0 = if selected { TEXT_DEEP_BLUE } else { TEXT_YELLOW_FAINT };
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use menu_components::HudFonts;

	fn grid_section(section: InventorySection, capacity: usize) -> MenuNode<()> {
		MenuNode::Section {
			label: "Inventory",
			children: vec![MenuNode::Section {
				label: section.label(),
				children: vec![MenuNode::GridCatalog {
					label: section.label(),
					max_selected: capacity,
					choices: Vec::new(),
				}],
			}],
		}
	}

	#[test]
	fn inventory_capacity_and_empty_focus_are_bounded() -> anyhow::Result<()> {
		for (section, expected) in [
			(InventorySection::Clothing, 6),
			(InventorySection::Tools, 3),
			(InventorySection::Skills, 3),
			(InventorySection::Tanzes, 4),
		] {
			let node = grid_section(section, expected);
			let (_, _, capacity) = section_data(&node, section)
				.ok_or_else(|| anyhow::anyhow!("missing {} section", section.label()))?;
			assert_eq!(capacity, expected, "{} capacity", section.label());
		}
		assert_eq!(bounded_focus(0, 0), None);
		assert_eq!(bounded_focus(99, 3), Some(2));
		assert_eq!(
			INVENTORY_GALLERY_ROW_HEIGHT * INVENTORY_GALLERY_VISIBLE_ROWS as f32 + GALLERY_ROW_GAP,
			GALLERY_VIEWPORT_HEIGHT
		);
		Ok(())
	}

	#[test]
	fn lubikan_ledger_keeps_every_account_tile() -> anyhow::Result<()> {
		let choices: Vec<EquipmentChoice<()>> = (0..3)
			.map(|item_key| EquipmentChoice {
				item_key,
				tile: GridCatalogChoice::<()> {
					label: format!("account-{item_key}"),
					detail: "Always equipped".into(),
					path: "",
					thumbnail_camera: character_ui_menu::ThumbnailCamera::DEFAULT,
					preview: character_ui_menu::PreviewColor::WHITE,
					image_key: None,
					selected: true,
					rank: Some(item_key as u8 + 1),
					item_key: Some(item_key),
					event: None,
				},
				unequip: None,
				move_left: None,
				move_right: None,
			})
			.collect();
		let node = MenuNode::Section {
			label: "Inventory",
			children: vec![MenuNode::Section {
				label: "Lubikan",
				children: vec![MenuNode::OrderedEquipment {
					rows: vec![character_ui_menu::OrderedEquipmentRow {
						label: "Lubikan",
						capacity: 1,
						choices,
					}],
				}],
			}],
		};
		let (drawer, ledger, capacity) = section_data(&node, InventorySection::Lubikan)
			.ok_or_else(|| anyhow::anyhow!("missing Lubikan section"))?;
		assert_eq!(drawer.len(), 3);
		assert!(drawer
			.iter()
			.all(|choice| choice.selected && choice.item_key.is_none() && choice.event.is_none()));
		assert_eq!(ledger.len(), 3);
		assert_eq!(capacity, 3);
		Ok(())
	}

	#[test]
	fn gallery_shell_spawns_two_rows_and_focusable_capacity_slots() -> anyhow::Result<()> {
		let choices = (0..15)
			.map(|index| GridCatalogChoice {
				label: format!("item-{index}"),
				detail: String::new(),
				path: "",
				thumbnail_camera: character_ui_menu::ThumbnailCamera::DEFAULT,
				preview: character_ui_menu::PreviewColor::WHITE,
				image_key: None,
				selected: false,
				rank: None,
				item_key: Some(index),
				event: Some(index as u8),
			})
			.collect::<Vec<_>>();
		let node = MenuNode::Section {
			label: "Inventory",
			children: vec![MenuNode::Section {
				label: "Clothing",
				children: vec![MenuNode::GridCatalog {
					label: "Clothing",
					max_selected: 6,
					choices,
				}],
			}],
		};
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_plugins(AssetPlugin::default());
		app.init_asset::<Image>();
		let viewport = app.world_mut().spawn(crate::widgets::OverlaySelectViewport).id();
		let fonts = HudFonts {
			black: Handle::default(),
			semibold: Handle::default(),
			regular: Handle::default(),
			logo: Handle::default(),
		};
		let mut session =
			InventoryMenuSession { drawer: InventoryDrawerMode::Gallery, ..default() };
		session.section_memory[InventorySection::Clothing.index()]
			[InventoryRegion::Drawer.index()]
		.gallery_scroll = 1;
		let _slots = app
			.world_mut()
			.run_system_once(move |mut commands: Commands, assets: Res<AssetServer>| {
				let mut thumbnails = crate::NoThumbnails;
				fill_inventory_screen(
					&mut commands,
					viewport,
					&fonts,
					&assets,
					InputPromptDevice::Keyboard,
					&node,
					true,
					&session,
					&mut thumbnails,
				)
			})
			.map_err(|error| anyhow::anyhow!("render system failed: {error}"))?;
		app.world_mut().flush();
		let mut slots = app.world_mut().query::<&InventoryAIOPSlot>();
		assert_eq!(slots.iter(app.world()).count(), 6);
		let mut menus = app.world_mut().query::<&HudMenu>();
		assert_eq!(
			menus
				.get(app.world(), _slots.menus[1])
				.map_err(|_| anyhow::anyhow!("missing Equipped menu"))?
				.item_count,
			6
		);
		let mut manual_scroll = app.world_mut().query::<(&HudScrollManual, &ScrollPosition)>();
		let manual_offsets = manual_scroll.iter(app.world()).collect::<Vec<_>>();
		assert_eq!(manual_offsets.len(), 1);
		assert_eq!(manual_offsets[0].1.y, INVENTORY_GALLERY_ROW_STEP);
		let mut drawer_nodes = app.world_mut().query::<(&InventoryScreenArea, &Node)>();
		let viewport_node = drawer_nodes
			.iter(app.world())
			.find_map(|(area, node)| (area.0 == InventoryRegion::Drawer).then_some(node))
			.ok_or_else(|| anyhow::anyhow!("missing gallery viewport"))?;
		assert_eq!(viewport_node.height, Val::Px(GALLERY_VIEWPORT_HEIGHT));
		let mut grids = app.world_mut().query::<&Node>();
		assert!(grids.iter(app.world()).any(|node| {
			node.display == Display::Grid
				&& node.grid_template_columns.len() == INVENTORY_GALLERY_COLUMNS
				&& node.grid_auto_rows.len() == 1
				&& node.grid_auto_rows[0] == GridTrack::px(INVENTORY_GALLERY_ROW_HEIGHT)
				&& node.min_height == Val::Px(gallery_content_height(3))
				&& node.flex_shrink == 0.0
		}));
		let mut tiles = app.world_mut().query::<&InventoryTile>();
		assert_eq!(tiles.iter(app.world()).count(), 15);
		Ok(())
	}
}
