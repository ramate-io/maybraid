//! Env-gated playground captures (`MAYBRAID_CAPTURE_DIR`).
//!
//! Still size is 960×540.

use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use vegetation_shaders::wesl::WindInputs;
use vegetation_tree_field::TreeFieldSettings;

use crate::camera::{yaw_pitch, CameraController};
use crate::kit;
use crate::scene;

pub const CAPTURE_DIR_ENV: &str = "MAYBRAID_CAPTURE_DIR";
pub const CAPTURE_WIDTH: u32 = 960;
pub const CAPTURE_HEIGHT: u32 = 540;
const WARMUP_FRAMES: u32 = 160;
const SETTLE_FRAMES: u32 = 36;
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
}

#[derive(Clone, Copy, Debug)]
pub struct CaptureShot {
	pub name: &'static str,
	pub camera: Vec3,
	pub look_at: Vec3,
	pub mode: CaptureMode,
	pub scene: CaptureScene,
	pub gust: bool,
}

impl CaptureShot {
	pub const fn gpu(name: &'static str, pose: (Vec3, Vec3), scene: CaptureScene) -> Self {
		Self { name, camera: pose.0, look_at: pose.1, mode: CaptureMode::Gpu, scene, gust: false }
	}

	pub const fn kit(name: &'static str, pose: (Vec3, Vec3)) -> Self {
		Self {
			name,
			camera: pose.0,
			look_at: pose.1,
			mode: CaptureMode::Kit,
			scene: CaptureScene::Close,
			gust: false,
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
		}
	}

	pub const fn with_gust(mut self) -> Self {
		self.gust = true;
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
				CaptureScene::Default => {
					scene::clear_patches(world);
					scene::spawn_default_scene_in_world(world);
				}
			},
			CaptureMode::Kit => {
				scene::spawn_close_only(world);
				kit::spawn_kit_from_patches(world);
				scene::clear_patches(world);
			}
			CaptureMode::Leafless => {
				scene::spawn_leafless_only(world);
			}
		}
		let mut settings = world.resource_mut::<TreeFieldSettings>();
		settings.wind =
			if self.gust { WindInputs::botw_gust(1.85) } else { WindInputs::gentle_botw(0.85) };
		settings.freeze_time_s = Some(if self.gust { 1.85 } else { 0.85 });
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
	CaptureShot::gpu("grove-edge", GROVE, CaptureScene::Grove),
	CaptureShot::gpu("grove", GROVE_FULL, CaptureScene::Grove),
	CaptureShot::gpu("slope-rake", SLOPE, CaptureScene::Slope),
	CaptureShot::gpu("lod-transition", LOD, CaptureScene::Lod),
	CaptureShot::gpu("wind-gust", GUST, CaptureScene::Close).with_gust(),
	CaptureShot::gpu("willow-full", WILLOW_FULL, CaptureScene::Willow),
	CaptureShot::gpu("willow-drape-close", WILLOW_DRAPE, CaptureScene::Willow),
	CaptureShot::gpu("willow-wind", WILLOW_FULL, CaptureScene::Willow).with_gust(),
];

#[derive(Resource, Debug)]
pub struct CaptureSession {
	pub dir: PathBuf,
	phase: CapturePhase,
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
		Some(Self { dir: PathBuf::from(dir), phase: CapturePhase::Warmup { frames: 0 } })
	}

	pub fn window_resolution() -> (u32, u32) {
		(CAPTURE_WIDTH, CAPTURE_HEIGHT)
	}
}

#[derive(SystemParam)]
pub(crate) struct CaptureView<'w, 's> {
	cameras: Query<'w, 's, (&'static mut Transform, &'static mut CameraController), With<Camera3d>>,
}

pub(crate) fn tick_capture(
	mut commands: Commands,
	mut session: ResMut<CaptureSession>,
	mut view: CaptureView,
	capturing: Query<Entity, With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
) {
	match session.phase {
		CapturePhase::Warmup { frames } => {
			if frames == 0 {
				view.apply_camera(&TREE_CAPTURE_SHOTS[0]);
				let shot = TREE_CAPTURE_SHOTS[0];
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
				if let Some(shot) = TREE_CAPTURE_SHOTS.get(index).copied() {
					view.apply_camera(&shot);
					commands.queue(move |world: &mut World| shot.apply_world(world));
				}
			}
			if frames + 1 >= SETTLE_FRAMES {
				session.phase = CapturePhase::Snap { index };
			} else {
				session.phase = CapturePhase::Settle { index, frames: frames + 1 };
			}
		}
		CapturePhase::Snap { index } => {
			let Some(shot) = TREE_CAPTURE_SHOTS.get(index) else {
				exit.write(AppExit::Success);
				return;
			};
			if let Err(error) = std::fs::create_dir_all(&session.dir) {
				error!("tree capture: cannot create {}: {error}", session.dir.display());
				exit.write(AppExit::from_code(2));
				return;
			}
			let path = session.dir.join(format!("{}.png", shot.name));
			info!("tree capture: {}", path.display());
			commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
			session.phase = CapturePhase::WaitSave { index, frames: 0 };
		}
		CapturePhase::WaitSave { index, frames } => {
			if (capturing.iter().next().is_none() && frames >= 2) || frames + 1 >= SAVE_FRAMES {
				let next = index + 1;
				if next >= TREE_CAPTURE_SHOTS.len() {
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

impl CaptureView<'_, '_> {
	fn apply_camera(&mut self, shot: &CaptureShot) {
		if let Ok((mut transform, mut controller)) = self.cameras.single_mut() {
			*transform = Transform::from_translation(shot.camera).looking_at(shot.look_at, Vec3::Y);
			let (yaw, pitch) = yaw_pitch(transform.rotation);
			controller.yaw = yaw;
			controller.pitch = pitch;
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
			"grove-edge",
			"grove",
			"slope-rake",
			"lod-transition",
			"wind-gust",
			"kit-orbit",
			"kit-close",
			"willow-full",
			"willow-drape-close",
			"willow-wind",
		] {
			assert!(TREE_CAPTURE_SHOTS.iter().any(|shot| shot.name == name), "missing {name}");
		}
		let gpu = TREE_CAPTURE_SHOTS.iter().find(|shot| shot.name == "canopy-close");
		let kit = TREE_CAPTURE_SHOTS.iter().find(|shot| shot.name == "kit-close");
		let gpu = gpu.ok_or_else(|| anyhow::anyhow!("missing gpu close"))?;
		let kit = kit.ok_or_else(|| anyhow::anyhow!("missing kit close"))?;
		assert_eq!(gpu.camera, kit.camera);
		assert_eq!(gpu.look_at, kit.look_at);
		assert!(TREE_CAPTURE_SHOTS.iter().any(|shot| shot.gust));
		Ok(())
	}
}
