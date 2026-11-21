//! Env-gated playground captures (`MAYBRAID_CAPTURE_DIR`).
//!
//! Still size is 960×540, matching other playground previews.

use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use vegetation_grass_field::GrassFieldSettings;
use vegetation_shaders::wesl::WindInputs;

use crate::camera::{yaw_pitch, CameraController};
use crate::scene;

pub const CAPTURE_DIR_ENV: &str = "MAYBRAID_CAPTURE_DIR";
pub const CAPTURE_WIDTH: u32 = 960;
pub const CAPTURE_HEIGHT: u32 = 540;
const WARMUP_FRAMES: u32 = 160;
const SETTLE_FRAMES: u32 = 36;
const SAVE_FRAMES: u32 = 28;

const GROUND: (Vec3, Vec3) = (Vec3::new(6.0, 0.55, 7.5), Vec3::new(10.0, 0.35, 12.0));
const CLOSE: (Vec3, Vec3) = (Vec3::new(8.4, 0.22, 9.1), Vec3::new(10.2, 0.18, 11.4));
const OVERHEAD: (Vec3, Vec3) = (Vec3::new(12.0, 11.0, 14.5), Vec3::new(12.0, 0.0, 12.0));
const SLOPE: (Vec3, Vec3) = (Vec3::new(18.0, 3.4, 12.0), Vec3::new(26.0, 1.4, 6.0));
const LOD: (Vec3, Vec3) = (Vec3::new(5.4, 1.45, 5.8), Vec3::new(18.0, 0.25, 22.0));
const ORBIT: (Vec3, Vec3) = (Vec3::new(10.0, 3.2, 16.0), Vec3::new(8.0, 0.3, 8.0));
const CREST: (Vec3, Vec3) = (Vec3::new(12.0, 1.2, 32.0), Vec3::new(12.0, 1.8, 22.0));
const CLUMP: (Vec3, Vec3) = (Vec3::new(3.6, 0.42, 4.4), Vec3::new(3.0, 0.28, 3.0));

#[derive(Component)]
pub struct KeySun;

#[derive(Component)]
pub struct FillSky;

#[derive(Clone, Copy, Debug)]
pub struct CaptureShot {
	pub name: &'static str,
	pub camera: Vec3,
	pub look_at: Vec3,
	pub grass: bool,
	pub gust: bool,
}

impl CaptureShot {
	pub const fn grass(name: &'static str, pose: (Vec3, Vec3)) -> Self {
		Self { name, camera: pose.0, look_at: pose.1, grass: true, gust: false }
	}

	pub const fn turf(name: &'static str, pose: (Vec3, Vec3)) -> Self {
		Self { name, camera: pose.0, look_at: pose.1, grass: false, gust: false }
	}

	pub const fn with_gust(mut self) -> Self {
		self.gust = true;
		self
	}

	pub fn apply_world(self, world: &mut World) {
		if self.grass {
			if !scene::has_patches(world) {
				scene::spawn_default_scene_in_world(world);
			}
		} else {
			scene::clear_patches(world);
		}
		let mut settings = world.resource_mut::<GrassFieldSettings>();
		settings.wind =
			if self.gust { WindInputs::botw_gust(1.85) } else { WindInputs::gentle_botw(0.85) };
		settings.freeze_time_s = Some(if self.gust { 1.85 } else { 0.85 });
	}
}

pub const GRASS_CAPTURE_SHOTS: &[CaptureShot] = &[
	CaptureShot::grass("meadow-ground", GROUND),
	CaptureShot::turf("turf-meadow-ground", GROUND),
	CaptureShot::grass("blade-close", CLOSE),
	CaptureShot::grass("meadow-overhead", OVERHEAD),
	CaptureShot::turf("turf-meadow-overhead", OVERHEAD),
	CaptureShot::grass("slope-rake", SLOPE),
	CaptureShot::turf("turf-slope-rake", SLOPE),
	CaptureShot::grass("lod-transition", LOD),
	CaptureShot::turf("turf-lod-transition", LOD),
	CaptureShot::grass("wind-gust", GROUND).with_gust(),
	CaptureShot::grass("meadow-orbit", ORBIT),
	CaptureShot::turf("turf-meadow-orbit", ORBIT),
	CaptureShot::grass("crest-skyline", CREST),
	CaptureShot::grass("clump-close", CLUMP),
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
				view.apply_camera(&GRASS_CAPTURE_SHOTS[0]);
				let shot = GRASS_CAPTURE_SHOTS[0];
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
				if let Some(shot) = GRASS_CAPTURE_SHOTS.get(index).copied() {
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
			let Some(shot) = GRASS_CAPTURE_SHOTS.get(index) else {
				exit.write(AppExit::Success);
				return;
			};
			if let Err(error) = std::fs::create_dir_all(&session.dir) {
				error!("grass capture: cannot create {}: {error}", session.dir.display());
				exit.write(AppExit::from_code(2));
				return;
			}
			let path = session.dir.join(format!("{}.png", shot.name));
			info!("grass capture: {}", path.display());
			commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
			session.phase = CapturePhase::WaitSave { index, frames: 0 };
		}
		CapturePhase::WaitSave { index, frames } => {
			if (capturing.iter().next().is_none() && frames >= 2) || frames + 1 >= SAVE_FRAMES {
				let next = index + 1;
				if next >= GRASS_CAPTURE_SHOTS.len() {
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
			"meadow-ground",
			"turf-meadow-ground",
			"blade-close",
			"meadow-overhead",
			"turf-meadow-overhead",
			"slope-rake",
			"turf-slope-rake",
			"lod-transition",
			"turf-lod-transition",
			"wind-gust",
			"meadow-orbit",
			"turf-meadow-orbit",
			"crest-skyline",
			"clump-close",
		] {
			assert!(GRASS_CAPTURE_SHOTS.iter().any(|shot| shot.name == name), "missing {name}");
		}
		let ground = GRASS_CAPTURE_SHOTS.iter().find(|shot| shot.name == "meadow-ground").unwrap();
		let turf = GRASS_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "turf-meadow-ground")
			.unwrap();
		assert_eq!(ground.camera, turf.camera);
		assert_eq!(ground.look_at, turf.look_at);
		assert!(ground.grass && !turf.grass);
		assert!(GRASS_CAPTURE_SHOTS.iter().any(|shot| shot.gust));
		Ok(())
	}
}
