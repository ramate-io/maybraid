//! GPU stills of the RB-held overlay lens across authored skill maps.
//!
//! Real compositor + authored tiles / DDR feed + a 3D world behind the hole:
//!
//! ```sh
//! VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   WGPU_BACKEND=vk \
//!   xvfb-run -a cargo run -p maybraid-skill-map --example overlay_lens_capture --release --locked
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::audio::AudioPlugin;
use bevy::camera::{ClearColorConfig, ScalingMode};
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use bevy::ui::widget::ViewportNode;
use bevy::window::WindowResolution;
use character_items::{SkillMapKind, SkillMapSpec};
use maybraid_skill_map::{
	authored_map_from_spec, map_view_extent, spawn_skill_map_view, OverlayParams, SkillMapBackdrop,
	SkillMapMember, SkillMapOverlay, SkillMapOverlayLens, SkillMapOverlayMaterial,
	SkillMapOverlayPlugin, SkillMapTileAssets, SkillMapTileMaterial, SkillMapTileMaterialPlugin,
	OVERLAY_TILE_AMOUNT,
};

const CAPTURE_WIDTH: u32 = 1920;
const CAPTURE_HEIGHT: u32 = 1080;

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
	let frac = SkillMapOverlayLens::min_axis_radius_frac();
	info!(
		lens_min_axis_frac = frac,
		radius_px_1080 = frac * CAPTURE_HEIGHT as f32,
		width = CAPTURE_WIDTH,
		height = CAPTURE_HEIGHT,
		"overlay lens capture: pocket-derived screen fraction"
	);
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Skill-map overlay lens capture".into(),
						resolution: WindowResolution::new(CAPTURE_WIDTH, CAPTURE_HEIGHT)
							.with_scale_factor_override(1.0),
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
		.add_systems(
			Update,
			(rebuild_map, capture_map_overlay_clear, drive_lens, capture_scene, advance_after_shot)
				.chain(),
		)
		.run();
}

#[derive(Resource)]
struct CaptureDir(PathBuf);

#[derive(Resource)]
struct OverlayCapture {
	material: Handle<SkillMapOverlayMaterial>,
	viewport: Entity,
	member: SkillMapMember,
}

#[derive(Component)]
struct CaptureMapCamera;

#[derive(Component)]
struct CaptureViewport;

#[derive(Resource)]
struct CaptureShots {
	map: usize,
	pose: usize,
	requested: bool,
	shown_at: Option<Duration>,
	rebuild: bool,
}

