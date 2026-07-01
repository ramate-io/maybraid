mod braidman;

use bevy::asset::RenderAssetUsages;
use bevy::ecs::event::EntityEvent;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, TextureFormat, TextureUsages};
use crozon_characters::species::braidman::BraidmanColor;
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText, GameCommandUiConfig};

use crate::{
	camera_focus::PendingCameraFocus,
	preview::ConceptPreviewConfig,
	thumbnail::ThumbnailCache,
};

pub use braidman::{CameraFocus, CreatorUiAction, UiAssetTarget, UiColorTarget};

const PANEL_WIDTH: f32 = 360.0;
const PANEL_HEIGHT_PERCENT: f32 = 82.0;
const BUTTON_HEIGHT: f32 = 22.0;
pub(crate) const SLIDER_STEP: f32 = 0.05;
pub(crate) const TILT_STEP_DEG: f32 = 0.5;
pub(crate) const THUMBNAIL_SIZE: f32 = 54.0;
const SCROLL_LINE_PX: f32 = 14.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UiSection {
	#[default]
	Presets,
	Body,
	HeadFeatures,
	Hair,
	Clothing,
	Animation,
}

impl UiSection {
	pub(crate) const fn label(self) -> &'static str {
		match self {
			Self::Presets => "Presets",
			Self::Body => "Body",
			Self::HeadFeatures => "Head & Features",
			Self::Hair => "Hair",
			Self::Clothing => "Clothing",
			Self::Animation => "Animation",
		}
	}
}

#[derive(Resource, Debug)]
pub struct CreatorUiState {
	pub presets_open: bool,
	pub body_open: bool,
	pub head_features_open: bool,
	pub hair_open: bool,
	pub clothing_open: bool,
	pub animation_open: bool,
	pub hovered: Option<UiAssetTarget>,
	pub last_selected: Option<UiAssetTarget>,
	layout_revision: u64,
}

impl Default for CreatorUiState {
	fn default() -> Self {
		Self {
			presets_open: true,
			body_open: true,
			head_features_open: false,
			hair_open: false,
			clothing_open: true,
			animation_open: false,
			hovered: None,
			last_selected: None,
			layout_revision: 0,
		}
	}
}

impl CreatorUiState {
	pub const fn focused_target(&self) -> Option<UiAssetTarget> {
		match self.hovered {
			Some(target) => Some(target),
			None => self.last_selected,
		}
	}

	pub(crate) fn is_open(&self, section: UiSection) -> bool {
		match section {
			UiSection::Presets => self.presets_open,
			UiSection::Body => self.body_open,
			UiSection::HeadFeatures => self.head_features_open,
			UiSection::Hair => self.hair_open,
			UiSection::Clothing => self.clothing_open,
			UiSection::Animation => self.animation_open,
		}
	}

	pub(crate) fn toggle(&mut self, section: UiSection) {
		match section {
			UiSection::Presets => self.presets_open = !self.presets_open,
			UiSection::Body => self.body_open = !self.body_open,
			UiSection::HeadFeatures => self.head_features_open = !self.head_features_open,
			UiSection::Hair => self.hair_open = !self.hair_open,
			UiSection::Clothing => self.clothing_open = !self.clothing_open,
			UiSection::Animation => self.animation_open = !self.animation_open,
		}
		self.layout_revision += 1;
	}
}

#[derive(Resource, Default)]
pub struct CreatorUiSyncState {
	config_key: String,
	layout_revision: u64,
}

#[derive(Component)]
pub struct CreatorUiRoot;

#[derive(Component)]
pub struct CreatorUiScrollViewport;

#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct CreatorUiScroll {
	pub entity: Entity,
	pub delta: Vec2,
}

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Crozon character concepts - / cmd - F1 drawer - L look - WASD".into(),
		empty_console_text: "Console: (errors & `help` output) - wheel or PgUp/PgDn".into(),
		root_background: Color::srgba(0.12, 0.14, 0.18, 0.82),
		controls_hint: "F1 hide drawer - L lock look - help - Enter - up/down history".into(),
	}
}

pub fn drawer_config() -> GameCommandDrawerConfig {
	GameCommandDrawerConfig { open_at_start: false, ..GameCommandDrawerConfig::default() }
}

pub(crate) fn sync_command_status_text(
	config: Res<ConceptPreviewConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = config.status_label();
}

pub fn setup_creator_ui(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut images: ResMut<Assets<Image>>,
	mut thumbnails: ResMut<ThumbnailCache>,
	config: Res<ConceptPreviewConfig>,
	ui_state: Res<CreatorUiState>,
	mut sync_state: ResMut<CreatorUiSyncState>,
) {
	sync_state.config_key = config.sync_key();
	sync_state.layout_revision = ui_state.layout_revision;
	spawn_creator_ui(
		&mut commands,
		asset_server.as_ref(),
		&mut images,
		&mut thumbnails,
		&config,
		&ui_state,
	);
}

