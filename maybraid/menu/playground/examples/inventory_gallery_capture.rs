//! Headless capture of the shared inventory layout (Starter Kit + modal).
//!
//! The overflow bag is a **fixture** for the scroll demo. It is not the
//! generated Starter Kit. Real generation still grants three clothing items.
//!
//! ```bash
//! INVENTORY_GALLERY_CAPTURE_DIR=/tmp/inventory-gallery \
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   WGPU_BACKEND=vk \
//!   xvfb-run -a cargo run -p menu-playground --example inventory_gallery_capture
//! ```

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::audio::AudioPlugin;
use bevy::ecs::entity_disabling::Disabled;
use bevy::ecs::query::Allow;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use character_creation_menus::{CharacterMenu, InventoryLayout, MenuEvent, STARTER_KIT_LABEL};
use character_items::{
	catalog_key_uses_yellow_silhouette, self_lubikan_catalog_key, ClothingMaterial, ClothingMesh,
	FirearmMesh, Inventory, InventoryItem, ItemColor, ItemRng, TanzId, BASE_STARTER_TANZES,
};
use character_ui_menu::{InventoryDrawerMode, InventoryRegion, InventorySection, MenuComponent};
use characters::StandaloneCharacterMaterialRefPlugin;
use characters::{CharacterHostsPlugin, HiddenUntilSceneReady, PendingCharacterAssembly};
use item_icons::{
	request_inventory_icons, request_self_lubikan_icon, CaptureCacheRoot, CaptureClient,
	CaptureCompleted, CaptureFailed, CaptureHostReady, CaptureHostRealized, CaptureHostRoot,
	ItemCaptureSubjects, ItemIconCache, ItemIconDiscovery, ItemIconPlugin, PoseCaptureSubjects,
};
use lod::LodViewer;
use maybraid_character_ui_menu_renderer::inventory::{
	INVENTORY_GALLERY_COLUMNS, INVENTORY_GALLERY_VISIBLE_ROWS,
};
use maybraid_character_ui_menu_renderer::{
	fill_inventory_loading, fill_inventory_screen, find_overlay_node, spawn_starter_kit_shell,
	InventoryItemKey, InventoryMenuSession, MenuThumbnailContext,
};
use menu_components::theme::TEXT_YELLOW;
use menu_components::{
	spawn_header_line, spawn_hud_action, HudFonts, HudMenu, HudMenuItem, HudOverlayMenu,
	HudScrollViewport, InputPromptDevice, InventoryHudMenu, ReorderMark,
};
use menu_screens::SpinRevealScreen;
use tanz_user::request_inventory_tanz_icons;

fn assets_path() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Inventory layout capture".into(),
						resolution: capture_resolution().into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path().to_string_lossy().into(), ..default() })
				.disable::<AudioPlugin>(),
		)
		.add_plugins(CharacterHostsPlugin)
		.add_plugins(StandaloneCharacterMaterialRefPlugin)
		.add_plugins(maybraid_character_ui_menu_renderer::MaybraidCharacterMenuRendererPlugin::<
			MenuEvent,
		>::default())
		.add_plugins(menu_playground::CharacterPreviewPlugin)
		.insert_resource(menu_playground::CharacterMenuState(CharacterMenu::default()))
		.add_plugins(ItemIconPlugin)
		.add_systems(Update, menu_components::sync_hover_tiles)
		.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.10)))
		.insert_resource(CaptureDir(shot_dir()))
		.init_resource::<CaptureShots>()
		.add_systems(
			Startup,
			(
				setup,
				(
					characters_playground::camera::setup_camera,
					add_lod_viewer_to_camera.after(characters_playground::camera::setup_camera),
				)
					.chain(),
				allow_slow_braidman_icon,
				request_icon,
			),
		)
		.add_systems(Update, (log_capture_outcome, log_host_progress, apply_reorder_mark))
		.add_systems(
			Update,
			(apply_capture_scroll, capture_scene, advance_after_shot, log_gallery_viewport).chain(),
		)
		.run();
}

