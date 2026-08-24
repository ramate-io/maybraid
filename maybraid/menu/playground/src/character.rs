//! Right-justified character-creator panel over the playground camera.

use bevy::prelude::*;
use character_ui_menu::{AssetThumbnailDisplay, MenuComponent};
use crozon_character_ui_menus::{CharacterMenu, MenuEvent, SectionId, SectionOpenState};
use game_commands::command::TextEntryFocus;
use maybraid_character_ui_menu_renderer::{
	find_overlay_node, overlay_closes_on_pick, render_overlay_body, spawn_overlay_shell,
	CharacterMenuEvent, CloseOverlaySelect, MaybraidCharacterMenuRendererPlugin, MaybraidMenuSink,
	MenuButton, MenuJustify, MenuSink, NoThumbnails, OpenSelectKey, OverlaySelectRoot,
	OverlaySelectViewport, RenderContext, ToggleSectionKey,
};
use menu_components::{spawn_scroll_pane, HudFonts, PANEL_ROW_GAP};
use menu_screens::MenuScreen;

const PANEL_WIDTH: f32 = 480.0;
const PANEL_HEIGHT_PERCENT: f32 = 82.0;

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

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
struct OverlaySelectState {
	open: Option<&'static str>,
}

#[derive(Resource, Default)]
struct OverlayUiSyncState {
	open: Option<&'static str>,
	menu_dirty: bool,
}

#[derive(Component)]
struct CharacterPanelViewport;

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
			.init_resource::<OverlaySelectState>()
			.init_resource::<OverlayUiSyncState>()
			.add_systems(
				Update,
				(
					apply_show_character,
					dispatch_character_interactions,
					close_overlay_on_escape,
					sync_character_ui,
					sync_overlay_select,
				)
					.chain(),
			);
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
	mut overlay: ResMut<OverlaySelectState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
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
	overlay.open = None;
	overlay_sync.open = None;
	overlay_sync.menu_dirty = false;
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
	viewports: Query<Entity, With<CharacterPanelViewport>>,
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
				viewport = spawn_scroll_pane(
					shell,
					CharacterPanelViewport,
					AlignItems::FlexEnd,
					PANEL_ROW_GAP,
				);
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

fn sync_overlay_select(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	screens: Query<Entity, With<CharacterScreen>>,
	overlays: Query<Entity, With<OverlaySelectRoot>>,
	viewports: Query<Entity, With<OverlaySelectViewport>>,
	menu_state: Res<CharacterMenuState>,
	ui_state: Res<CharacterUiState>,
	overlay: Res<OverlaySelectState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
) {
	let Ok(screen) = screens.single() else {
		return;
	};
	let menu_changed = overlay_sync.menu_dirty || menu_state.is_changed();
	let open_changed = overlay_sync.open != overlay.open;
	if !open_changed && !menu_changed {
		return;
	}
	overlay_sync.open = overlay.open;
	overlay_sync.menu_dirty = false;

	let Some(key) = overlay.open else {
		for entity in &overlays {
			commands.entity(entity).despawn();
		}
		return;
	};

	let fonts = HudFonts::load(asset_server.as_ref());
	let nodes = menu_state.0.menu_nodes();
	let Some(node) = find_overlay_node(&nodes, key) else {
		for entity in &overlays {
			commands.entity(entity).despawn();
		}
		return;
	};

	if overlays.is_empty() || open_changed {
		for entity in &overlays {
			commands.entity(entity).despawn();
		}
		let mut viewport = Entity::PLACEHOLDER;
		commands.entity(screen).with_children(|root| {
			viewport = spawn_overlay_shell(root, &fonts, key);
		});
		populate_overlay_viewport(&mut commands, viewport, &fonts, node, ui_state.as_ref());
		return;
	}

	for viewport in &viewports {
		populate_overlay_viewport(&mut commands, viewport, &fonts, node, ui_state.as_ref());
	}
}

fn populate_overlay_viewport<E: Copy + Send + Sync + 'static>(
	commands: &mut Commands,
	viewport: Entity,
	fonts: &HudFonts,
	node: &character_ui_menu::MenuNode<E>,
	ui_state: &CharacterUiState,
) {
	let mut prewarm = Vec::new();
	let mut thumbnails = NoThumbnails;
	commands.entity(viewport).despawn_related::<Children>();
	commands.entity(viewport).with_children(|body| {
		let mut context = RenderContext {
			fonts,
			sections: &ui_state.sections,
			thumbnails: &mut thumbnails,
			asset_thumbnails: AssetThumbnailDisplay::None,
			prewarm: &mut prewarm,
		};
		render_overlay_body(node, body, &mut context, MenuJustify::Left);
	});
}

fn dispatch_character_interactions(
	mut menu_state: ResMut<CharacterMenuState>,
	mut menu_events: MessageWriter<CharacterMenuEvent<CharacterMenu>>,
	mut ui_state: ResMut<CharacterUiState>,
	mut ui_sync: ResMut<CharacterUiSyncState>,
	mut overlay: ResMut<OverlaySelectState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
	screens: Query<Entity, With<CharacterScreen>>,
	mut section_interactions: Query<
		(&Interaction, &ToggleSectionKey),
		(Changed<Interaction>, With<Button>),
	>,
	mut open_interactions: Query<
		(&Interaction, &OpenSelectKey),
		(Changed<Interaction>, With<Button>, Without<ToggleSectionKey>),
	>,
	mut close_interactions: Query<
		&Interaction,
		(Changed<Interaction>, With<Button>, With<CloseOverlaySelect>),
	>,
	mut menu_interactions: Query<
		(&Interaction, &MenuButton<MenuEvent>),
		(
			Changed<Interaction>,
			With<Button>,
			Without<ToggleSectionKey>,
			Without<OpenSelectKey>,
			Without<CloseOverlaySelect>,
		),
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

	for (interaction, open) in &mut open_interactions {
		if *interaction == Interaction::Pressed {
			overlay.open = Some(open.0);
		}
	}

	for interaction in &mut close_interactions {
		if *interaction == Interaction::Pressed {
			overlay.open = None;
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
		overlay_sync.menu_dirty = true;
		menu_events.write(CharacterMenuEvent::MenuUpdate(menu_state.0.clone()));
		if let Some(key) = overlay.open {
			if overlay_closes_on_pick(key) {
				overlay.open = None;
			}
		}
	}
}

fn close_overlay_on_escape(
	keys: Res<ButtonInput<KeyCode>>,
	focus: Option<Res<TextEntryFocus>>,
	mut overlay: ResMut<OverlaySelectState>,
	screens: Query<Entity, With<CharacterScreen>>,
) {
	if screens.is_empty() || overlay.open.is_none() {
		return;
	}
	if focus.is_some_and(|focus| focus.0) {
		return;
	}
	if keys.just_pressed(KeyCode::Escape) {
		overlay.open = None;
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
