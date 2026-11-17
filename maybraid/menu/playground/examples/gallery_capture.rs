//! Capture the character gallery keybind row and delete confirmation.
//!
//! ```bash
//! GALLERY_CAPTURE_DIR=/tmp/gallery-menu \
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   WGPU_BACKEND=vk \
//!   xvfb-run -a cargo run -p menu-playground --example gallery_capture --release --locked
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::audio::AudioPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use character_creation_menus::{ConceptSpecies, MenuEvent};
use character_persist::{CharacterId, SaveRoot};
use characters::{
	CharacterHostsPlugin, HiddenUntilSceneReady, PendingCharacterAssembly,
	StandaloneCharacterMaterialRefPlugin,
};
use lod::LodViewer;
use maybraid_menu_controller::MenuControllerPlugin;
use maybraid_virtual_pad_controller::PadHidPolicy;
use menu_components::{InputPromptDevice, MenuNav, MenuNavImpulse, TextMenu, MENU_CLEAR};
use menu_playground::{
	save_editing_character, CharacterMenuState, CharacterPreviewPlugin, CharacterPreviewRoot,
	CharacterSessionPlugin, GalleryLineupFigure,
};
use menu_screens::{request_show_gallery, GalleryChoice, GalleryScreen};

fn assets_path() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	let root = scratch_root();
	seed_roster(&root).expect("seed gallery roster");
	App::new()
		.insert_resource(PadHidPolicy::External)
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Character gallery capture".into(),
						resolution: (1280, 720).into(),
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
		.add_plugins(CharacterPreviewPlugin)
		.insert_resource(ClearColor(MENU_CLEAR))
		.insert_resource(root)
		.init_resource::<CharacterMenuState>()
		.insert_resource(CaptureDir(shot_dir()))
		.init_resource::<CaptureShots>()
		.add_plugins((MenuControllerPlugin, CharacterSessionPlugin))
		.add_systems(
			Startup,
			(
				setup,
				(
					characters_playground::camera::setup_camera,
					add_lod_viewer_to_camera.after(characters_playground::camera::setup_camera),
				)
					.chain(),
			),
		)
		.add_systems(Update, (capture_shots, quit_after_shots).chain())
		.run();
}

fn add_lod_viewer_to_camera(
	mut commands: Commands,
	cameras: Query<Entity, (With<Camera3d>, Without<LodViewer>)>,
) {
	for entity in &cameras {
		commands.entity(entity).insert(LodViewer);
	}
}

fn scratch_root() -> SaveRoot {
	SaveRoot::at(std::env::temp_dir().join(format!(
		"maybraid-gallery-capture-{}-{}",
		std::process::id(),
		CharacterId::new().to_hex()
	)))
}

fn seed_roster(root: &SaveRoot) -> anyhow::Result<()> {
	for (id, name, species) in [
		(CharacterId(11), "Jeff", ConceptSpecies::Braidman),
		(CharacterId(12), "Roamer", ConceptSpecies::Brodler),
	] {
		let mut menu = CharacterMenuState::default();
		menu.0.name = name.into();
		let _ = menu.0.apply(MenuEvent::SetSpecies(species));
		save_editing_character(root, id, &menu.0)?;
	}
	character_persist::save_active(root, CharacterId(11))?;
	Ok(())
}