fn capture_resolution() -> (u32, u32) {
	std::env::var("INVENTORY_GALLERY_RESOLUTION")
		.ok()
		.and_then(|value| {
			let (width, height) = value.split_once('x')?;
			Some((width.parse().ok()?, height.parse().ok()?))
		})
		.filter(|(width, height)| *width > 0 && *height > 0)
		.unwrap_or((960, 1080))
}

fn add_lod_viewer_to_camera(
	mut commands: Commands,
	cameras: Query<Entity, (With<Camera3d>, Without<LodViewer>)>,
) {
	for entity in &cameras {
		commands.entity(entity).insert(LodViewer);
	}
}

#[derive(Resource)]
struct CaptureDir(PathBuf);

#[derive(Resource, Default)]
struct CaptureShots {
	scene: CaptureScene,
	requested: bool,
	shown_at: Option<Duration>,
	icon_wait_started: Option<Duration>,
	failed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CaptureScene {
	#[default]
	Loading,
	WaitingForIcon,
	RealStarterKit,
	OverflowFixtureTop,
	OverflowFixtureScrolled,
	ModalInventory,
	ModalReorder,
	OverflowTanzEquipped,
	ModalLubikan,
	IconCloseup,
	Done,
}

impl CaptureScene {
	fn next(self) -> Self {
		match self {
			Self::Loading => Self::WaitingForIcon,
			Self::WaitingForIcon => Self::RealStarterKit,
			Self::RealStarterKit => Self::OverflowFixtureTop,
			Self::OverflowFixtureTop => Self::OverflowFixtureScrolled,
			Self::OverflowFixtureScrolled => Self::ModalInventory,
			Self::ModalInventory => Self::ModalReorder,
			Self::ModalReorder => Self::OverflowTanzEquipped,
			Self::OverflowTanzEquipped => Self::ModalLubikan,
			Self::ModalLubikan => Self::IconCloseup,
			Self::IconCloseup | Self::Done => Self::Done,
		}
	}

	fn file_stem(self) -> &'static str {
		match self {
			Self::Loading => "inventory_loading",
			Self::WaitingForIcon => "waiting",
			Self::RealStarterKit => "real_starter_kit",
			Self::OverflowFixtureTop => "overflow_fixture_top",
			Self::OverflowFixtureScrolled => "overflow_fixture_scrolled",
			Self::ModalInventory => "inventory_modal",
			Self::ModalReorder => "inventory_modal_reorder",
			Self::OverflowTanzEquipped => "inventory_overflow_tanzes",
			Self::ModalLubikan => "inventory_modal_lubikan",
			Self::IconCloseup => "self_lubikan_icon_closeup",
			Self::Done => "done",
		}
	}

	fn scroll_factor(self) -> f32 {
		match self {
			Self::OverflowFixtureScrolled | Self::ModalLubikan => 1.0,
			// Land on Tanzes so the narrow overlay still shows the overflowing
			// equipped Tanz row instead of Clothing at the top.
			Self::OverflowTanzEquipped => 0.72,
			_ => 0.0,
		}
	}
}

struct CatalogThumbs<'a> {
	icons: Option<&'a ItemIconCache>,
}

impl MenuThumbnailContext for CatalogThumbs<'_> {
	fn image_for_asset(
		&mut self,
		_label: &'static str,
		_asset_path: &'static str,
		_color: Color,
		_camera: character_ui_menu::ThumbnailCamera,
	) -> Option<Handle<Image>> {
		None
	}

	fn image_for_key(&mut self, key: u64) -> Option<Handle<Image>> {
		let icons = self.icons?;
		if catalog_key_uses_yellow_silhouette(key) {
			icons.silhouette_for_key(key)
		} else {
			icons.catalog_for_key(key)
		}
	}

	fn tint_for_key(&mut self, key: u64) -> Color {
		if catalog_key_uses_yellow_silhouette(key) {
			TEXT_YELLOW
		} else {
			Color::WHITE
		}
	}
}

