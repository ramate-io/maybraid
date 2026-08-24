//! Right-justified character-creator panel over the playground camera.

use bevy::ecs::event::EntityEvent;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use character_ui_menu::{AssetThumbnailDisplay, MenuComponent};
use crozon_character_ui_menus::{CharacterMenu, MenuEvent, SectionId, SectionOpenState};
use maybraid_character_ui_menu_renderer::{
	CharacterMenuEvent, MaybraidCharacterMenuRendererPlugin, MaybraidMenuSink, MenuButton,
	MenuJustify, MenuSink, NoThumbnails, RenderContext, ToggleSectionKey,
};
use menu_components::{HudFonts, PANEL_ROW_GAP};
use menu_screens::MenuScreen;

const PANEL_WIDTH: f32 = 480.0;
const PANEL_HEIGHT_PERCENT: f32 = 82.0;
const SCROLL_LINE_PX: f32 = 14.0;

/// Queue a character-panel spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowCharacter;

/// Marker on the spawned character-panel root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CharacterScreen;

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct CharacterMenuState(pub CharacterMenu);

impl Default for CharacterMenuState {
	fn default() -> Self {
		Self(CharacterMenu::default())
	}
}

#[derive(Resource, Debug)]
pub struct CharacterUiState {
	pub sections: SectionOpenState,
	layout_revision: u64,
}

impl Default for CharacterUiState {
	fn default() -> Self {
		Self { sections: SectionOpenState::default(), layout_revision: 0 }
	}
}

impl CharacterUiState {
	fn bump_layout_revision(&mut self) {
		self.layout_revision += 1;
	}

	fn toggle_section(&mut self, section: SectionId) {
		self.sections.toggle(section);
		self.bump_layout_revision();
	}
}

#[derive(Resource, Default)]
struct CharacterUiSyncState {
	layout_revision: u64,
	menu_dirty: bool,
}

#[derive(Component)]
struct CharacterUiScrollViewport;

#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
struct CharacterUiScroll {
	entity: Entity,
	delta: Vec2,
}

pub fn request_show_character(commands: &mut Commands) {
	commands.spawn(RequestShowCharacter);
}

pub struct CharacterScreenPlugin;

impl Plugin for CharacterScreenPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(MaybraidCharacterMenuRendererPlugin::<CharacterMenu>::default())
			.init_resource::<CharacterMenuState>()
			.init_resource::<CharacterUiState>()
			.init_resource::<CharacterUiSyncState>()
			.add_systems(
				Update,
				(
					apply_show_character,
					dispatch_character_interactions,
					sync_character_ui,
					send_character_ui_scroll_events,
				)
					.chain(),
			)
			.add_observer(on_character_ui_scroll);
	}
}

fn apply_show_character(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	requests: Query<Entity, With<RequestShowCharacter>>,
	existing: Query<Entity, With<MenuScreen>>,
	menu_state: Res<CharacterMenuState>,
	ui_state: Res<CharacterUiState>,
	mut sync_state: ResMut<CharacterUiSyncState>,
) {
	if requests.is_empty() {
		return;
	}
	for entity in &existing {
		commands.entity(entity).despawn();
	}
	for entity in &requests {
		commands.entity(entity).despawn();
	}
	sync_state.layout_revision = ui_state.layout_revision;
	sync_state.menu_dirty = false;
	let viewport = spawn_character_ui_shell(&mut commands);
	rebuild_character_ui_panel(
		&mut commands,
		&HudFonts::load(asset_server.as_ref()),
		menu_state.as_ref(),
		ui_state.as_ref(),
		[viewport],
	);
}

fn sync_character_ui(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	screens: Query<Entity, With<CharacterScreen>>,
	menu_state: Res<CharacterMenuState>,
	ui_state: Res<CharacterUiState>,
	mut sync_state: ResMut<CharacterUiSyncState>,
	viewports: Query<Entity, With<CharacterUiScrollViewport>>,
) {
	if screens.is_empty() {
		return;
	}
	let layout_changed = sync_state.layout_revision != ui_state.layout_revision;
	let menu_changed = sync_state.menu_dirty || menu_state.is_changed();
	if !layout_changed && !menu_changed {
		return;
	}
	sync_state.layout_revision = ui_state.layout_revision;
	sync_state.menu_dirty = false;
	rebuild_character_ui_panel(
		&mut commands,
		&HudFonts::load(asset_server.as_ref()),
		menu_state.as_ref(),
		ui_state.as_ref(),
		&viewports,
	);
}