fn shot_dir() -> PathBuf {
	std::env::var_os("GALLERY_CAPTURE_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("/tmp/gallery-menu"))
}

#[derive(Resource)]
struct CaptureDir(PathBuf);

#[derive(Resource, Default)]
struct CaptureShots {
	ready_at: Option<f32>,
	keyboard: bool,
	pad: bool,
	wheel_nav: bool,
	wheel: bool,
	confirm_open: bool,
	confirm: bool,
}

fn setup(mut commands: Commands, dir: Res<CaptureDir>) {
	let _ = std::fs::create_dir_all(&dir.0);
	request_show_gallery(&mut commands);
}

fn lineup_ready(
	figures: Query<(), With<GalleryLineupFigure>>,
	roots: Query<(), With<CharacterPreviewRoot>>,
	pending: Query<(), With<PendingCharacterAssembly>>,
	hidden: Query<(), With<HiddenUntilSceneReady>>,
	meshes: Query<(), With<Mesh3d>>,
) -> bool {
	figures.iter().len() >= 2
		&& roots.iter().len() >= 2
		&& pending.is_empty()
		&& hidden.is_empty()
		&& meshes.iter().len() >= 8
}

fn capture_shots(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<CaptureDir>,
	screens: Query<(), With<GalleryScreen>>,
	mut device: ResMut<InputPromptDevice>,
	mut shots: ResMut<CaptureShots>,
	menus: Query<(Entity, &TextMenu)>,
	mut choices: MessageWriter<GalleryChoice>,
	figures: Query<(), With<GalleryLineupFigure>>,
	roots: Query<(), With<CharacterPreviewRoot>>,
	pending: Query<(), With<PendingCharacterAssembly>>,
	hidden: Query<(), With<HiddenUntilSceneReady>>,
	meshes: Query<(), With<Mesh3d>>,
) {
	if screens.is_empty() {
		return;
	}
	if shots.ready_at.is_none() {
		if lineup_ready(figures, roots, pending, hidden, meshes) {
			shots.ready_at = Some(time.elapsed_secs());
		}
		return;
	}
	let Some(ready_at) = shots.ready_at else {
		return;
	};
	let elapsed = time.elapsed_secs() - ready_at;
	if !shots.keyboard && elapsed >= 2.4 {
		*device = InputPromptDevice::Keyboard;
		commands
			.spawn(Screenshot::primary_window())
			.observe(save_to_disk(dir.0.join("01-gallery-keyboard.png")));
		shots.keyboard = true;
	}
	if shots.keyboard && !shots.pad && elapsed >= 3.0 {
		*device = InputPromptDevice::Pad;
	}
	if shots.keyboard && !shots.pad && elapsed >= 3.7 {
		commands
			.spawn(Screenshot::primary_window())
			.observe(save_to_disk(dir.0.join("02-gallery-pad.png")));
		shots.pad = true;
	}
	if shots.pad && !shots.wheel_nav && elapsed >= 4.1 {
		if let Some((entity, _)) = menus.iter().next() {
			commands.trigger(MenuNavImpulse::new(entity, MenuNav::Right));
		}
		shots.wheel_nav = true;
	}
	if shots.wheel_nav && !shots.wheel && elapsed >= 5.0 {
		commands
			.spawn(Screenshot::primary_window())
			.observe(save_to_disk(dir.0.join("04-gallery-wheel.png")));
		shots.wheel = true;
	}
	if shots.wheel && !shots.confirm_open && elapsed >= 5.2 {
		choices.write(GalleryChoice::Delete);
		shots.confirm_open = true;
	}
	if shots.confirm_open && !shots.confirm && elapsed >= 6.0 {
		commands
			.spawn(Screenshot::primary_window())
			.observe(save_to_disk(dir.0.join("03-gallery-delete-confirm.png")));
		shots.confirm = true;
	}
}

fn request_exists(path: impl AsRef<Path>) -> bool {
	path.as_ref().is_file()
}

fn quit_after_shots(
	time: Res<Time>,
	shots: Res<CaptureShots>,
	dir: Res<CaptureDir>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
	mut done_at: Local<Option<Duration>>,
) {
	let done = shots.confirm
		&& request_exists(dir.0.join("01-gallery-keyboard.png"))
		&& request_exists(dir.0.join("02-gallery-pad.png"))
		&& request_exists(dir.0.join("03-gallery-delete-confirm.png"))
		&& request_exists(dir.0.join("04-gallery-wheel.png"))
		&& capturing.is_empty();
	if done && done_at.is_none() {
		*done_at = Some(time.elapsed());
	}
	if done_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(250))
		|| time.elapsed() > Duration::from_secs(45)
	{
		exit.write(AppExit::Success);
	}
}