fn shot_dir() -> PathBuf {
	std::env::var_os("INVENTORY_GALLERY_CAPTURE_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("/tmp/inventory-gallery"))
}

/// Generated Starter Kit: three clothing, two firearms, one grenade, one map,
/// Fisticuffs, and four base Tanzes.
fn real_starter_items() -> Vec<InventoryItem> {
	let mut rng = ItemRng::from_seed(42);
	rng.random_starter_loadout()
}

/// Scroll-demo fixture. Not granted by generation. Lives only in this example
/// and in `#[cfg(test)]` overflow coverage.
fn overflow_fixture_items() -> Vec<InventoryItem> {
	let sample = [
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Natural),
		InventoryItem::clothing(ClothingMesh::TankTop, ClothingMaterial::Cloth, ItemColor::Red),
		InventoryItem::clothing(ClothingMesh::Robe, ClothingMaterial::Cloth, ItemColor::Natural),
	];
	(0..12).map(|index| sample[index % sample.len()].clone()).collect()
}

fn sample_menu(items: Vec<InventoryItem>) -> CharacterMenu {
	let mut menu = CharacterMenu::for_create(items);
	menu.name = String::from("Misty");
	menu
}

/// Modal fixture: extra clothing stays unequipped, two tools, four Tanzes.
fn tanz_modal_items() -> Vec<InventoryItem> {
	let mut items = vec![
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Natural),
		InventoryItem::clothing(ClothingMesh::TankTop, ClothingMaterial::Cloth, ItemColor::Red),
		InventoryItem::clothing(ClothingMesh::Robe, ClothingMaterial::Cloth, ItemColor::Natural),
		InventoryItem::firearm(FirearmMesh::Bullpup),
		InventoryItem::firearm(FirearmMesh::Reltor),
	];
	items.extend(BASE_STARTER_TANZES.into_iter().map(InventoryItem::tanz));
	items
}

#[derive(Component)]
struct CaptureRoot;

fn setup(dir: Res<CaptureDir>) {
	let _ = std::fs::create_dir_all(&dir.0);
}

fn allow_slow_braidman_icon(mut cache: ResMut<CaptureCacheRoot>) {
	cache.max_wait_frames = 1800;
	// The offscreen render can report completion before the software backend
	// has finished warming materials and copies. Match the longer settle used
	// by the equipment capture fixture so first-run shaded icons are populated.
	cache.settle_frames = 30;
}

fn request_icon(
	mut discovery: ResMut<ItemIconDiscovery>,
	mut subjects: ResMut<ItemCaptureSubjects>,
	mut poses: ResMut<PoseCaptureSubjects>,
	mut cache: ResMut<ItemIconCache>,
	mut client: CaptureClient,
) {
	// Screen-capture is one session at a time. Pose stills must enqueue
	// before clothing / firearm hosts, which can sit unrealized for the
	// full wait window and starve Tanz thumbnails.
	let inventories = capture_icon_inventories();
	for inventory in &inventories {
		request_inventory_tanz_icons(
			inventory,
			&mut discovery,
			&mut poses,
			&mut client,
			&mut cache,
		);
	}
	request_self_lubikan_icon(&mut discovery, &mut client, &mut cache);
	for inventory in &inventories {
		request_inventory_icons(inventory, &mut discovery, &mut subjects, &mut client, &mut cache);
	}
}

fn capture_icon_inventories() -> [Inventory; 2] {
	[
		Inventory::with_starter_outfit(tanz_modal_items()),
		Inventory::with_starter_outfit(real_starter_items()),
	]
}

fn log_capture_outcome(
	mut failed: MessageReader<CaptureFailed>,
	mut completed: MessageReader<CaptureCompleted>,
) {
	for event in failed.read() {
		warn!(id = event.key.id.as_str(), error = %event.error, "item icon capture failed");
	}
	for event in completed.read() {
		info!(id = event.key.id.as_str(), "item icon capture completed");
	}
}

