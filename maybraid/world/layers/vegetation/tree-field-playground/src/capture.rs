//! Env-gated playground captures (`MAYBRAID_CAPTURE_DIR`).
//!
//! Still size is 960×540.

use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Capturing, Screenshot, save_to_disk};
use vegetation_bush_field::BushField;
use vegetation_grass_field::GrassField;
use vegetation_shaders::wesl::WindInputs;
use vegetation_tree_field::{LeafCloseStyle, TreeField, TreeFieldSettings, TreeGrove};

use crate::camera::{CameraController, yaw_pitch};
use crate::kit;
use crate::scene;
use crate::world;

pub const CAPTURE_DIR_ENV: &str = "MAYBRAID_CAPTURE_DIR";
pub const CAPTURE_WIDTH: u32 = 960;
pub const CAPTURE_HEIGHT: u32 = 540;
const WARMUP_FRAMES: u32 = 160;
const SETTLE_FRAMES: u32 = 48;
const SETTLE_SKIP_FRAMES: u32 = 12;
const SAVE_FRAMES: u32 = 28;

const ORBIT: (Vec3, Vec3) = (Vec3::new(16.5, 8.2, 18.0), Vec3::new(0.0, 7.0, 0.0));
const OVERHEAD: (Vec3, Vec3) = (Vec3::new(1.4, 28.0, 1.1), Vec3::new(0.0, 0.0, 0.0));
const UNDERSIDE: (Vec3, Vec3) = (Vec3::new(3.2, 1.55, 4.6), Vec3::new(0.2, 10.4, 0.0));
const CLOSE: (Vec3, Vec3) = (Vec3::new(8.2, 9.4, 7.4), Vec3::new(0.4, 8.6, 0.2));
const LEAF_CLOSE: (Vec3, Vec3) = (Vec3::new(4.9, 8.85, 4.5), Vec3::new(3.3, 8.45, 2.9));
const LEAFLESS: (Vec3, Vec3) = (Vec3::new(22.5, 9.0, -1.5), Vec3::new(16.0, 7.2, -8.0));
const GROVE: (Vec3, Vec3) = (Vec3::new(2.0, 9.5, 48.0), Vec3::new(22.0, 8.0, 22.0));
const GROVE_FULL: (Vec3, Vec3) = (Vec3::new(20.0, 11.0, 46.0), Vec3::new(22.0, 8.0, 22.0));
const SLOPE: (Vec3, Vec3) = (Vec3::new(8.0, 8.5, 22.0), Vec3::new(32.0, 7.0, 6.0));
const LOD: (Vec3, Vec3) = (Vec3::new(-10.0, 14.0, 40.0), Vec3::new(44.0, 10.0, 10.0));
const GUST: (Vec3, Vec3) = (Vec3::new(15.8, 8.0, 17.2), Vec3::new(0.0, 7.0, 0.0));
const WILLOW_FULL: (Vec3, Vec3) = (Vec3::new(17.2, 6.0, 15.8), Vec3::new(0.0, 5.4, 0.0));
const WILLOW_DRAPE: (Vec3, Vec3) = (Vec3::new(4.7, 5.15, 4.1), Vec3::new(2.1, 4.55, 1.7));

#[derive(Component)]
pub struct KeySun;

#[derive(Component)]
pub struct FillSky;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureMode {
	Gpu,
	Kit,
	Leafless,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureScene {
	#[allow(dead_code)]
	Default,
	Close,
	Grove,
	Slope,
	Lod,
	Willow,
	WorldForest,
	WorldWalk,
	WorldFar,
}

#[derive(Clone, Copy, Debug)]
pub struct CaptureShot {
	pub name: &'static str,
	pub camera: Vec3,
	pub look_at: Vec3,
	pub mode: CaptureMode,
	pub scene: CaptureScene,
	pub gust: bool,
	pub leaf_style: Option<LeafCloseStyle>,
}

impl CaptureShot {
	pub const fn gpu(name: &'static str, pose: (Vec3, Vec3), scene: CaptureScene) -> Self {
		Self {
			name,
			camera: pose.0,
			look_at: pose.1,
			mode: CaptureMode::Gpu,
			scene,
			gust: false,
			leaf_style: None,
		}
	}

