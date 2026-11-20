//! Env-gated playground captures (`MAYBRAID_CAPTURE_DIR`).
//!
//! Still size is 960×540.

use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use vegetation_bush_field::BushFieldSettings;
use vegetation_shaders::wesl::WindInputs;

use crate::camera::{yaw_pitch, CameraController};
use crate::kit;
use crate::scene;

pub const CAPTURE_DIR_ENV: &str = "MAYBRAID_CAPTURE_DIR";
pub const CAPTURE_WIDTH: u32 = 960;
pub const CAPTURE_HEIGHT: u32 = 540;
const WARMUP_FRAMES: u32 = 160;
const SETTLE_FRAMES: u32 = 36;
const SAVE_FRAMES: u32 = 28;

const ORBIT: (Vec3, Vec3) = (Vec3::new(4.35, 1.25, 5.4), Vec3::new(3.0, 0.38, 3.0));
const OVERHEAD: (Vec3, Vec3) = (Vec3::new(5.0, 6.8, 5.2), Vec3::new(5.0, 0.0, 5.0));
const UNDERSIDE: (Vec3, Vec3) = (Vec3::new(3.08, 0.05, 3.42), Vec3::new(3.0, 0.36, 3.0));
const CLOSE: (Vec3, Vec3) = (Vec3::new(3.72, 0.58, 4.05), Vec3::new(3.0, 0.36, 3.0));
const LEAFLESS: (Vec3, Vec3) = (Vec3::new(7.55, 0.72, 3.55), Vec3::new(6.5, 0.32, 2.2));
const DENSE: (Vec3, Vec3) = (Vec3::new(2.6, 1.05, 7.4), Vec3::new(5.0, 0.32, 5.0));
const SLOPE: (Vec3, Vec3) = (Vec3::new(12.5, 2.6, 7.0), Vec3::new(18.0, 1.1, 4.0));
const LOD: (Vec3, Vec3) = (Vec3::new(2.0, 1.05, 2.6), Vec3::new(10.0, 0.28, 10.0));
const GUST: (Vec3, Vec3) = (Vec3::new(4.2, 1.05, 5.6), Vec3::new(3.2, 0.34, 3.2));

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
	Default,
	Close,
	Dense,
	Slope,
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
				CaptureScene::Dense => {
					scene::clear_patches(world);
					scene::spawn_kind(world, scene::PatchKind::Dense);
				}
				CaptureScene::Slope => {
					scene::clear_patches(world);
					scene::spawn_kind(world, scene::PatchKind::Slope);
				}
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
		let mut settings = world.resource_mut::<BushFieldSettings>();
		settings.wind =
			if self.gust { WindInputs::botw_gust(1.85) } else { WindInputs::gentle_botw(0.85) };
		settings.freeze_time_s = Some(if self.gust { 1.85 } else { 0.85 });
	}
}

pub const BUSH_CAPTURE_SHOTS: &[CaptureShot] = &[
	CaptureShot::gpu("bush-orbit", ORBIT, CaptureScene::Close),
	CaptureShot::kit("kit-orbit", ORBIT),
	CaptureShot::gpu("bush-overhead", OVERHEAD, CaptureScene::Dense),
	CaptureShot::gpu("bush-underside", UNDERSIDE, CaptureScene::Close),
	CaptureShot::leafless("wood-leafless", LEAFLESS),
	CaptureShot::gpu("bush-close", CLOSE, CaptureScene::Close),
	CaptureShot::kit("kit-close", CLOSE),
	CaptureShot::gpu("dense-patch", DENSE, CaptureScene::Dense),
	CaptureShot::gpu("slope-rake", SLOPE, CaptureScene::Slope),
	CaptureShot::gpu("lod-transition", LOD, CaptureScene::Dense),
	CaptureShot::gpu("wind-gust", GUST, CaptureScene::Close).with_gust(),
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
				view.apply_camera(&BUSH_CAPTURE_SHOTS[0]);
				let shot = BUSH_CAPTURE_SHOTS[0];
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
				if let Some(shot) = BUSH_CAPTURE_SHOTS.get(index).copied() {
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
			let Some(shot) = BUSH_CAPTURE_SHOTS.get(index) else {
				exit.write(AppExit::Success);
				return;
			};
			if let Err(error) = std::fs::create_dir_all(&session.dir) {
				error!("bush capture: cannot create {}: {error}", session.dir.display());
				exit.write(AppExit::from_code(2));
				return;
			}
			let path = session.dir.join(format!("{}.png", shot.name));
			info!("bush capture: {}", path.display());
			commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
			session.phase = CapturePhase::WaitSave { index, frames: 0 };
		}
		CapturePhase::WaitSave { index, frames } => {
			if (capturing.iter().next().is_none() && frames >= 2) || frames + 1 >= SAVE_FRAMES {
				let next = index + 1;
				if next >= BUSH_CAPTURE_SHOTS.len() {
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
			"bush-orbit",
			"bush-overhead",
			"bush-underside",
			"wood-leafless",
			"bush-close",
			"dense-patch",
			"slope-rake",
			"lod-transition",
			"wind-gust",
			"kit-orbit",
			"kit-close",
		] {
			assert!(BUSH_CAPTURE_SHOTS.iter().any(|shot| shot.name == name), "missing {name}");
		}
		let gpu = BUSH_CAPTURE_SHOTS.iter().find(|shot| shot.name == "bush-close");
		let kit = BUSH_CAPTURE_SHOTS.iter().find(|shot| shot.name == "kit-close");
		let gpu = gpu.ok_or_else(|| anyhow::anyhow!("missing gpu close"))?;
		let kit = kit.ok_or_else(|| anyhow::anyhow!("missing kit close"))?;
		assert_eq!(gpu.camera, kit.camera);
		assert_eq!(gpu.look_at, kit.look_at);
		assert!(BUSH_CAPTURE_SHOTS.iter().any(|shot| shot.gust));
		Ok(())
	}
}