fn log_host_progress(
	time: Res<Time>,
	mut last: Local<u32>,
	hosts: Query<(
		Entity,
		&CaptureHostRoot,
		Option<&CaptureHostRealized>,
		Option<&CaptureHostReady>,
	)>,
	children: Query<&Children>,
	meshes: Query<Entity, (With<Mesh3d>, Allow<Disabled>)>,
	pending: Query<(), With<PendingCharacterAssembly>>,
	hidden: Query<(), With<HiddenUntilSceneReady>>,
) {
	let secs = time.elapsed_secs() as u32;
	if secs == *last || secs % 3 != 0 {
		return;
	}
	*last = secs;
	for (entity, host, realized, ready) in &hosts {
		let mut mesh_count = 0_u32;
		let mut pending_count = 0_u32;
		let mut hidden_count = 0_u32;
		fn walk(
			entity: Entity,
			children: &Query<&Children>,
			meshes: &Query<Entity, (With<Mesh3d>, Allow<Disabled>)>,
			pending: &Query<(), With<PendingCharacterAssembly>>,
			hidden: &Query<(), With<HiddenUntilSceneReady>>,
			mesh_count: &mut u32,
			pending_count: &mut u32,
			hidden_count: &mut u32,
		) {
			if meshes.get(entity).is_ok() {
				*mesh_count += 1;
			}
			if pending.get(entity).is_ok() {
				*pending_count += 1;
			}
			if hidden.get(entity).is_ok() {
				*hidden_count += 1;
			}
			if let Ok(kids) = children.get(entity) {
				for child in kids.iter() {
					walk(
						child,
						children,
						meshes,
						pending,
						hidden,
						mesh_count,
						pending_count,
						hidden_count,
					);
				}
			}
		}
		walk(
			entity,
			&children,
			&meshes,
			&pending,
			&hidden,
			&mut mesh_count,
			&mut pending_count,
			&mut hidden_count,
		);
		info!(
			id = %host.id.as_str(),
			kind = %host.kind,
			realized = realized.is_some(),
			ready = ready.is_some(),
			mesh_count,
			pending_count,
			hidden_count,
			"capture host progress"
		);
	}
}

fn icon_ready(cache: &ItemIconCache) -> bool {
	let catalog_ready = capture_icon_inventories().iter().all(|inventory| {
		InventoryLayout::starter_kit(inventory.items.clone())
			.captures
			.iter()
			.all(|need| {
				cache
					.catalog_entry(need.key, need.silhouette)
					.and_then(|entry| entry.ready_image())
					.is_some()
			})
	});
	catalog_ready && cache.silhouette_for_key(self_lubikan_catalog_key()).is_some()
}

const SCENE_DEADLINE: Duration = Duration::from_secs(48);

fn scene_deadline_hit(shown_at: Option<Duration>, now: Duration) -> bool {
	shown_at.is_some_and(|at| now > at + SCENE_DEADLINE)
}

fn spawn_grant_scene(
	commands: &mut Commands,
	fonts: &HudFonts,
	assets: &AssetServer,
	items: Vec<InventoryItem>,
	title: &str,
	icons: Option<&ItemIconCache>,
) {
	let layout = InventoryLayout::starter_kit(items);
	let mut viewport = Entity::PLACEHOLDER;
	let mut continue_parent = Entity::PLACEHOLDER;
	commands
		.spawn((
			CaptureRoot,
			SpinRevealScreen,
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				..default()
			},
			BackgroundColor(Color::srgb(0.04, 0.05, 0.07)),
		))
		.with_children(|root| {
			viewport = spawn_starter_kit_shell(
				root,
				fonts,
				title,
				TEXT_YELLOW,
				(
					maybraid_character_ui_menu_renderer::OverlaySelectRoot,
					InventoryMenuSession::default(),
				),
			);
			continue_parent = root
				.spawn((
					Node {
						width: Val::Percent(100.0),
						flex_direction: FlexDirection::Column,
						align_items: AlignItems::Center,
						padding: UiRect::bottom(Val::Px(24.0)),
						..default()
					},
					Pickable::IGNORE,
				))
				.id();
		});
	let mut thumbs = CatalogThumbs { icons };
	let slots = fill_inventory_screen(
		commands,
		viewport,
		fonts,
		assets,
		InputPromptDevice::Keyboard,
		&layout.node,
		false,
		&InventoryMenuSession::default(),
		&mut thumbs,
	);
	if let Ok(mut footer) = commands.get_entity(continue_parent) {
		footer.with_children(|footer| {
			spawn_hud_action(
				footer,
				fonts,
				"Continue",
				JustifyContent::Center,
				HudMenuItem { index: slots.item_count[0], menu: viewport },
			);
		});
	}
	if let Ok(mut viewport) = commands.get_entity(viewport) {
		viewport.insert((HudMenu::retain(slots.item_count[0] + 1, None), HudOverlayMenu));
	}
}