	pub const fn kit(name: &'static str, pose: (Vec3, Vec3)) -> Self {
		Self {
			name,
			camera: pose.0,
			look_at: pose.1,
			mode: CaptureMode::Kit,
			scene: CaptureScene::Close,
			gust: false,
			leaf_style: None,
		}
	}

	pub const fn leafless(name: &'static str, pose: (Vec3, Vec3)) -> Self {
		Self {
			name,
			camera: pose.0,
			look_at: pose.1,
			mode: CaptureMode::Leafless,
			scene: CaptureScene::Close,
			gust: false,
			leaf_style: None,
		}
	}

	pub const fn with_gust(mut self) -> Self {
		self.gust = true;
		self
	}

	pub const fn with_leaves(mut self, style: LeafCloseStyle) -> Self {
		self.leaf_style = Some(style);
		self
	}

	pub const fn with_scene(mut self, scene: CaptureScene) -> Self {
		self.scene = scene;
		self
	}

	pub fn apply_world(self, world: &mut World) {
		kit::clear_kit(world);
		match self.mode {
			CaptureMode::Gpu => match self.scene {
				CaptureScene::Close => scene::spawn_close_only(world),
				CaptureScene::Grove => {
					scene::clear_patches(world);
					scene::spawn_kind(world, scene::PatchKind::Grove);
				}
				CaptureScene::Slope => {
					scene::clear_patches(world);
					scene::spawn_kind(world, scene::PatchKind::Slope);
				}
				CaptureScene::Lod => {
					scene::clear_patches(world);
					scene::spawn_kind(world, scene::PatchKind::Lod);
				}
				CaptureScene::Willow => scene::spawn_willow_only(world),
				CaptureScene::WorldForest => {
					scene::clear_patches(world);
					world::spawn_world_forest(world, true);
				}
				CaptureScene::WorldWalk => {
					scene::clear_patches(world);
					world::spawn_world_walk(world, true);
				}
				CaptureScene::WorldFar => {
					scene::clear_patches(world);
					world::spawn_world_far(world, true);
				}
				CaptureScene::Default => {
					scene::clear_patches(world);
					scene::spawn_default_scene_in_world(world);
				}
			},
			CaptureMode::Kit => match self.scene {
				CaptureScene::Grove => {
					scene::clear_patches(world);
					scene::spawn_kind(world, scene::PatchKind::Grove);
					kit::spawn_kit_from_patches(world);
					scene::clear_patches(world);
				}
				CaptureScene::WorldForest => {
					scene::clear_patches(world);
					world::spawn_world_forest(world, false);
				}
				CaptureScene::WorldWalk => {
					scene::clear_patches(world);
					world::spawn_world_walk(world, false);
				}
				CaptureScene::WorldFar => {
					scene::clear_patches(world);
					world::spawn_world_far(world, false);
				}
				_ => {
					scene::spawn_close_only(world);
					kit::spawn_kit_from_patches(world);
					scene::clear_patches(world);
				}
			},
			CaptureMode::Leafless => {
				scene::spawn_leafless_only(world);
			}
		}
		let mut settings = world.resource_mut::<TreeFieldSettings>();
		settings.wind =
			if self.gust { WindInputs::botw_gust(1.85) } else { WindInputs::gentle_botw(0.85) };
		settings.freeze_time_s = Some(if self.gust { 1.85 } else { 0.85 });
		settings.leaf_close_style = self.leaf_style;
		settings.diagnostics = true;
	}
}

pub const TREE_CAPTURE_SHOTS: &[CaptureShot] = &[
	CaptureShot::gpu("canopy-orbit", ORBIT, CaptureScene::Close),
	CaptureShot::kit("kit-orbit", ORBIT),
	CaptureShot::gpu("canopy-overhead", OVERHEAD, CaptureScene::Close),
	CaptureShot::gpu("canopy-underside", UNDERSIDE, CaptureScene::Close),
	CaptureShot::leafless("wood-leafless", LEAFLESS),
	CaptureShot::gpu("canopy-close", CLOSE, CaptureScene::Close),
	CaptureShot::kit("kit-close", CLOSE),
	CaptureShot::gpu("canopy-leaf-close", LEAF_CLOSE, CaptureScene::Close),
	CaptureShot::gpu("canopy-leaf-close-dither", LEAF_CLOSE, CaptureScene::Close)
		.with_leaves(LeafCloseStyle::Dither),
	CaptureShot::gpu("canopy-leaf-close-smooth", LEAF_CLOSE, CaptureScene::Close)
		.with_leaves(LeafCloseStyle::Smooth),
	CaptureShot::gpu("canopy-leaf-close-crisp", LEAF_CLOSE, CaptureScene::Close)
		.with_leaves(LeafCloseStyle::Crisp),
	CaptureShot::gpu("grove-edge", GROVE, CaptureScene::Grove),
	CaptureShot::kit("grove-edge-legacy", GROVE).with_scene(CaptureScene::Grove),
	CaptureShot::gpu("grove-edge-gpu", GROVE, CaptureScene::Grove),
	CaptureShot::gpu("grove", GROVE_FULL, CaptureScene::Grove),
	CaptureShot::kit("grove-legacy", GROVE_FULL).with_scene(CaptureScene::Grove),
	CaptureShot::gpu("grove-gpu", GROVE_FULL, CaptureScene::Grove),
	CaptureShot::gpu("slope-rake", SLOPE, CaptureScene::Slope),
	CaptureShot::gpu("lod-transition", LOD, CaptureScene::Lod),
	CaptureShot::gpu("wind-gust", GUST, CaptureScene::Close).with_gust(),
	CaptureShot::gpu("willow-full", WILLOW_FULL, CaptureScene::Willow),
	CaptureShot::gpu("willow-drape-close", WILLOW_DRAPE, CaptureScene::Willow),
	CaptureShot::gpu("willow-drape-close-dither", WILLOW_DRAPE, CaptureScene::Willow)
		.with_leaves(LeafCloseStyle::Dither),
	CaptureShot::gpu("willow-drape-close-crisp", WILLOW_DRAPE, CaptureScene::Willow)
		.with_leaves(LeafCloseStyle::Crisp),
	CaptureShot::gpu("willow-wind", WILLOW_FULL, CaptureScene::Willow).with_gust(),
	CaptureShot::kit("forest-vista-legacy", world::VISTA).with_scene(CaptureScene::WorldForest),
	CaptureShot::gpu("forest-vista-gpu", world::VISTA, CaptureScene::WorldForest),
	CaptureShot::kit("forest-edge-legacy", world::EDGE).with_scene(CaptureScene::WorldForest),
	CaptureShot::gpu("forest-edge-gpu", world::EDGE, CaptureScene::WorldForest),
	CaptureShot::kit("forest-floor-legacy", world::FLOOR).with_scene(CaptureScene::WorldForest),
	CaptureShot::gpu("forest-floor-gpu", world::FLOOR, CaptureScene::WorldForest),
	CaptureShot::kit("forest-far-legacy", world::FAR).with_scene(CaptureScene::WorldFar),
	CaptureShot::gpu("forest-far-gpu", world::FAR, CaptureScene::WorldFar),
	CaptureShot::kit("walk-start-legacy", world::WALK_START).with_scene(CaptureScene::WorldWalk),
	CaptureShot::gpu("walk-start-gpu", world::WALK_START, CaptureScene::WorldWalk),
	CaptureShot::kit("walk-mid-legacy", world::WALK_MID).with_scene(CaptureScene::WorldWalk),
	CaptureShot::gpu("walk-mid-gpu", world::WALK_MID, CaptureScene::WorldWalk),
	CaptureShot::kit("walk-end-legacy", world::WALK_END).with_scene(CaptureScene::WorldWalk),
	CaptureShot::gpu("walk-end-gpu", world::WALK_END, CaptureScene::WorldWalk),
];

#[derive(Resource, Debug)]
pub struct CaptureSession {
	pub dir: PathBuf,
	shots: Vec<CaptureShot>,
	phase: CapturePhase,
	dt_ms: Vec<f32>,
}

impl CaptureSession {
	fn record_frame(&mut self, dt_secs: f32) {
		self.dt_ms.push(dt_secs * 1000.0);
	}

