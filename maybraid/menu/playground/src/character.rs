//! Right-justified character-creator panel over the playground camera.

use bevy::prelude::*;
use character_ui_menu::{AssetThumbnailDisplay, MenuComponent};
use crozon_character_ui_menus::{CharacterMenu, MenuEvent};
use maybraid_character_ui_menu_renderer::{
	find_overlay_node, overlay_closes_on_pick, render_overlay_body, spawn_overlay_shell,
	CharacterHudSystems, CharacterMenuEvent, MaybraidCharacterMenuRendererPlugin, MaybraidMenuSink,
	MenuJustify, MenuSink, NoThumbnails, OverlayClose, OverlayOpen, OverlaySelectRoot,
	OverlaySelectViewport, RenderContext,
};
use menu_components::{
	spawn_scroll_pane, ActiveOverlayKey, HudFonts, HudMenu, HudOverlayMenu, MenuActivate,
	MenuFocus, PANEL_ROW_GAP,
};
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

#[derive(Resource, Default)]
struct CharacterUiSyncState {
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
		app.add_plugins(MaybraidCharacterMenuRendererPlugin::<MenuEvent>::default())
			.init_resource::<CharacterMenuState>()
			.init_resource::<CharacterUiSyncState>()
			.init_resource::<OverlaySelectState>()
			.init_resource::<OverlayUiSyncState>()
			.add_observer(on_overlay_open)
			.add_observer(on_overlay_close)
			.add_observer(on_menu_activate)
			.add_observer(on_menu_focus)
			.add_systems(
				Update,
				(apply_show_character, sync_character_ui, sync_overlay_select)
					.chain()
					.in_set(CharacterHudSystems::Sync),
			);
	}
}

fn apply_show_character(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	requests: Query<Entity, With<RequestShowCharacter>>,
	existing: Query<Entity, With<MenuScreen>>,
	menu_state: Res<CharacterMenuState>,
	mut sync_state: ResMut<CharacterUiSyncState>,
	mut overlay: ResMut<OverlaySelectState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
	mut active_overlay: ResMut<ActiveOverlayKey>,
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
	active_overlay.0 = None;
	sync_state.menu_dirty = false;
	let viewport = spawn_character_ui_shell(&mut commands);
	rebuild_character_ui_panel(
		&mut commands,
		&HudFonts::load(asset_server.as_ref()),
		menu_state.as_ref(),
		[viewport],
		&[],
	);
}

fn sync_character_ui(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	screens: Query<Entity, With<CharacterScreen>>,
	menu_state: Res<CharacterMenuState>,
	mut sync_state: ResMut<CharacterUiSyncState>,
	viewports: Query<(Entity, Option<&HudMenu>), With<CharacterPanelViewport>>,
) {
	if screens.is_empty() {
		return;
	}
	if !sync_state.menu_dirty && !menu_state.is_changed() {
		return;
	}
	sync_state.menu_dirty = false;
	let previous: Vec<(Entity, Option<HudMenu>)> =
		viewports.iter().map(|(entity, menu)| (entity, menu.copied())).collect();
	rebuild_character_ui_panel(
		&mut commands,
		&HudFonts::load(asset_server.as_ref()),
		menu_state.as_ref(),
		previous.iter().map(|(entity, _)| *entity),
		&previous,
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
	viewports: impl IntoIterator<Item = Entity>,
	previous: &[(Entity, Option<HudMenu>)],
) {
	let mut prewarm = Vec::new();
	for viewport in viewports {
		let keep = previous
			.iter()
			.find(|(entity, _)| *entity == viewport)
			.and_then(|(_, menu)| *menu);
		commands.entity(viewport).despawn_related::<Children>();
		let mut item_count = 0;
		commands.entity(viewport).with_children(|panel| {
			item_count =
				populate_character_ui_panel(panel, fonts, menu_state, viewport, &mut prewarm);
		});
		commands.entity(viewport).insert(HudMenu::retain(item_count, keep));
	}
}

fn populate_character_ui_panel(
	panel: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	menu_state: &CharacterMenuState,
	hud_menu: Entity,
	prewarm: &mut Vec<character_ui_menu::ThumbnailRequest>,
) -> usize {
	let mut thumbnails = NoThumbnails;
	let mut context = RenderContext {
		fonts,
		thumbnails: &mut thumbnails,
		asset_thumbnails: AssetThumbnailDisplay::None,
		prewarm,
		hud_menu,
		hud_item_count: 0,
	};
	MaybraidMenuSink::new(MenuJustify::Right).render_nodes(
		&menu_state.0.menu_nodes(),
		panel,
		&mut context,
	);
	context.hud_item_count
}

fn sync_overlay_select(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	screens: Query<Entity, With<CharacterScreen>>,
	overlays: Query<Entity, With<OverlaySelectRoot>>,
	viewports: Query<(Entity, Option<&HudMenu>), With<OverlaySelectViewport>>,
	menu_state: Res<CharacterMenuState>,
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
		populate_overlay_viewport(&mut commands, viewport, &fonts, node, None);
		return;
	}

	for (viewport, menu) in &viewports {
		populate_overlay_viewport(&mut commands, viewport, &fonts, node, menu.copied());
	}
}