fn spawn_modal_scene(
	commands: &mut Commands,
	fonts: &HudFonts,
	assets: &AssetServer,
	icons: Option<&ItemIconCache>,
	items: Vec<InventoryItem>,
	session: InventoryMenuSession,
	unequip_back_two: bool,
	overlay_width: Option<f32>,
) {
	let mut menu = sample_menu(items);
	// Mixed shots: clothing and tools each keep one unequipped tile, and
	// Healing Salute / Left Strafe stay equipped so Tanzes also stacks.
	if unequip_back_two {
		if let Some(inventory) = menu.inventory.as_mut() {
			let drop: Vec<usize> = inventory
				.items
				.iter()
				.enumerate()
				.filter_map(|(index, item)| match item.tanz_id() {
					Some(TanzId::RightStrafe | TanzId::ForwardDash) => Some(index),
					_ => None,
				})
				.collect();
			for index in drop {
				let _ = inventory.unequip(index);
			}
			if let Some(index) = inventory.weapons.last().copied() {
				let _ = inventory.unequip(index);
			}
		}
	}
	let nodes = menu.menu_nodes();
	let Some(node) = find_overlay_node(&nodes, "Inventory").cloned() else {
		return;
	};
	let mut viewport = Entity::PLACEHOLDER;
	commands.insert_resource(menu_playground::CharacterMenuState(menu.clone()));
	commands
		.spawn((
			CaptureRoot,
			menu_playground::CharacterScreen,
			Node {
				width: overlay_width.map(Val::Px).unwrap_or(Val::Percent(100.0)),
				height: Val::Percent(100.0),
				..default()
			},
			BackgroundColor(Color::srgba(0.07, 0.08, 0.10, 0.38)),
		))
		.with_children(|root| {
			viewport = root
				.spawn((
					maybraid_character_ui_menu_renderer::OverlaySelectRoot,
					session,
					Node {
						position_type: PositionType::Absolute,
						left: Val::Px(0.0),
						top: Val::Px(0.0),
						width: Val::Percent(100.0),
						height: Val::Percent(100.0),
						..default()
					},
					Pickable::IGNORE,
				))
				.id();
		});
	let mut thumbs = CatalogThumbs { icons };
	fill_inventory_screen(
		commands,
		viewport,
		fonts,
		assets,
		InputPromptDevice::Keyboard,
		&node,
		true,
		&session,
		&mut thumbs,
	);
}

fn apply_reorder_mark(
	shots: Res<CaptureShots>,
	mut commands: Commands,
	tiles: Query<(Entity, &InventoryItemKey, Option<&ReorderMark>)>,
) {
	if shots.scene != CaptureScene::ModalReorder {
		return;
	}
	if tiles.iter().any(|(_, _, mark)| mark.is_some()) {
		return;
	}
	let Some((entity, _, _)) =
		tiles.iter().find(|(_, key, _)| key.region == InventoryRegion::Equipped)
	else {
		return;
	};
	commands.entity(entity).insert(ReorderMark);
}

fn spawn_icon_closeup(commands: &mut Commands, fonts: &HudFonts, image: Option<Handle<Image>>) {
	commands
		.spawn((
			CaptureRoot,
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				row_gap: Val::Px(20.0),
				..default()
			},
			BackgroundColor(Color::srgb(0.07, 0.08, 0.10)),
		))
		.with_children(|root| {
			spawn_header_line(root, fonts, "Self Lubikan", None, 42.0, TEXT_YELLOW);
			root.spawn((
				Node { width: Val::Px(384.0), height: Val::Px(384.0), ..default() },
				match image {
					Some(image) => ImageNode { image, color: TEXT_YELLOW, ..default() },
					None => ImageNode::default(),
				},
				BackgroundColor(Color::srgb(0.12, 0.13, 0.16)),
			));
		});
}

