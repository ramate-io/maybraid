//! Headless captures of the Fisticuffs inventory icon and equipment switch.
//!
//! ```bash
//! FISTICUFFS_CAPTURE_DIR=/tmp/fisticuffs-equipment \
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   WGPU_BACKEND=vk \
//!   xvfb-run -a cargo run -p menu-playground --example fisticuffs_equipment_capture
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::audio::AudioPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use character_creation_menus::InventoryLayout;
use character_items::{
	catalog_key_uses_yellow_silhouette, fisticuffs_catalog_key, self_lubikan_catalog_key,
	FirearmMesh, Inventory, InventoryItem,
};
use item_icons::{
	request_inventory_icons, CaptureCacheRoot, CaptureClient, CaptureCompleted, CaptureFailed,
	CaptureHostReady, CaptureHostRealized, CaptureHostRoot, ItemCaptureSubjects, ItemIconCache,
	ItemIconDiscovery, ItemIconPlugin,
};
use maybraid_character_ui_menu_renderer::{
	fill_inventory_layout, spawn_overlay_shell, MenuThumbnailContext,
};
use menu_components::theme::TEXT_YELLOW;
use menu_components::{spawn_header_line, HudFonts, InputPromptDevice};

fn assets_path() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Fisticuffs equipment capture".into(),
						resolution: (960, 720).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path().to_string_lossy().into(), ..default() })
				.disable::<AudioPlugin>(),
		)
		.add_plugins(ItemIconPlugin)
		.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.10)))
		.insert_resource(CaptureDir(shot_dir()))
		.init_resource::<CaptureShots>()
		.add_systems(Startup, (setup, allow_slow_braidman_icon, request_icon))
		.add_systems(Update, (log_capture_outcome, log_host_progress))
		.add_systems(Update, (capture_scene, advance_after_shot).chain())
		.run();
}

#[derive(Resource)]
struct CaptureDir(PathBuf);

#[derive(Resource, Default)]
struct CaptureShots {
	scene: CaptureScene,
	requested: bool,
	shown_at: Option<Duration>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CaptureScene {
	#[default]
	WaitingForIcon,
	ArmsIcon,
	FisticuffsSelected,
	SwitchedToGun,
	Done,
}

impl CaptureScene {
	fn next(self) -> Self {
		match self {
			Self::WaitingForIcon => Self::ArmsIcon,
			Self::ArmsIcon => Self::FisticuffsSelected,
			Self::FisticuffsSelected => Self::SwitchedToGun,
			Self::SwitchedToGun | Self::Done => Self::Done,
		}
	}