pub fn sync_creator_ui(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut images: ResMut<Assets<Image>>,
	mut thumbnails: ResMut<ThumbnailCache>,
	config: Res<ConceptPreviewConfig>,
	ui_state: Res<CreatorUiState>,
	mut sync_state: ResMut<CreatorUiSyncState>,
	roots: Query<Entity, With<CreatorUiRoot>>,
) {
	let key = config.sync_key();
	if sync_state.config_key == key && sync_state.layout_revision == ui_state.layout_revision {
		return;
	}
	sync_state.config_key = key;
	sync_state.layout_revision = ui_state.layout_revision;
	for root in &roots {
		commands.entity(root).try_despawn();
	}
	spawn_creator_ui(
		&mut commands,
		asset_server.as_ref(),
		&mut images,
		&mut thumbnails,
		&config,
		&ui_state,
	);
}

pub fn react_creator_ui(
	mut interactions: Query<(&Interaction, &CreatorUiAction), (Changed<Interaction>, With<Button>)>,
	mut config: ResMut<ConceptPreviewConfig>,
	mut ui_state: ResMut<CreatorUiState>,
	mut pending_camera: ResMut<PendingCameraFocus>,
) {
	for (interaction, action) in &mut interactions {
		if *interaction == Interaction::Hovered {
			if let Some(target) = action.focus_target() {
				ui_state.hovered = Some(target);
			}
			continue;
		}
		if *interaction != Interaction::Pressed {
			continue;
		}
		if let Some(target) = action.focus_target() {
			ui_state.last_selected = Some(target);
			pending_camera.focus = Some(target.camera_focus());
		}
		let ConceptPreviewConfig::Braidman { config: braidman, animation } = config.as_mut();
		braidman::apply_action(*action, braidman, animation, &mut ui_state);
	}
}

fn spawn_creator_ui(
	commands: &mut Commands,
	asset_server: &AssetServer,
	images: &mut Assets<Image>,
	thumbnails: &mut ThumbnailCache,
	config: &ConceptPreviewConfig,
	ui_state: &CreatorUiState,
) {
	thumbnails.begin_ui_rebuild();
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(10.0),
				right: Val::Px(10.0),
				width: Val::Px(PANEL_WIDTH),
				height: Val::Percent(PANEL_HEIGHT_PERCENT),
				padding: UiRect::all(Val::Px(8.0)),
				flex_direction: FlexDirection::Column,
				overflow: Overflow::clip(),
				..default()
			},
			BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.84)),
			CreatorUiRoot,
		))
		.with_children(|shell| {
			shell
				.spawn((
					Node {
						width: Val::Percent(100.0),
						flex_grow: 1.0,
						flex_shrink: 1.0,
						min_height: Val::Px(0.0),
						flex_direction: FlexDirection::Column,
						row_gap: Val::Px(5.0),
						overflow: Overflow::scroll_y(),
						..default()
					},
					ScrollPosition::default(),
					Pickable::default(),
					CreatorUiScrollViewport,
				))
				.with_children(|panel| {
					text(panel, "Character Concepts", 15.0, Color::WHITE);
					text(
						panel,
						"Expandable controls, thumbnails, colors, and focus camera.",
						10.0,
						muted(),
					);
					braidman::populate_panel(
						panel,
						asset_server,
						images,
						thumbnails,
						config,
						ui_state,
					);
				});
		});
}

pub(crate) fn section(
	parent: &mut ChildSpawnerCommands,
	section: UiSection,
	state: &CreatorUiState,
	body: impl FnOnce(&mut ChildSpawnerCommands),
) {
	let open = state.is_open(section);
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(4.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|section_parent| {
			button(
				section_parent,
				&format!("{} {}", if open { "v" } else { ">" }, section.label()),
				CreatorUiAction::ToggleSection(section),
				open,
			);
			if open {
				body(section_parent);
			}
		});
}

pub(crate) fn subsection(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	body: impl FnOnce(&mut ChildSpawnerCommands),
) {
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(4.0),
				padding: UiRect::new(Val::Px(2.0), Val::Px(0.0), Val::Px(0.0), Val::Px(2.0)),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|sub| {
			text(sub, label, 12.0, Color::srgb(0.78, 0.84, 0.92));
			body(sub);
		});
}

pub(crate) fn selector(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	value: &'static str,
	action: fn(i32) -> CreatorUiAction,
) {
	parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, label, 11.0, Color::WHITE);
		button(row, "<", action(-1), false);
		text(row, value, 11.0, Color::srgb(0.85, 0.95, 1.0));
		button(row, ">", action(1), false);
	});
}