fn inventory_session(
	section: InventorySection,
	drawer: InventoryDrawerMode,
	region: InventoryRegion,
	scroll_rows: usize,
) -> InventoryMenuSession {
	let mut session = InventoryMenuSession { section, drawer, region, ..default() };
	let memory = &mut session.section_memory[section.index()][region.index()];
	memory.gallery_scroll = scroll_rows;
	if region == InventoryRegion::Drawer
		&& drawer == InventoryDrawerMode::Gallery
		&& scroll_rows > 0
	{
		// Keep focus in the last visible row for any saved scroll offset.
		let item_count = overflow_fixture_items().len();
		let key = InventoryItemKey {
			index: ((scroll_rows + INVENTORY_GALLERY_VISIBLE_ROWS - 1) * INVENTORY_GALLERY_COLUMNS)
				.min(item_count.saturating_sub(1)),
			equipped: false,
			region,
		};
		memory.focused_item = Some(key);
		session.focus.0 = Some(key);
	}
	session
}

fn spawn_scene(
	commands: &mut Commands,
	fonts: &HudFonts,
	assets: &AssetServer,
	scene: CaptureScene,
	icons: Option<&ItemIconCache>,
) {
	match scene {
		CaptureScene::RealStarterKit => {
			spawn_grant_scene(
				commands,
				fonts,
				assets,
				real_starter_items(),
				STARTER_KIT_LABEL,
				icons,
			);
		}
		CaptureScene::OverflowFixtureTop => {
			spawn_modal_scene(
				commands,
				fonts,
				assets,
				icons,
				overflow_fixture_items(),
				inventory_session(
					InventorySection::Clothing,
					InventoryDrawerMode::Gallery,
					InventoryRegion::Drawer,
					0,
				),
				true,
				None,
			);
		}
		CaptureScene::OverflowFixtureScrolled => {
			spawn_modal_scene(
				commands,
				fonts,
				assets,
				icons,
				overflow_fixture_items(),
				inventory_session(
					InventorySection::Clothing,
					InventoryDrawerMode::Gallery,
					InventoryRegion::Drawer,
					1,
				),
				true,
				None,
			);
		}
		CaptureScene::ModalInventory => {
			spawn_modal_scene(
				commands,
				fonts,
				assets,
				icons,
				tanz_modal_items(),
				inventory_session(
					InventorySection::Clothing,
					InventoryDrawerMode::Rotary,
					InventoryRegion::Drawer,
					0,
				),
				true,
				None,
			);
		}
		CaptureScene::ModalReorder => {
			let session = inventory_session(
				InventorySection::Tools,
				InventoryDrawerMode::Rotary,
				InventoryRegion::Equipped,
				0,
			);
			spawn_modal_scene(
				commands,
				fonts,
				assets,
				icons,
				tanz_modal_items(),
				session,
				true,
				None,
			);
		}
		CaptureScene::ModalLubikan => {
			spawn_modal_scene(
				commands,
				fonts,
				assets,
				icons,
				tanz_modal_items(),
				inventory_session(
					InventorySection::Lubikan,
					InventoryDrawerMode::Gallery,
					InventoryRegion::Drawer,
					0,
				),
				true,
				None,
			);
		}
		CaptureScene::OverflowTanzEquipped => {
			spawn_modal_scene(
				commands,
				fonts,
				assets,
				icons,
				tanz_modal_items(),
				inventory_session(
					InventorySection::Tanzes,
					InventoryDrawerMode::Rotary,
					InventoryRegion::Equipped,
					0,
				),
				false,
				None,
			);
		}
		CaptureScene::IconCloseup => {
			let image =
				icons.and_then(|cache| cache.silhouette_for_key(self_lubikan_catalog_key()));
			spawn_icon_closeup(commands, fonts, image);
		}
		CaptureScene::Loading => {
			let mut viewport = Entity::PLACEHOLDER;
			commands
				.spawn((
					CaptureRoot,
					SpinRevealScreen,
					Node {
						width: Val::Percent(100.0),
						height: Val::Percent(100.0),
						flex_direction: FlexDirection::Column,
						..default()
					},
					BackgroundColor(Color::srgb(0.04, 0.05, 0.07)),
				))
				.with_children(|root| {
					viewport =
						spawn_starter_kit_shell(root, fonts, STARTER_KIT_LABEL, TEXT_YELLOW, ());
				});
			fill_inventory_loading(commands, viewport, fonts);
		}
		CaptureScene::WaitingForIcon | CaptureScene::Done => {}
	}
}