	fn reset_frames(&mut self) {
		self.dt_ms.clear();
	}

	fn frame_summary(&self) -> (u32, f32, f32, f32) {
		if self.dt_ms.is_empty() {
			return (0, 0.0, 0.0, 0.0);
		}
		let mut sorted = self.dt_ms.clone();
		sorted.sort_by(|a, b| a.total_cmp(b));
		let n = sorted.len();
		let mean = sorted.iter().sum::<f32>() / n as f32;
		let p95 = sorted[((n as f32 - 1.0) * 0.95).round() as usize];
		(n as u32, mean, p95, sorted[n - 1])
	}
}

#[derive(Clone, Copy, Debug)]
enum CapturePhase {
	Warmup { frames: u32 },
	Settle { index: usize, frames: u32 },
	Snap { index: usize },
	WaitSave { index: usize, frames: u32 },
}

impl CaptureSession {
	pub fn from_env() -> Option<Self> {
		let dir = std::env::var_os(CAPTURE_DIR_ENV)?;
		if dir.is_empty() {
			return None;
		}
		let shots = selected_shots();
		if shots.is_empty() {
			return None;
		}
		Some(Self {
			dir: PathBuf::from(dir),
			shots,
			phase: CapturePhase::Warmup { frames: 0 },
			dt_ms: Vec::new(),
		})
	}