pub(crate) fn button(
	parent: &mut ChildSpawnerCommands,
	label: &str,
	action: CreatorUiAction,
	active: bool,
) {
	parent
		.spawn((
			Button,
			Node {
				min_width: Val::Px(28.0),
				height: Val::Px(BUTTON_HEIGHT),
				padding: UiRect::axes(Val::Px(7.0), Val::Px(2.0)),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(if active {
				Color::srgba(0.16, 0.34, 0.50, 0.95)
			} else {
				Color::srgba(0.18, 0.20, 0.24, 0.92)
			}),
			action,
		))
		.with_children(|button| text(button, label, 10.0, Color::WHITE));
}

pub(crate) fn color_swatches(
	parent: &mut ChildSpawnerCommands,
	target: UiColorTarget,
	active: BraidmanColor,
) {
	parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, braidman::color_target_label(target), 11.0, Color::WHITE);
		for color in braidman::COLORS {
			row.spawn((
				Button,
				Node {
					width: Val::Px(22.0),
					height: Val::Px(18.0),
					border: UiRect::all(Val::Px(if *color == active { 2.0 } else { 1.0 })),
					..default()
				},
				BorderColor::all(if *color == active { Color::WHITE } else { muted() }),
				BackgroundColor(color.color()),
				CreatorUiAction::SetColor(target, *color),
			));
		}
	});
}

pub(crate) fn inline_color_swatches(
	parent: &mut ChildSpawnerCommands,
	target: UiColorTarget,
	active: BraidmanColor,
) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				flex_wrap: FlexWrap::Wrap,
				column_gap: Val::Px(3.0),
				row_gap: Val::Px(3.0),
				align_items: AlignItems::Center,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			for color in braidman::COLORS {
				row.spawn((
					Button,
					Node {
						width: Val::Px(20.0),
						height: Val::Px(16.0),
						border: UiRect::all(Val::Px(if *color == active { 2.0 } else { 1.0 })),
						..default()
					},
					BorderColor::all(if *color == active { Color::WHITE } else { muted() }),
					BackgroundColor(color.color()),
					CreatorUiAction::SetColor(target, *color),
				));
			}
		});
}

pub(crate) fn text(parent: &mut ChildSpawnerCommands, value: &str, size: f32, color: Color) {
	parent.spawn((
		Text::new(value.to_string()),
		TextFont { font_size: size, ..default() },
		TextColor(color),
		Pickable::IGNORE,
	));
}

pub(crate) fn row_node() -> Node {
	Node {
		width: Val::Percent(100.0),
		min_height: Val::Px(24.0),
		flex_direction: FlexDirection::Row,
		column_gap: Val::Px(5.0),
		align_items: AlignItems::Center,
		justify_content: JustifyContent::SpaceBetween,
		..default()
	}
}

pub(crate) fn muted() -> Color {
	Color::srgba(0.72, 0.78, 0.86, 1.0)
}

pub fn send_creator_ui_scroll_events(
	mut mouse_wheel_reader: MessageReader<MouseWheel>,
	hover_map: Res<HoverMap>,
	mut commands: Commands,
) {
	for mouse_wheel in mouse_wheel_reader.read() {
		let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);
		if mouse_wheel.unit == MouseScrollUnit::Line {
			delta *= SCROLL_LINE_PX;
		}
		for pointer_map in hover_map.values() {
			for entity in pointer_map.keys().copied() {
				commands.trigger(CreatorUiScroll { entity, delta });
			}
		}
	}
}

pub fn on_creator_ui_scroll(
	mut scroll: On<CreatorUiScroll>,
	mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode), With<CreatorUiScrollViewport>>,
) {
	let Ok((mut scroll_position, node, computed)) = query.get_mut(scroll.entity) else {
		return;
	};

	let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();
	let delta = &mut scroll.delta;

	if node.overflow.x == OverflowAxis::Scroll && delta.x != 0. {
		let max =
			if delta.x > 0. { scroll_position.x >= max_offset.x } else { scroll_position.x <= 0. };
		if !max {
			scroll_position.x += delta.x;
			delta.x = 0.;
		}
	}

	if node.overflow.y == OverflowAxis::Scroll && delta.y != 0. {
		let max =
			if delta.y > 0. { scroll_position.y >= max_offset.y } else { scroll_position.y <= 0. };
		if !max {
			scroll_position.y += delta.y;
			delta.y = 0.;
		}
	}

	if *delta == Vec2::ZERO {
		scroll.propagate(false);
	}
}

pub fn thumbnail_image() -> Image {
	let mut image = Image::new_uninit(
		default(),
		TextureDimension::D2,
		TextureFormat::Bgra8UnormSrgb,
		RenderAssetUsages::all(),
	);
	image.texture_descriptor.usage =
		TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
	image
}