fn apply_capture_scroll(
	shots: Res<CaptureShots>,
	mut viewports: Query<
		(&ComputedNode, &mut ScrollPosition),
		(With<HudScrollViewport>, Without<InventoryHudMenu>),
	>,
) {
	for (computed, mut scroll) in &mut viewports {
		let max = ((computed.content_size().y - computed.size().y)
			* computed.inverse_scale_factor())
		.max(0.0);
		scroll.y = max * shots.scene.scroll_factor();
	}
}

fn log_gallery_viewport(
	mut logged: Local<HashSet<Entity>>,
	viewports: Query<
		(Entity, &ComputedNode, &ScrollPosition),
		(With<InventoryHudMenu>, With<HudScrollViewport>),
	>,
) {
	for (entity, computed, scroll) in &viewports {
		let content = computed.content_size();
		let size = computed.size();
		if size.min_element() <= 0.0 || content.min_element() <= 0.0 {
			continue;
		}
		if logged.insert(entity) {
			let max_scroll = ((content.y - size.y) * computed.inverse_scale_factor()).max(0.0);
			info!(
				?entity,
				content_width = content.x,
				content_height = content.y,
				viewport_width = size.x,
				viewport_height = size.y,
				scroll_y = scroll.y,
				max_scroll,
				"inventory gallery viewport laid out"
			);
		}
	}
}

fn capture_scene(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<CaptureDir>,
	assets: Res<AssetServer>,
	cache: Res<ItemIconCache>,
	mut shots: ResMut<CaptureShots>,
	roots: Query<(), With<CaptureRoot>>,
	preview_roots: Query<
		&characters::CharacterMembers,
		With<menu_playground::CharacterPreviewRoot>,
	>,
	preview_entities: Query<(), With<menu_playground::CharacterPreviewRoot>>,
	mut exit: MessageWriter<AppExit>,
) {
	if shots.failed || shots.scene == CaptureScene::Done || shots.requested {
		return;
	}
	if shots.scene == CaptureScene::WaitingForIcon {
		let wait_started = *shots.icon_wait_started.get_or_insert(time.elapsed());
		if icon_ready(&cache) {
			let have = cache.silhouette_for_key(self_lubikan_catalog_key()).is_some();
			info!(have, secs = time.elapsed_secs(), "self-lubikan icon wait finished");
			shots.icon_wait_started = None;
			shots.scene = CaptureScene::RealStarterKit;
			let fonts = HudFonts::load(assets.as_ref());
			spawn_scene(&mut commands, &fonts, assets.as_ref(), shots.scene, Some(cache.as_ref()));
		} else if time.elapsed() > wait_started + Duration::from_secs(90) {
			shots.failed = true;
			error!(
				secs = time.elapsed_secs(),
				"item thumbnails did not become ready before capture deadline; see capture host diagnostics"
			);
			exit.write(AppExit::error());
		}
		return;
	}
	if roots.is_empty() {
		if shots.scene == CaptureScene::Loading {
			let fonts = HudFonts::load(assets.as_ref());
			spawn_scene(&mut commands, &fonts, assets.as_ref(), shots.scene, Some(cache.as_ref()));
		}
		return;
	}
	if shots.shown_at.is_none() {
		shots.shown_at = Some(time.elapsed());
	}
	if scene_needs_character_preview(shots.scene) && preview_roots.is_empty() {
		if scene_deadline_hit(shots.shown_at, time.elapsed()) {
			shots.failed = true;
			error!(
				scene = shots.scene.file_stem(),
				preview_roots = preview_entities.iter().count(),
				secs = time.elapsed_secs(),
				"character preview did not realize before capture deadline"
			);
			exit.write(AppExit::error());
		}
		return;
	}
	let settle_time = if scene_needs_character_preview(shots.scene) {
		Duration::from_secs(6)
	} else {
		Duration::from_millis(2200)
	};
	if shots.shown_at.is_none_or(|at| time.elapsed() <= at + settle_time) {
		return;
	}
	let path = dir.0.join(format!("{}.png", shots.scene.file_stem()));
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
	shots.requested = true;
}

