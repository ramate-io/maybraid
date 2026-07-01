use bevy::asset::RenderAssetUsages;
use bevy::ecs::event::EntityEvent;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, TextureFormat, TextureUsages};
use bevy_character_ui_menu_renderer::{MenuButton, MenuThumbnailContext, RenderContext, Renderer};
use character_ui_menu::ThumbnailCamera;
use crozon_character_ui_menus::{
	AssetOption, CharacterMenu, MenuEvent, SectionId, SectionOpenState,
};
use crozon_characters::species::{
	brodler::{assets::HornMesh, BrodlerHeadMesh},
	common::{BodyMesh, ClothingMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh},
};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText, GameCommandUiConfig};

use crate::{
	camera_focus::{focus_debug_enabled, queue_camera_focus, PendingCameraFocus},
	focus_reference::FocusReferenceSyncState,
	preview::{
		ConceptPreviewConfig, ConceptPreviewSyncState, ConceptSpecies, PreviewRespawnCooldown,
	},
	species_session::{reset_for_species_switch, CameraFocusBootState, SpeciesSessionState},
	thumbnail::{self, ThumbnailCache},
};

pub use character_ui_menu::CameraFocus;

const PANEL_WIDTH: f32 = 360.0;
const PANEL_HEIGHT_PERCENT: f32 = 82.0;
const SCROLL_LINE_PX: f32 = 14.0;

#[derive(Resource, Debug)]
pub struct CreatorUiState {
	pub sections: SectionOpenState,
	pub hovered: Option<CameraFocus>,
	pub last_selected: Option<CameraFocus>,
	layout_revision: u64,
}

impl Default for CreatorUiState {
	fn default() -> Self {
		Self {
			sections: SectionOpenState::default(),
			hovered: None,
			last_selected: None,
			layout_revision: 0,
		}
	}
}

impl CreatorUiState {
	pub fn focused_target(&self) -> Option<CameraFocus> {
		self.hovered.or(self.last_selected)
	}

	pub fn bump_layout_revision(&mut self) {
		self.layout_revision += 1;
	}

	fn toggle_section(&mut self, section: SectionId) {
		self.sections.toggle(section);
		self.bump_layout_revision();
	}
}

#[derive(Resource, Default)]
pub struct CreatorUiSyncState {
	layout_revision: u64,
	species: Option<ConceptSpecies>,
	sync_key: String,
}

#[derive(Component)]
pub struct CreatorUiRoot;

#[derive(Component)]
pub struct CreatorUiScrollViewport;

#[derive(Component, Clone, Copy, Debug)]
pub(crate) enum SpeciesButton {
	Switch(ConceptSpecies),
}

#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct CreatorUiScroll {
	pub entity: Entity,
	pub delta: Vec2,
}

struct CachedThumbnails<'a> {
	cache: &'a ThumbnailCache,
}

impl MenuThumbnailContext for CachedThumbnails<'_> {
	fn image_for_asset(
		&mut self,
		_label: &'static str,
		asset_path: &'static str,
		color: Color,
		_camera: ThumbnailCamera,
	) -> Option<Handle<Image>> {
		self.cache.cached_image(asset_path, color)
	}
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
	sync_state.layout_revision = ui_state.layout_revision;
	sync_state.species = Some(config.species());
	sync_state.sync_key = config.sync_key();
	let viewport = spawn_creator_ui_shell(&mut commands);
	rebuild_creator_ui_panel(
		&mut commands,
		asset_server.as_ref(),
		images.as_mut(),
		thumbnails.as_mut(),
		config.as_ref(),
		ui_state.as_ref(),
		[viewport],
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
	viewports: Query<Entity, With<CreatorUiScrollViewport>>,
) {
	let species = config.species();
	let sync_key = config.sync_key();
	let layout_changed = sync_state.layout_revision != ui_state.layout_revision;
	let species_changed = sync_state.species != Some(species);
	let config_changed = sync_state.sync_key != sync_key;
	if !layout_changed && !species_changed && !config_changed && !roots.is_empty() {
		return;
	}
	sync_state.layout_revision = ui_state.layout_revision;
	sync_state.species = Some(species);
	sync_state.sync_key = sync_key;
	if roots.is_empty() {
		spawn_creator_ui_shell(&mut commands);
	}
	rebuild_creator_ui_panel(
		&mut commands,
		asset_server.as_ref(),
		images.as_mut(),
		thumbnails.as_mut(),
		config.as_ref(),
		ui_state.as_ref(),
		&viewports,
	);
}