	fn file_stem(self) -> &'static str {
		match self {
			Self::WaitingForIcon => "waiting",
			Self::ArmsIcon => "fisticuffs_arms_icon",
			Self::FisticuffsSelected => "fisticuffs_selected",
			Self::SwitchedToGun => "switched_to_gun",
			Self::Done => "done",
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
		let cache = self.icons?;
		if catalog_key_uses_yellow_silhouette(key) {
			cache.silhouette_for_key(key)
		} else {
			cache.shaded_for_key(key)
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
	std::env::var_os("FISTICUFFS_CAPTURE_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("/tmp/fisticuffs-equipment"))
}

fn switch_bag(fisticuffs_primary: bool) -> Inventory {
	let items = vec![InventoryItem::Fisticuffs, InventoryItem::firearm(FirearmMesh::Bullpup)];
	let weapons = if fisticuffs_primary { vec![0, 1] } else { vec![1, 0] };
	Inventory { items, clothing: Vec::new(), weapons, skills: Vec::new(), tanzes: Vec::new() }
}

fn capture_icon_inventories() -> [Inventory; 2] {
	[switch_bag(true), switch_bag(false)]
}

#[derive(Component)]
struct CaptureRoot;

fn setup(mut commands: Commands, dir: Res<CaptureDir>) {
	let _ = std::fs::create_dir_all(&dir.0);
	commands.spawn(Camera2d);
}

fn allow_slow_braidman_icon(mut cache: ResMut<CaptureCacheRoot>) {
	cache.max_wait_frames = 3600;
	cache.settle_frames = 24;
}

fn request_icon(
	mut discovery: ResMut<ItemIconDiscovery>,
	mut subjects: ResMut<ItemCaptureSubjects>,
	mut cache: ResMut<ItemIconCache>,
	mut client: CaptureClient,
) {
	for inventory in capture_icon_inventories() {
		request_inventory_icons(&inventory, &mut discovery, &mut subjects, &mut client, &mut cache);
	}
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
	meshes: Query<Entity, With<Mesh3d>>,
) {
	let secs = time.elapsed_secs() as u32;
	if secs == *last || secs % 3 != 0 {
		return;
	}
	*last = secs;
	for (entity, host, realized, ready) in &hosts {
		let mut mesh_count = 0_u32;
		fn walk(
			entity: Entity,
			children: &Query<&Children>,
			meshes: &Query<Entity, With<Mesh3d>>,
			count: &mut u32,
		) {
			if meshes.get(entity).is_ok() {
				*count += 1;
			}
			if let Ok(kids) = children.get(entity) {
				for child in kids.iter() {
					walk(child, children, meshes, count);
				}
			}
		}
		walk(entity, &children, &meshes, &mut mesh_count);
		info!(
			kind = %host.kind,
			realized = realized.is_some(),
			ready = ready.is_some(),
			mesh_count,
			"capture host progress"
		);
	}
}

fn icon_ready(cache: &ItemIconCache) -> bool {
	cache.silhouette_for_key(fisticuffs_catalog_key()).is_some()
		&& cache.silhouette_for_key(self_lubikan_catalog_key()).is_some()
}

const SCENE_DEADLINE: Duration = Duration::from_secs(48);

fn scene_deadline_hit(shown_at: Option<Duration>, now: Duration) -> bool {
	shown_at.is_some_and(|at| now > at + SCENE_DEADLINE)
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
			spawn_header_line(root, fonts, "Fisticuffs", None, 42.0, TEXT_YELLOW);
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

fn spawn_inventory_scene(
	commands: &mut Commands,
	fonts: &HudFonts,
	assets: &AssetServer,
	inventory: &Inventory,
	icons: Option<&ItemIconCache>,
) {
	let layout = InventoryLayout::character_menu(inventory);
	let mut viewport = Entity::PLACEHOLDER;
	commands
		.spawn((
			CaptureRoot,
			Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
			BackgroundColor(Color::srgb(0.07, 0.08, 0.10)),
		))
		.with_children(|root| {
			viewport = spawn_overlay_shell(
				root,
				fonts,
				"Inventory",
				TEXT_YELLOW,
				Some((assets, InputPromptDevice::Keyboard)),
			);
		});
	let mut thumbs = CatalogThumbs { icons };
	fill_inventory_layout(commands, viewport, fonts, &layout.node, true, None, None, &mut thumbs);
}

fn spawn_scene(
	commands: &mut Commands,
	fonts: &HudFonts,
	assets: &AssetServer,
	scene: CaptureScene,
	icons: Option<&ItemIconCache>,
) {
	match scene {
		CaptureScene::ArmsIcon => {
			let image = icons.and_then(|cache| cache.silhouette_for_key(fisticuffs_catalog_key()));
			spawn_icon_closeup(commands, fonts, image);
		}
		CaptureScene::FisticuffsSelected => {
			spawn_inventory_scene(commands, fonts, assets, &switch_bag(true), icons);
		}
		CaptureScene::SwitchedToGun => {
			spawn_inventory_scene(commands, fonts, assets, &switch_bag(false), icons);
		}
		CaptureScene::WaitingForIcon | CaptureScene::Done => {}
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
) {
	if shots.scene == CaptureScene::Done || shots.requested {
		return;
	}
	if shots.scene == CaptureScene::WaitingForIcon {
		if icon_ready(&cache) || time.elapsed_secs() > 90.0 {
			let have = cache.silhouette_for_key(fisticuffs_catalog_key()).is_some();
			info!(have, secs = time.elapsed_secs(), "fisticuffs icon wait finished");
			shots.scene = CaptureScene::ArmsIcon;
			let fonts = HudFonts::load(assets.as_ref());
			spawn_scene(&mut commands, &fonts, assets.as_ref(), shots.scene, Some(cache.as_ref()));
		}
		return;
	}
	if roots.is_empty() {
		return;
	}
	if shots.shown_at.is_none() {
		shots.shown_at = Some(time.elapsed());
	}
	if shots
		.shown_at
		.is_none_or(|at| time.elapsed() <= at + Duration::from_millis(2200))
	{
		return;
	}
	let path = dir.0.join(format!("{}.png", shots.scene.file_stem()));
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
	shots.requested = true;
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
			exit.write(AppExit::Success);
		}
		return;
	}
	let path = dir.0.join(format!("{}.png", shots.scene.file_stem()));
	if path.is_file() && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if !(ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(250))
		|| scene_deadline_hit(shots.shown_at, time.elapsed()))
	{
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
	use character_items::FISTICUFFS_CAPTURE_LABEL;
	use item_icons::CaptureId;

	#[test]
	fn capture_waits_for_fisticuffs_and_self_lubikan() {
		use character_items::SELF_LUBIKAN_CAPTURE_LABEL;

		let mut cache = ItemIconCache::default();
		assert!(!icon_ready(&cache));
		cache.insert_silhouette(CaptureId::new(SELF_LUBIKAN_CAPTURE_LABEL), Handle::default());
		assert!(!icon_ready(&cache), "Fisticuffs silhouette must enter the wait set");
		cache.insert_silhouette(CaptureId::new(FISTICUFFS_CAPTURE_LABEL), Handle::default());
		assert!(icon_ready(&cache));
	}

	#[test]
	fn switch_bags_keep_one_fisticuffs_and_change_primary() {
		let fists = switch_bag(true);
		let gun = switch_bag(false);
		assert!(fists.primary_weapon().is_some_and(InventoryItem::is_fisticuffs));
		assert_eq!(
			gun.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
		assert_eq!(fists.items.iter().filter(|item| item.is_fisticuffs()).count(), 1);
		assert_eq!(gun.items.iter().filter(|item| item.is_fisticuffs()).count(), 1);
	}
}
