//! GPU stills of the RB-held overlay lens (idle, fire film, unravel, re-plait).
//!
//! Real compositor + authored skill-map tiles + a 3D world behind the hole:
//!
//! ```sh
//! VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   WGPU_BACKEND=vk \
//!   xvfb-run -a cargo run -p maybraid-skill-map --example overlay_lens_capture --release --locked
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::audio::AudioPlugin;
use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use bevy::ui::widget::ViewportNode;
use maybraid_skill_map::{
	authored_map, map_view_extent, spawn_skill_map_view, OverlayParams, SkillKind, SkillMapMember,
	SkillMapOverlay, SkillMapOverlayLens, SkillMapOverlayMaterial, SkillMapOverlayPlugin,
	SkillMapTileAssets, SkillMapTileMaterial, SkillMapTileMaterialPlugin, OVERLAY_TILE_AMOUNT,
};

fn captures_dir() -> PathBuf {
	std::env::var_os("OVERLAY_LENS_CAPTURE_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("captures"))
}

fn assets_path() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	let dir = captures_dir();
	let _ = std::fs::create_dir_all(&dir);
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Skill-map overlay lens capture".into(),
						resolution: (1280, 720).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path().to_string_lossy().into(), ..default() })
				.disable::<AudioPlugin>(),
		)
		.add_plugins(SkillMapTileMaterialPlugin)
		.add_plugins(SkillMapOverlayPlugin)
		.insert_resource(ClearColor(Color::srgb(0.38, 0.58, 0.86)))
		.insert_resource(CaptureDir(dir))
		.init_resource::<CaptureShots>()
		.add_systems(PostStartup, setup)
		.add_systems(Update, (drive_lens, capture_scene, advance_after_shot).chain())
		.run();
}

#[derive(Resource)]
struct CaptureDir(PathBuf);

#[derive(Resource)]
struct OverlayCapture {
	material: Handle<SkillMapOverlayMaterial>,
}

#[derive(Resource, Default)]
struct CaptureShots {
	scene: CaptureScene,
	requested: bool,
	shown_at: Option<Duration>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CaptureScene {
	#[default]
	Idle,
	FireFlash,
	Unravel,
	Replait,
	Done,
}

impl CaptureScene {
	fn next(self) -> Self {
		match self {
			Self::Idle => Self::FireFlash,
			Self::FireFlash => Self::Unravel,
			Self::Unravel => Self::Replait,
			Self::Replait | Self::Done => Self::Done,
		}
	}

	fn file_stem(self) -> &'static str {
		match self {
			Self::Idle => "overlay-lens-idle",
			Self::FireFlash => "overlay-lens-fire-flash",
			Self::Unravel => "overlay-lens-debraid-uncurl",
			Self::Replait => "overlay-lens-debraid-replait",
			Self::Done => "done",
		}
	}

	fn lens(self) -> SkillMapOverlayLens {
		match self {
			Self::Idle | Self::Done => SkillMapOverlayLens::default(),
			Self::FireFlash => SkillMapOverlayLens::peak_flash(),
			Self::Unravel => SkillMapOverlayLens::mid_uncurl(),
			Self::Replait => SkillMapOverlayLens::mid_replait(),
		}
	}

	fn warmup(self) -> Duration {
		match self {
			Self::Idle => Duration::from_millis(1400),
			_ => Duration::from_millis(350),
		}
	}
}