	pub fn window_resolution() -> (u32, u32) {
		(CAPTURE_WIDTH, CAPTURE_HEIGHT)
	}
}

#[derive(SystemParam)]
pub(crate) struct CaptureView<'w, 's> {
	cameras: Query<
		'w,
		's,
		(&'static mut Transform, &'static mut CameraController, &'static mut Projection),
		With<Camera3d>,
	>,
}

pub(crate) fn tick_capture(
	mut commands: Commands,
	mut session: ResMut<CaptureSession>,
	mut view: CaptureView,
	capturing: Query<Entity, With<Capturing>>,
	entities: Query<Entity>,
	kits: Query<Entity, With<kit::KitPreview>>,
	meshes: Query<Entity, With<Mesh3d>>,
	field: Res<TreeField>,
	bush_field: Res<BushField>,
	grass_field: Res<GrassField>,
	groves: Query<&'static TreeGrove>,
	time: Res<Time>,
	mut exit: MessageWriter<AppExit>,
) {
	match session.phase {
		CapturePhase::Warmup { frames } => {
			if frames == 0 {
				view.apply_camera(&session.shots[0]);
				let shot = session.shots[0];
				commands.queue(move |world: &mut World| shot.apply_world(world));
			}
			if frames + 1 >= WARMUP_FRAMES {
				session.phase = CapturePhase::Settle { index: 0, frames: 0 };
			} else {
				session.phase = CapturePhase::Warmup { frames: frames + 1 };
			}
		}
		CapturePhase::Settle { index, frames } => {
			if frames == 0 {
				session.reset_frames();
				if let Some(shot) = session.shots.get(index).copied() {
					view.apply_camera(&shot);
					commands.queue(move |world: &mut World| shot.apply_world(world));
				}
			} else if frames >= SETTLE_SKIP_FRAMES {
				session.record_frame(time.delta_secs());
			}
			if frames + 1 >= SETTLE_FRAMES {
				session.phase = CapturePhase::Snap { index };
			} else {
				session.phase = CapturePhase::Settle { index, frames: frames + 1 };
			}
		}
		CapturePhase::Snap { index } => {
			let Some(shot) = session.shots.get(index).copied() else {
				exit.write(AppExit::Success);
				return;
			};
			if let Err(error) = std::fs::create_dir_all(&session.dir) {
				error!("tree capture: cannot create {}: {error}", session.dir.display());
				exit.write(AppExit::from_code(2));
				return;
			}
			let (samples, mean, p95, max) = session.frame_summary();
			let entities = entities.iter().count();
			let kits = kits.iter().count();
			let meshes = meshes.iter().count();
			let mut plants = field.plants.len();
			let mut branches = field.branches.len();
			let mut leaves = field.leaves.len();
			let mut upload = field.upload_bytes();
			for grove in &groves {
				plants += grove.0.plants.len();
				branches += grove.0.branches.len();
				leaves += grove.0.leaves.len();
				upload += grove.0.upload_bytes();
			}
			let stats = format!(
				"shot={} mode={:?} samples={} dt_ms_mean={mean:.3} dt_ms_p95={p95:.3} dt_ms_max={max:.3} entities={entities} kit_meshes={kits} mesh3d={meshes} plants={plants} branches={branches} leaves={leaves} bushes={} grass={} upload_B={upload}\n",
				shot.name,
				shot.mode,
				samples,
				bush_field.plants.len(),
				grass_field.blades.len(),
			);
			info!(target: "maybraid.tree_field", stats = stats.trim(), "tree capture stats");
			if let Err(error) =
				std::fs::write(session.dir.join(format!("{}.stats.txt", shot.name)), &stats)
			{
				error!("tree capture: cannot write stats: {error}");
			}
			let path = session.dir.join(format!("{}.png", shot.name));
			info!("tree capture: {}", path.display());
			commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
			session.phase = CapturePhase::WaitSave { index, frames: 0 };
		}
		CapturePhase::WaitSave { index, frames } => {
			if (capturing.iter().next().is_none() && frames >= 2) || frames + 1 >= SAVE_FRAMES {
				let next = index + 1;
				if next >= session.shots.len() {
					exit.write(AppExit::Success);
				} else {
					session.phase = CapturePhase::Settle { index: next, frames: 0 };
				}
			} else {
				session.phase = CapturePhase::WaitSave { index, frames: frames + 1 };
			}
		}
	}
}

fn selected_shots() -> Vec<CaptureShot> {
	let all = TREE_CAPTURE_SHOTS;
	let Ok(filter) = std::env::var("MAYBRAID_CAPTURE_FILTER") else {
		return all.to_vec();
	};
	let names: Vec<_> = filter.split(',').map(str::trim).filter(|name| !name.is_empty()).collect();
	if names.is_empty() {
		return all.to_vec();
	}
	all.iter()
		.copied()
		.filter(|shot| names.iter().any(|name| shot.name == *name))
		.collect()
}

impl CaptureView<'_, '_> {
	fn apply_camera(&mut self, shot: &CaptureShot) {
		if let Ok((mut transform, mut controller, mut projection)) = self.cameras.single_mut() {
			*transform = Transform::from_translation(shot.camera).looking_at(shot.look_at, Vec3::Y);
			let (yaw, pitch) = yaw_pitch(transform.rotation);
			controller.yaw = yaw;
			controller.pitch = pitch;
			if let Projection::Perspective(perspective) = projection.as_mut() {
				perspective.far =
					if shot.scene == CaptureScene::WorldFar { world::FAR_CLIP_M } else { 600.0 };
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn capture_set_covers_required_views() -> anyhow::Result<()> {
		assert_eq!(CAPTURE_WIDTH, 960);
		assert_eq!(CAPTURE_HEIGHT, 540);
		for name in [
			"canopy-orbit",
			"canopy-overhead",
			"canopy-underside",
			"wood-leafless",
			"canopy-close",
			"canopy-leaf-close",
			"canopy-leaf-close-dither",
			"canopy-leaf-close-smooth",
			"canopy-leaf-close-crisp",
			"grove-edge",
			"grove-edge-legacy",
			"grove-edge-gpu",
			"grove",
			"grove-legacy",
			"grove-gpu",
			"slope-rake",
			"lod-transition",
			"wind-gust",
			"kit-orbit",
			"kit-close",
			"willow-full",
			"willow-drape-close",
			"willow-drape-close-dither",
			"willow-drape-close-crisp",
			"willow-wind",
			"forest-vista-legacy",
			"forest-vista-gpu",
			"forest-edge-legacy",
			"forest-edge-gpu",
			"forest-floor-legacy",
			"forest-floor-gpu",
			"forest-far-legacy",
			"forest-far-gpu",
		] {
			assert!(TREE_CAPTURE_SHOTS.iter().any(|shot| shot.name == name), "missing {name}");
		}
		let gpu = TREE_CAPTURE_SHOTS.iter().find(|shot| shot.name == "canopy-close");
		let kit = TREE_CAPTURE_SHOTS.iter().find(|shot| shot.name == "kit-close");
		let gpu = gpu.ok_or_else(|| anyhow::anyhow!("missing gpu close"))?;
		let kit = kit.ok_or_else(|| anyhow::anyhow!("missing kit close"))?;
		assert_eq!(gpu.camera, kit.camera);
		assert_eq!(gpu.look_at, kit.look_at);
		let grove_gpu = TREE_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "grove-gpu")
			.ok_or_else(|| anyhow::anyhow!("missing grove gpu"))?;
		let grove_legacy = TREE_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "grove-legacy")
			.ok_or_else(|| anyhow::anyhow!("missing grove legacy"))?;
		assert_eq!(grove_gpu.camera, grove_legacy.camera);
		assert_eq!(grove_gpu.look_at, grove_legacy.look_at);
		let grove_edge_gpu = TREE_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "grove-edge-gpu")
			.ok_or_else(|| anyhow::anyhow!("missing grove edge gpu"))?;
		let grove_edge_legacy = TREE_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "grove-edge-legacy")
			.ok_or_else(|| anyhow::anyhow!("missing grove edge legacy"))?;
		assert_eq!(grove_edge_gpu.camera, grove_edge_legacy.camera);
		assert_eq!(grove_edge_gpu.look_at, grove_edge_legacy.look_at);
		for (gpu_name, legacy_name) in [
			("forest-vista-gpu", "forest-vista-legacy"),
			("forest-edge-gpu", "forest-edge-legacy"),
			("forest-floor-gpu", "forest-floor-legacy"),
			("forest-far-gpu", "forest-far-legacy"),
			("walk-start-gpu", "walk-start-legacy"),
			("walk-mid-gpu", "walk-mid-legacy"),
			("walk-end-gpu", "walk-end-legacy"),
		] {
			let gpu = TREE_CAPTURE_SHOTS
				.iter()
				.find(|shot| shot.name == gpu_name)
				.ok_or_else(|| anyhow::anyhow!("missing {gpu_name}"))?;
			let legacy = TREE_CAPTURE_SHOTS
				.iter()
				.find(|shot| shot.name == legacy_name)
				.ok_or_else(|| anyhow::anyhow!("missing {legacy_name}"))?;
			assert_eq!(gpu.camera, legacy.camera, "{gpu_name}");
			assert_eq!(gpu.look_at, legacy.look_at, "{gpu_name}");
			let scene = if gpu_name.starts_with("walk-") {
				CaptureScene::WorldWalk
			} else if gpu_name.contains("-far-") {
				CaptureScene::WorldFar
			} else {
				CaptureScene::WorldForest
			};
			assert_eq!(gpu.scene, scene, "{gpu_name}");
			assert_eq!(legacy.scene, scene, "{legacy_name}");
		}
		let drape = TREE_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "willow-drape-close")
			.ok_or_else(|| anyhow::anyhow!("missing willow drape close"))?;
		for name in ["willow-drape-close-dither", "willow-drape-close-crisp"] {
			let shot = TREE_CAPTURE_SHOTS
				.iter()
				.find(|shot| shot.name == name)
				.ok_or_else(|| anyhow::anyhow!("missing {name}"))?;
			assert_eq!(shot.camera, drape.camera);
			assert_eq!(shot.look_at, drape.look_at);
		}
		assert!(TREE_CAPTURE_SHOTS.iter().any(|shot| shot.gust));
		Ok(())
	}

	#[test]
	fn frame_summary_reports_mean_p95_and_max() {
		let mut session = CaptureSession {
			dir: PathBuf::from("."),
			shots: Vec::new(),
			phase: CapturePhase::Warmup { frames: 0 },
			dt_ms: Vec::new(),
		};
		assert_eq!(session.frame_summary(), (0, 0.0, 0.0, 0.0));
		for dt in [0.010, 0.012, 0.011, 0.020] {
			session.record_frame(dt);
		}
		let (n, mean, p95, max) = session.frame_summary();
		assert_eq!(n, 4);
		assert!((mean - 13.25).abs() < 1e-3);
		assert!((p95 - 20.0).abs() < 1e-3);
		assert!((max - 20.0).abs() < 1e-3);
		session.reset_frames();
		assert_eq!(session.frame_summary(), (0, 0.0, 0.0, 0.0));
	}
}