fn spawn_character_ui_shell(commands: &mut Commands) -> Entity {
	let mut viewport = Entity::PLACEHOLDER;
	commands
		.spawn((
			CharacterScreen,
			MenuScreen,
			Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
			Pickable::IGNORE,
		))
		.with_children(|root| {
			root.spawn((
				Node {
					position_type: PositionType::Absolute,
					top: Val::Px(10.0),
					right: Val::Px(10.0),
					width: Val::Px(PANEL_WIDTH),
					height: Val::Percent(PANEL_HEIGHT_PERCENT),
					padding: UiRect::all(Val::Px(16.0)),
					flex_direction: FlexDirection::Column,
					overflow: Overflow::clip(),
					..default()
				},
				Pickable::IGNORE,
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
							align_items: AlignItems::FlexEnd,
							row_gap: Val::Px(PANEL_ROW_GAP),
							overflow: Overflow::scroll_y(),
							..default()
						},
						ScrollPosition::default(),
						Pickable::default(),
						CharacterUiScrollViewport,
					))
					.id();
			});
		});
	viewport
}

fn rebuild_character_ui_panel(
	commands: &mut Commands,
	fonts: &HudFonts,
	menu_state: &CharacterMenuState,
	ui_state: &CharacterUiState,
	viewports: impl IntoIterator<Item = Entity>,
) {
	let mut prewarm = Vec::new();
	for viewport in viewports {
		commands.entity(viewport).despawn_related::<Children>();
		commands.entity(viewport).with_children(|panel| {
			populate_character_ui_panel(panel, fonts, menu_state, ui_state, &mut prewarm);
		});
	}
}

fn populate_character_ui_panel(
	panel: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	menu_state: &CharacterMenuState,
	ui_state: &CharacterUiState,
	prewarm: &mut Vec<character_ui_menu::ThumbnailRequest>,
) {
	let mut thumbnails = NoThumbnails;
	let mut context = RenderContext {
		fonts,
		sections: &ui_state.sections,
		thumbnails: &mut thumbnails,
		asset_thumbnails: AssetThumbnailDisplay::None,
		prewarm,
	};
	MaybraidMenuSink::new(MenuJustify::Right).render_nodes(
		&menu_state.0.menu_nodes(),
		panel,
		&mut context,
	);
}

fn dispatch_character_interactions(
	mut menu_state: ResMut<CharacterMenuState>,
	mut menu_events: MessageWriter<CharacterMenuEvent<CharacterMenu>>,
	mut ui_state: ResMut<CharacterUiState>,
	mut ui_sync: ResMut<CharacterUiSyncState>,
	screens: Query<Entity, With<CharacterScreen>>,
	mut section_interactions: Query<
		(&Interaction, &ToggleSectionKey),
		(Changed<Interaction>, With<Button>),
	>,
	mut menu_interactions: Query<
		(&Interaction, &MenuButton<MenuEvent>),
		(Changed<Interaction>, With<Button>, Without<ToggleSectionKey>),
	>,
) {
	if screens.is_empty() {
		return;
	}
	for (interaction, toggle) in &mut section_interactions {
		if *interaction != Interaction::Pressed {
			continue;
		}
		if let Some(section) = section_id_for_label(toggle.0) {
			ui_state.toggle_section(section);
		}
	}

	for (interaction, button) in &mut menu_interactions {
		if *interaction != Interaction::Pressed {
			continue;
		}
		let event = button.0;
		if let Some(focus) = menu_state.0.camera_focus_for_event(event) {
			menu_events.write(CharacterMenuEvent::CameraFocus(focus));
		}
		if !menu_state.0.apply(event) {
			continue;
		}
		ui_sync.menu_dirty = true;
		menu_events.write(CharacterMenuEvent::MenuUpdate(menu_state.0.clone()));
	}
}

fn send_character_ui_scroll_events(
	mut mouse_wheel_reader: MessageReader<MouseWheel>,
	hover_map: Res<HoverMap>,
	mut commands: Commands,
	screens: Query<Entity, With<CharacterScreen>>,
) {
	if screens.is_empty() {
		return;
	}
	for mouse_wheel in mouse_wheel_reader.read() {
		let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);
		if mouse_wheel.unit == MouseScrollUnit::Line {
			delta *= SCROLL_LINE_PX;
		}
		for pointer_map in hover_map.values() {
			for entity in pointer_map.keys().copied() {
				commands.trigger(CharacterUiScroll { entity, delta });
			}
		}
	}
}

fn on_character_ui_scroll(
	mut scroll: On<CharacterUiScroll>,
	mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode), With<CharacterUiScrollViewport>>,
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

fn section_id_for_label(label: &'static str) -> Option<SectionId> {
	match label {
		"Presets" => Some(SectionId::Presets),
		"Head" => Some(SectionId::Head),
		"Body" => Some(SectionId::Body),
		"Head & Features" => Some(SectionId::HeadFeatures),
		"Hair" => Some(SectionId::Hair),
		"Clothing" => Some(SectionId::Clothing),
		"Animation" => Some(SectionId::Animation),
		_ => None,
	}
}