fn setup(
	mut commands: Commands,
	mut images: ResMut<Assets<Image>>,
	mut overlays: ResMut<Assets<SkillMapOverlayMaterial>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut standards: ResMut<Assets<StandardMaterial>>,
	mut tiles: ResMut<Assets<SkillMapTileMaterial>>,
	tile_assets: Res<SkillMapTileAssets>,
) {
	for handle in [
		tile_assets.land_fire.clone(),
		tile_assets.land_wave.clone(),
		tile_assets.land_rock.clone(),
		tile_assets.land_cosmo.clone(),
		tile_assets.water.clone(),
		tile_assets.fireball.clone(),
		tile_assets.dumbwave.clone(),
		tile_assets.rockadder.clone(),
		tile_assets.cosimo.clone(),
		tile_assets.cursor.clone(),
	] {
		if let Some(mut material) = tiles.get_mut(&handle) {
			material.set_overlay_amount(OVERLAY_TILE_AMOUNT);
		}
	}

	commands.spawn((
		Camera3d::default(),
		Transform::from_xyz(0.0, 3.4, 7.2).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y),
	));
	commands.spawn((
		DirectionalLight { illuminance: 12_000.0, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.6, 0.0)),
	));
	commands.spawn((
		Mesh3d(meshes.add(Plane3d::default().mesh().size(28.0, 28.0))),
		MeshMaterial3d(standards.add(Color::srgb(0.30, 0.52, 0.24))),
		Transform::from_xyz(0.0, 0.0, 0.0),
	));
	commands.spawn((
		Mesh3d(meshes.add(Cuboid::new(1.1, 1.6, 1.1))),
		MeshMaterial3d(standards.add(Color::srgb(0.82, 0.22, 0.16))),
		Transform::from_xyz(0.0, 0.8, 0.0),
	));
	commands.spawn((
		Mesh3d(meshes.add(Cuboid::new(0.8, 0.8, 0.8))),
		MeshMaterial3d(standards.add(Color::srgb(0.90, 0.78, 0.22))),
		Transform::from_xyz(1.6, 0.4, -0.8),
	));

	let user = commands.spawn_empty().id();
	let session = commands.spawn_empty().id();
	let member = SkillMapMember { user, session };
	let mut projection = OrthographicProjection::default_2d();
	let extent = map_view_extent();
	projection.scaling_mode = ScalingMode::Fixed { width: extent.x, height: extent.y };
	projection.scale = 1.0;
	let view = spawn_skill_map_view(
		&mut commands,
		&mut images,
		&tile_assets,
		authored_map(SkillKind::Fireball, 11),
		member,
		-2,
		projection,
		(),
	);
	let material = overlays.add(SkillMapOverlayMaterial::new(view.image.clone()));
	let params =
		OverlayParams::from_overlay_lens(SkillMapOverlay::Expanded, SkillMapOverlayLens::default());
	if let Some(mut overlay) = overlays.get_mut(&material) {
		overlay.params = params;
	}
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				right: Val::Px(0.0),
				top: Val::Px(0.0),
				bottom: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				..default()
			},
			BackgroundColor(Color::NONE),
		))
		.with_children(|root| {
			root.spawn((
				ViewportNode::new(view.camera),
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(0.0),
					right: Val::Px(0.0),
					top: Val::Px(0.0),
					bottom: Val::Px(0.0),
					..default()
				},
				Visibility::Hidden,
			));
			root.spawn((MaterialNode(material.clone()), SkillMapOverlay::Expanded.pane_node()));
		});
	commands.insert_resource(OverlayCapture { material });
}

fn drive_lens(
	shots: Res<CaptureShots>,
	capture: Res<OverlayCapture>,
	mut overlays: ResMut<Assets<SkillMapOverlayMaterial>>,
) {
	if shots.scene == CaptureScene::Done {
		return;
	}
	if let Some(mut material) = overlays.get_mut(&capture.material) {
		material.params =
			OverlayParams::from_overlay_lens(SkillMapOverlay::Expanded, shots.scene.lens());
	}
}

fn capture_scene(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<CaptureDir>,
	mut shots: ResMut<CaptureShots>,
) {
	if shots.scene == CaptureScene::Done || shots.requested {
		return;
	}
	if shots.shown_at.is_none() {
		shots.shown_at = Some(time.elapsed());
	}
	let Some(shown) = shots.shown_at else {
		return;
	};
	if time.elapsed() < shown + shots.scene.warmup() {
		return;
	}
	let path = dir.0.join(format!("{}.png", shots.scene.file_stem()));
	info!(path = %path.display(), scene = ?shots.scene, "capturing overlay lens still");
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
	shots.requested = true;
}

fn advance_after_shot(
	time: Res<Time>,
	dir: Res<CaptureDir>,
	mut shots: ResMut<CaptureShots>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
	mut ready_at: Local<Option<Duration>>,
) {
	if shots.scene == CaptureScene::Done {
		exit.write(AppExit::Success);
		return;
	}
	if !shots.requested {
		if shots.shown_at.is_some_and(|at| time.elapsed() > at + Duration::from_secs(12)) {
			exit.write(AppExit::Success);
		}
		return;
	}
	let path = dir.0.join(format!("{}.png", shots.scene.file_stem()));
	if path.is_file() && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if !(ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(200))
		|| shots.shown_at.is_some_and(|at| time.elapsed() > at + Duration::from_secs(10)))
	{
		return;
	}
	shots.requested = false;
	shots.shown_at = None;
	*ready_at = None;
	shots.scene = shots.scene.next();
	if shots.scene == CaptureScene::Done {
		exit.write(AppExit::Success);
	}
}