pub fn react_creator_ui(
	mut menu_interactions: Query<(&Interaction, &MenuButton), (Changed<Interaction>, With<Button>)>,
	mut species_interactions: Query<
		(&Interaction, &SpeciesButton),
		(Changed<Interaction>, With<Button>, Without<MenuButton>),
	>,
	mut config: ResMut<ConceptPreviewConfig>,
	mut ui_state: ResMut<CreatorUiState>,
	mut pending_camera: ResMut<PendingCameraFocus>,
	mut session: ResMut<SpeciesSessionState>,
	mut preview_sync: ResMut<ConceptPreviewSyncState>,
	mut focus_sync: ResMut<FocusReferenceSyncState>,
	mut respawn_cooldown: ResMut<PreviewRespawnCooldown>,
	mut camera_boot: ResMut<CameraFocusBootState>,
) {
	for (interaction, SpeciesButton::Switch(species)) in &mut species_interactions {
		if *interaction != Interaction::Pressed || config.species() == *species {
			continue;
		}
		reset_for_species_switch(
			*species,
			&mut session,
			&mut config,
			&mut ui_state,
			&mut preview_sync,
			&mut focus_sync,
			&mut respawn_cooldown,
			&mut pending_camera,
			&mut camera_boot,
		);
	}

	for (interaction, button) in &mut menu_interactions {
		if *interaction == Interaction::Hovered {
			if let Some(focus) = menu_from_config(&config).camera_focus_for_event(button.0) {
				ui_state.hovered = Some(focus);
			}
			continue;
		}
		if *interaction != Interaction::Pressed {
			continue;
		}
		match button.0 {
			MenuEvent::ToggleSection(section) => {
				ui_state.toggle_section(section);
				continue;
			}
			event => {
				let mut menu = menu_from_config(&config);
				if let Some(focus) = menu.camera_focus_for_event(event) {
					ui_state.last_selected = Some(focus);
					queue_camera_focus(&mut pending_camera, focus, format!("ui-press:{event:?}"));
				}
				if !menu.apply(event) {
					continue;
				}
				apply_menu_to_config(menu, &mut config);
				preview_sync.invalidate();
				focus_sync.invalidate();
				respawn_cooldown.frames_remaining = 0;
				if focus_debug_enabled() {
					info!("[camera-focus] typed-ui event={event:?}");
				}
			}
		}
	}
}

pub fn refresh_creator_ui_display() {}

fn menu_from_config(config: &ConceptPreviewConfig) -> CharacterMenu {
	match config {
		ConceptPreviewConfig::Braidman { config, animation } => {
			CharacterMenu::from_braidman(config, *animation)
		}
		ConceptPreviewConfig::Brodler { config, animation } => {
			CharacterMenu::from_brodler(config, *animation)
		}
	}
}

fn apply_menu_to_config(menu: CharacterMenu, config: &mut ConceptPreviewConfig) {
	match menu.species.value {
		crozon_character_ui_menus::ConceptSpecies::Braidman => {
			*config = ConceptPreviewConfig::braidman_with_animation(
				menu.braidman_config(),
				menu.animation(),
			);
		}
		crozon_character_ui_menus::ConceptSpecies::Brodler => {
			*config = ConceptPreviewConfig::brodler_with_animation(
				menu.brodler_config(),
				menu.animation(),
			);
		}
	}
}

fn spawn_creator_ui_shell(commands: &mut Commands) -> Entity {
	let mut viewport = Entity::PLACEHOLDER;
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
			viewport = shell
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
				.id();
		});
	viewport
}

fn rebuild_creator_ui_panel(
	commands: &mut Commands,
	asset_server: &AssetServer,
	images: &mut Assets<Image>,
	thumbnails: &mut ThumbnailCache,
	config: &ConceptPreviewConfig,
	ui_state: &CreatorUiState,
	viewports: impl IntoIterator<Item = Entity>,
) {
	thumbnails.begin_ui_rebuild();
	prewarm_thumbnails(commands, images, asset_server, thumbnails, config);
	for viewport in viewports {
		commands.entity(viewport).despawn_related::<Children>();
		commands.entity(viewport).with_children(|panel| {
			populate_creator_ui_panel(panel, thumbnails, config, ui_state);
		});
	}
}

fn populate_creator_ui_panel(
	panel: &mut ChildSpawnerCommands,
	thumbnails: &ThumbnailCache,
	config: &ConceptPreviewConfig,
	ui_state: &CreatorUiState,
) {
	text(panel, "Character Concepts", 15.0, Color::WHITE);
	text(panel, "Typed menu primitives rendered through a shared Bevy layer.", 10.0, muted());
	species_picker(panel, config.species());
	let menu = match config {
		ConceptPreviewConfig::Braidman { config, animation } => {
			CharacterMenu::from_braidman(config, *animation)
		}
		ConceptPreviewConfig::Brodler { config, animation } => {
			CharacterMenu::from_brodler(config, *animation)
		}
	};
	let mut thumbnails = CachedThumbnails { cache: thumbnails };
	let mut context = RenderContext { sections: ui_state.sections, thumbnails: &mut thumbnails };
	Renderer::default().render(panel, &menu, &mut context);
}