impl Default for CaptureShots {
	fn default() -> Self {
		Self { map: 0, pose: 0, requested: false, shown_at: None, rebuild: false }
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CapturePose {
	Idle,
	FireFlash,
	Unravel,
	Replait,
}

impl CapturePose {
	const ALL: [Self; 4] = [Self::Idle, Self::FireFlash, Self::Unravel, Self::Replait];

	fn slug(self) -> &'static str {
		match self {
			Self::Idle => "idle",
			Self::FireFlash => "fire-flash",
			Self::Unravel => "debraid-uncurl",
			Self::Replait => "debraid-replait",
		}
	}

	fn lens(self) -> SkillMapOverlayLens {
		match self {
			Self::Idle => SkillMapOverlayLens::default(),
			Self::FireFlash => SkillMapOverlayLens::sweep_flash(),
			Self::Unravel => SkillMapOverlayLens::mid_uncurl(),
			Self::Replait => SkillMapOverlayLens::mid_replait(),
		}
	}

	fn warmup(self, first_of_map: bool) -> Duration {
		if first_of_map && self == Self::Idle {
			Duration::from_millis(2800)
		} else {
			Duration::from_millis(400)
		}
	}
}

fn current_kind(shots: &CaptureShots) -> Option<SkillMapKind> {
	SkillMapKind::VALUES.get(shots.map).copied()
}

fn current_pose(shots: &CaptureShots) -> Option<CapturePose> {
	CapturePose::ALL.get(shots.pose).copied()
}

fn file_stem(kind: SkillMapKind, pose: CapturePose) -> String {
	format!("overlay-lens-{}-{}", kind.label(), pose.slug())
}

fn spawn_world(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	standards: &mut Assets<StandardMaterial>,
) {
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
}

fn tint_overlay_tiles(tiles: &mut Assets<SkillMapTileMaterial>, assets: &SkillMapTileAssets) {
	for handle in [
		assets.land_fire.clone(),
		assets.land_wave.clone(),
		assets.land_rock.clone(),
		assets.land_cosmo.clone(),
		assets.water.clone(),
		assets.fireball.clone(),
		assets.dumbwave.clone(),
		assets.rockadder.clone(),
		assets.cosimo.clone(),
		assets.cursor.clone(),
	] {
		if let Some(mut material) = tiles.get_mut(&handle) {
			material.set_overlay_amount(OVERLAY_TILE_AMOUNT);
		}
	}
}

fn spawn_capture_map(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	tile_assets: &SkillMapTileAssets,
	kind: SkillMapKind,
	member: SkillMapMember,
) -> (Entity, Handle<Image>) {
	let mut projection = OrthographicProjection::default_2d();
	let extent = map_view_extent();
	projection.scaling_mode = ScalingMode::Fixed { width: extent.x, height: extent.y };
	projection.scale = 1.0;
	let authored = authored_map_from_spec(SkillMapSpec::new(kind, 11));
	let backdrop = SkillMapBackdrop { kind: authored.kind };
	let view = spawn_skill_map_view(
		commands,
		images,
		tile_assets,
		authored,
		member,
		-2,
		projection,
		(CaptureMapCamera, backdrop),
	);
	(view.camera, view.image)
}

/// Match live play: expanded overlay clear is translucent, so land veil stays
/// ~0.21 and only opaque arrows / hit line / power marks ride the world window.
fn capture_map_overlay_clear(
	mut cameras: Query<(&mut Camera, &SkillMapBackdrop), With<CaptureMapCamera>>,
) {
	for (mut camera, backdrop) in &mut cameras {
		camera.clear_color =
			ClearColorConfig::Custom(backdrop.kind.overlay_clear(SkillMapOverlay::Expanded));
	}
}

fn despawn_capture_map(
	commands: &mut Commands,
	cameras: &Query<Entity, With<CaptureMapCamera>>,
	names: &Query<(Entity, &Name)>,
) {
	for entity in cameras.iter() {
		commands.entity(entity).despawn();
	}
	for (entity, name) in names.iter() {
		let label = name.as_str();
		if label == "skill-map-tile"
			|| label == "skill-map-feed-lanes"
			|| label.starts_with("skill-map-feed-step-")
		{
			commands.entity(entity).despawn();
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
	tint_overlay_tiles(&mut tiles, &tile_assets);
	spawn_world(&mut commands, &mut meshes, &mut standards);

	let user = commands.spawn_empty().id();
	let session = commands.spawn_empty().id();
	let member = SkillMapMember { user, session };
	let (camera, image) =
		spawn_capture_map(&mut commands, &mut images, &tile_assets, SkillMapKind::Fireball, member);
	let material = overlays.add(SkillMapOverlayMaterial::new(image.clone()));
	let params =
		OverlayParams::from_overlay_lens(SkillMapOverlay::Expanded, SkillMapOverlayLens::default());
	if let Some(mut overlay) = overlays.get_mut(&material) {
		overlay.params = params;
	}

	let root = commands
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
		.id();
	let viewport = commands
		.spawn((
			ViewportNode::new(camera),
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				right: Val::Px(0.0),
				top: Val::Px(0.0),
				bottom: Val::Px(0.0),
				..default()
			},
			Visibility::Hidden,
			CaptureViewport,
		))
		.id();
	let pane = commands
		.spawn((MaterialNode(material.clone()), SkillMapOverlay::Expanded.pane_node()))
		.id();
	commands.entity(root).add_children(&[viewport, pane]);
	commands.insert_resource(OverlayCapture { material, viewport, member });
}

fn rebuild_map(
	mut commands: Commands,
	mut shots: ResMut<CaptureShots>,
	capture: Res<OverlayCapture>,
	mut images: ResMut<Assets<Image>>,
	mut overlays: ResMut<Assets<SkillMapOverlayMaterial>>,
	tile_assets: Res<SkillMapTileAssets>,
	cameras: Query<Entity, With<CaptureMapCamera>>,
	names: Query<(Entity, &Name)>,
	mut viewports: Query<&mut ViewportNode, With<CaptureViewport>>,
) {
	if !shots.rebuild {
		return;
	}
	let Some(kind) = current_kind(&shots) else {
		shots.rebuild = false;
		return;
	};
	despawn_capture_map(&mut commands, &cameras, &names);
	let (camera, image) =
		spawn_capture_map(&mut commands, &mut images, &tile_assets, kind, capture.member);
	if let Ok(mut viewport) = viewports.get_mut(capture.viewport) {
		*viewport = ViewportNode::new(camera);
	}
	if let Some(mut overlay) = overlays.get_mut(&capture.material) {
		overlay.map = image;
	}
	shots.rebuild = false;
	shots.shown_at = None;
	info!(map = kind.label(), "rebuilt authored skill map for overlay capture");
}

fn drive_lens(
	shots: Res<CaptureShots>,
	capture: Res<OverlayCapture>,
	mut overlays: ResMut<Assets<SkillMapOverlayMaterial>>,
) {
	let Some(pose) = current_pose(&shots) else {
		return;
	};
	if let Some(mut material) = overlays.get_mut(&capture.material) {
		material.params = OverlayParams::from_overlay_lens(SkillMapOverlay::Expanded, pose.lens());
	}
}

fn capture_scene(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<CaptureDir>,
	mut shots: ResMut<CaptureShots>,
) {
	if shots.rebuild || current_kind(&shots).is_none() || shots.requested {
		return;
	}
	let Some(kind) = current_kind(&shots) else {
		return;
	};
	let Some(pose) = current_pose(&shots) else {
		return;
	};
	if shots.shown_at.is_none() {
		shots.shown_at = Some(time.elapsed());
	}
	let Some(shown) = shots.shown_at else {
		return;
	};
	if time.elapsed() < shown + pose.warmup(shots.pose == 0) {
		return;
	}
	let path = dir.0.join(format!("{}.png", file_stem(kind, pose)));
	let _ = std::fs::remove_file(&path);
	info!(
		path = %path.display(),
		map = kind.label(),
		pose = pose.slug(),
		width = CAPTURE_WIDTH,
		height = CAPTURE_HEIGHT,
		"capturing overlay lens still"
	);
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
	if current_kind(&shots).is_none() {
		exit.write(AppExit::Success);
		return;
	}
	if shots.rebuild {
		return;
	}
	if !shots.requested {
		if shots.shown_at.is_some_and(|at| time.elapsed() > at + Duration::from_secs(16)) {
			warn!("overlay lens capture timed out waiting to request a still");
			exit.write(AppExit::Success);
		}
		return;
	}
	let Some(kind) = current_kind(&shots) else {
		return;
	};
	let Some(pose) = current_pose(&shots) else {
		return;
	};
	let path = dir.0.join(format!("{}.png", file_stem(kind, pose)));
	if path.is_file() && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if !(ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(200))
		|| shots.shown_at.is_some_and(|at| time.elapsed() > at + Duration::from_secs(12)))
	{
		return;
	}
	shots.requested = false;
	shots.shown_at = None;
	*ready_at = None;
	shots.pose += 1;
	if shots.pose >= CapturePose::ALL.len() {
		shots.pose = 0;
		shots.map += 1;
		if current_kind(&shots).is_some() {
			shots.rebuild = true;
		} else {
			exit.write(AppExit::Success);
		}
	}
}
