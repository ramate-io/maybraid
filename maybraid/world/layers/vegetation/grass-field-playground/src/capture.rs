//! Env-gated playground captures (`MAYBRAID_CAPTURE_DIR`).

use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};

use crate::camera::{yaw_pitch, CameraController};

pub const CAPTURE_DIR_ENV: &str = "MAYBRAID_CAPTURE_DIR";
const WARMUP_FRAMES: u32 = 140;
const SETTLE_FRAMES: u32 = 22;
const SAVE_FRAMES: u32 = 28;

#[derive(Component)]
pub struct KeySun;

#[derive(Component)]
pub struct FillSky;

#[derive(Clone, Copy, Debug)]
pub struct CaptureShot {
	pub name: &'static str,
	pub camera: Vec3,
	pub look_at: Vec3,
}

pub const GRASS_CAPTURE_SHOTS: &[CaptureShot] = &[
	CaptureShot {
		name: "meadow-orbit",
		camera: Vec3::new(10.0, 3.2, 16.0),
		look_at: Vec3::new(8.0, 0.3, 8.0),
	},
	CaptureShot {
		name: "meadow-ground",
		camera: Vec3::new(6.0, 0.55, 7.5),
		look_at: Vec3::new(10.0, 0.35, 12.0),
	},
	CaptureShot {
		name: "meadow-overhead",
		camera: Vec3::new(12.0, 11.0, 14.5),
		look_at: Vec3::new(12.0, 0.0, 12.0),
	},
	CaptureShot {
		name: "slope-rake",
		camera: Vec3::new(18.0, 3.4, 12.0),
		look_at: Vec3::new(26.0, 1.4, 6.0),
	},
	CaptureShot {
		name: "crest-skyline",
		camera: Vec3::new(12.0, 1.2, 32.0),
		look_at: Vec3::new(12.0, 1.8, 22.0),
	},
	CaptureShot {
		name: "clump-close",
		camera: Vec3::new(3.6, 0.42, 4.4),
		look_at: Vec3::new(3.0, 0.28, 3.0),
	},
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
				view.apply_shot(&GRASS_CAPTURE_SHOTS[0]);
			}
			if frames + 1 >= WARMUP_FRAMES {
				session.phase = CapturePhase::Settle { index: 0, frames: 0 };
			} else {
				session.phase = CapturePhase::Warmup { frames: frames + 1 };
			}
		}
		CapturePhase::Settle { index, frames } => {
			if frames == 0 {
				if let Some(shot) = GRASS_CAPTURE_SHOTS.get(index) {
					view.apply_shot(shot);
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
	fn apply_shot(&mut self, shot: &CaptureShot) {
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
		assert_eq!(GRASS_CAPTURE_SHOTS.len(), 6);
		for name in
			["meadow-orbit", "meadow-ground", "meadow-overhead", "slope-rake", "crest-skyline"]
		{
			assert!(GRASS_CAPTURE_SHOTS.iter().any(|shot| shot.name == name), "missing {name}");
		}
		Ok(())
	}
}