fn prewarm_thumbnails(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	asset_server: &AssetServer,
	thumbnails: &mut ThumbnailCache,
	config: &ConceptPreviewConfig,
) {
	match config {
		ConceptPreviewConfig::Braidman { config, .. } => {
			let skin = config.colors.skin_color().color();
			for body in BodyMesh::VALUES {
				prewarm_asset_option(
					commands,
					images,
					asset_server,
					thumbnails,
					*body,
					config.colors.body.color(),
				);
			}
			for head in HeadMesh::VALUES {
				prewarm_asset_option(commands, images, asset_server, thumbnails, *head, skin);
			}
			prewarm_common_feature_thumbnails(commands, images, asset_server, thumbnails, |slot| {
				match slot {
					CommonThumbnailSlot::Eye => config.colors.eyes.color(),
					CommonThumbnailSlot::Mouth => config.colors.mouth.color(),
					CommonThumbnailSlot::Nose | CommonThumbnailSlot::Ear => skin,
				}
			});
			for hair in HairMesh::VALUES {
				prewarm_asset_option(
					commands,
					images,
					asset_server,
					thumbnails,
					*hair,
					config.colors.hair.color(),
				);
			}
			for clothing in ClothingMesh::VALUES {
				prewarm_asset_option(
					commands,
					images,
					asset_server,
					thumbnails,
					*clothing,
					config.colors.clothing_color(*clothing).color(),
				);
			}
		}
		ConceptPreviewConfig::Brodler { config, .. } => {
			for head in BrodlerHeadMesh::VALUES {
				prewarm_asset_option(
					commands,
					images,
					asset_server,
					thumbnails,
					*head,
					config.colors.skin.color(),
				);
			}
			for horns in HornMesh::VALUES {
				prewarm_asset_option(
					commands,
					images,
					asset_server,
					thumbnails,
					*horns,
					config.colors.horns.color(),
				);
			}
			prewarm_common_feature_thumbnails(commands, images, asset_server, thumbnails, |slot| {
				match slot {
					CommonThumbnailSlot::Eye => config.colors.eyes.color(),
					CommonThumbnailSlot::Mouth => config.colors.mouth.color(),
					CommonThumbnailSlot::Nose | CommonThumbnailSlot::Ear => {
						config.colors.skin.color()
					}
				}
			});
			for hair in HairMesh::VALUES {
				prewarm_asset_option(
					commands,
					images,
					asset_server,
					thumbnails,
					*hair,
					config.colors.hair.color(),
				);
			}
			for clothing in ClothingMesh::VALUES {
				prewarm_asset_option(
					commands,
					images,
					asset_server,
					thumbnails,
					*clothing,
					config.colors.clothing_color(*clothing).color(),
				);
			}
		}
	}
}

#[derive(Clone, Copy)]
enum CommonThumbnailSlot {
	Eye,
	Nose,
	Mouth,
	Ear,
}

fn prewarm_common_feature_thumbnails(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	asset_server: &AssetServer,
	thumbnails: &mut ThumbnailCache,
	color_for_slot: impl Fn(CommonThumbnailSlot) -> Color,
) {
	for eye in EyeMesh::VALUES {
		prewarm_asset_option(
			commands,
			images,
			asset_server,
			thumbnails,
			*eye,
			color_for_slot(CommonThumbnailSlot::Eye),
		);
	}
	for nose in NoseMesh::VALUES {
		prewarm_asset_option(
			commands,
			images,
			asset_server,
			thumbnails,
			*nose,
			color_for_slot(CommonThumbnailSlot::Nose),
		);
	}
	for mouth in MouthMesh::VALUES {
		prewarm_asset_option(
			commands,
			images,
			asset_server,
			thumbnails,
			*mouth,
			color_for_slot(CommonThumbnailSlot::Mouth),
		);
	}
	for ear in EarMesh::VALUES {
		prewarm_asset_option(
			commands,
			images,
			asset_server,
			thumbnails,
			*ear,
			color_for_slot(CommonThumbnailSlot::Ear),
		);
	}
}

fn prewarm_asset_option<T: AssetOption>(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	asset_server: &AssetServer,
	thumbnails: &mut ThumbnailCache,
	value: T,
	color: Color,
) {
	let asset = value.asset();
	if asset.path.is_empty() {
		return;
	}
	let _ = thumbnail::image_for_asset(
		commands,
		images,
		asset_server,
		thumbnails,
		asset.label,
		asset.path,
		color,
		asset.thumbnail_camera,
	);
}

fn species_picker(panel: &mut ChildSpawnerCommands, active: ConceptSpecies) {
	panel
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(6.0),
				row_gap: Val::Px(6.0),
				margin: UiRect::bottom(Val::Px(6.0)),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			text(row, "Species", 11.0, Color::WHITE);
			for species in [ConceptSpecies::Braidman, ConceptSpecies::Brodler] {
				species_button(row, species, active == species);
			}
		});
}

fn species_button(parent: &mut ChildSpawnerCommands, species: ConceptSpecies, active: bool) {
	parent
		.spawn((
			Button,
			Node {
				min_width: Val::Px(28.0),
				height: Val::Px(22.0),
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
			SpeciesButton::Switch(species),
		))
		.with_children(|button| text(button, species.label(), 10.0, Color::WHITE));
}

fn text(parent: &mut ChildSpawnerCommands, value: &str, size: f32, color: Color) {
	parent.spawn((
		Text::new(value.to_string()),
		TextFont { font_size: size, ..default() },
		TextColor(color),
		Pickable::IGNORE,
	));
}

fn muted() -> Color {
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

#[allow(dead_code)]
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