fn scene_needs_character_preview(scene: CaptureScene) -> bool {
	matches!(
		scene,
		CaptureScene::OverflowFixtureTop
			| CaptureScene::OverflowFixtureScrolled
			| CaptureScene::ModalInventory
			| CaptureScene::ModalReorder
			| CaptureScene::OverflowTanzEquipped
			| CaptureScene::ModalLubikan
	)
}

#[allow(clippy::too_many_arguments)]
fn advance_after_shot(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<CaptureDir>,
	assets: Res<AssetServer>,
	cache: Res<ItemIconCache>,
	mut shots: ResMut<CaptureShots>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
	mut ready_at: Local<Option<Duration>>,
	roots: Query<Entity, With<CaptureRoot>>,
) {
	if shots.failed {
		return;
	}
	if shots.scene == CaptureScene::Done {
		if time.elapsed_secs() > 16.0 {
			exit.write(AppExit::Success);
		}
		return;
	}
	if !shots.requested {
		if shots.scene != CaptureScene::WaitingForIcon
			&& scene_deadline_hit(shots.shown_at, time.elapsed())
		{
			shots.failed = true;
			error!(scene = shots.scene.file_stem(), "capture scene timed out before screenshot");
			exit.write(AppExit::error());
		}
		return;
	}
	let path = dir.0.join(format!("{}.png", shots.scene.file_stem()));
	if path.is_file() && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if scene_deadline_hit(shots.shown_at, time.elapsed())
		&& !ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(250))
	{
		shots.failed = true;
		error!(
			scene = shots.scene.file_stem(),
			"screenshot did not finish before capture deadline"
		);
		exit.write(AppExit::error());
		return;
	}
	if !ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(250)) {
		return;
	}
	for entity in &roots {
		commands.entity(entity).try_despawn();
	}
	shots.requested = false;
	shots.shown_at = None;
	*ready_at = None;
	shots.scene = shots.scene.next();
	if shots.scene == CaptureScene::Done {
		exit.write(AppExit::Success);
		return;
	}
	let fonts = HudFonts::load(assets.as_ref());
	spawn_scene(&mut commands, &fonts, assets.as_ref(), shots.scene, Some(cache.as_ref()));
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::SELF_LUBIKAN_CAPTURE_LABEL;
	use item_icons::CaptureId;

	#[test]
	fn gallery_waits_for_the_self_lubikan_silhouette() {
		let mut cache = ItemIconCache::default();
		assert!(!icon_ready(&cache));
		cache.insert_silhouette(CaptureId::new(SELF_LUBIKAN_CAPTURE_LABEL), Handle::default());
		assert!(!icon_ready(&cache));
		for inventory in capture_icon_inventories() {
			for item in inventory.items {
				if let Some(id) = item.tanz_id() {
					use screen_capture::RepresentativePose;
					cache
						.insert_silhouette(tanz_user::TanzPose(id).capture_id(), Handle::default());
				} else if let Some(label) = item.capture_id_label() {
					let id = CaptureId::new(label);
					if item.catalog_thumb_is_silhouette() {
						cache.insert_silhouette(id.clone(), Handle::default());
					}
					cache.insert_shaded(id, Handle::default());
				}
			}
		}
		assert!(icon_ready(&cache));
	}

	#[test]
	fn scene_deadlines_use_shown_at_not_total_app_time() {
		let shown_at = Some(Duration::from_secs(50));
		assert!(!scene_deadline_hit(shown_at, Duration::from_secs(60)));
		assert!(scene_deadline_hit(shown_at, Duration::from_secs(99)));
		assert!(!scene_deadline_hit(None, Duration::from_secs(90)));
	}
}