fn populate_overlay_viewport<E: Copy + Send + Sync + 'static>(
	commands: &mut Commands,
	viewport: Entity,
	fonts: &HudFonts,
	node: &character_ui_menu::MenuNode<E>,
	previous: Option<HudMenu>,
) {
	let mut prewarm = Vec::new();
	let mut thumbnails = NoThumbnails;
	commands.entity(viewport).despawn_related::<Children>();
	let mut item_count = 0;
	commands.entity(viewport).with_children(|body| {
		let mut context = RenderContext {
			fonts,
			thumbnails: &mut thumbnails,
			asset_thumbnails: AssetThumbnailDisplay::None,
			prewarm: &mut prewarm,
			hud_menu: viewport,
			hud_item_count: 0,
		};
		render_overlay_body(node, body, &mut context, MenuJustify::Left);
		item_count = context.hud_item_count;
	});
	commands
		.entity(viewport)
		.insert((HudMenu::retain(item_count, previous), HudOverlayMenu));
}

fn on_overlay_open(
	open: On<OverlayOpen>,
	mut overlay: ResMut<OverlaySelectState>,
	mut active_overlay: ResMut<ActiveOverlayKey>,
	screens: Query<Entity, With<CharacterScreen>>,
) {
	if screens.is_empty() {
		return;
	}
	overlay.open = Some(open.event().key);
	active_overlay.0 = Some(open.event().key);
}

fn on_overlay_close(
	_close: On<OverlayClose>,
	mut overlay: ResMut<OverlaySelectState>,
	mut active_overlay: ResMut<ActiveOverlayKey>,
	screens: Query<Entity, With<CharacterScreen>>,
) {
	if screens.is_empty() {
		return;
	}
	overlay.open = None;
	active_overlay.0 = None;
}

fn on_menu_activate(
	activate: On<MenuActivate<MenuEvent>>,
	mut menu_state: ResMut<CharacterMenuState>,
	mut menu_events: MessageWriter<CharacterMenuEvent<MenuEvent>>,
	mut ui_sync: ResMut<CharacterUiSyncState>,
	mut overlay: ResMut<OverlaySelectState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
	mut active_overlay: ResMut<ActiveOverlayKey>,
	screens: Query<Entity, With<CharacterScreen>>,
) {
	if screens.is_empty() {
		return;
	}
	let event = activate.event().choice;
	if !menu_state.0.apply(event) {
		return;
	}
	ui_sync.menu_dirty = true;
	overlay_sync.menu_dirty = true;
	menu_events.write(CharacterMenuEvent::Menu(event));
	if let Some(key) = overlay.open {
		let nodes = menu_state.0.menu_nodes();
		if find_overlay_node(&nodes, key).is_some_and(overlay_closes_on_pick) {
			overlay.open = None;
			active_overlay.0 = None;
		}
	}
}

fn on_menu_focus(
	focus: On<MenuFocus<MenuEvent>>,
	menu_state: Res<CharacterMenuState>,
	mut menu_events: MessageWriter<CharacterMenuEvent<MenuEvent>>,
	screens: Query<Entity, With<CharacterScreen>>,
) {
	if screens.is_empty() {
		return;
	}
	if let Some(camera) = menu_state.0.camera_focus_for_event(focus.event().choice) {
		menu_events.write(CharacterMenuEvent::CameraFocus(camera));
	}
}
