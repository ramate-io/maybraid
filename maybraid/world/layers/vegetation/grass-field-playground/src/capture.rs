//! Env-gated playground captures (`MAYBRAID_CAPTURE_DIR`).
//!
//! Still size is 960×540, matching other playground previews.

use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use vegetation_grass_field::{
	GrassFieldSettings, GrassPartingDebug, GrassPartingMode, GrassPartingSettings,
};
use vegetation_shaders::wesl::WindInputs;

use crate::camera::{yaw_pitch, CameraController};
use crate::scene;

pub const CAPTURE_DIR_ENV: &str = "MAYBRAID_CAPTURE_DIR";
pub const CAPTURE_SET_ENV: &str = "MAYBRAID_CAPTURE_SET";
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureScene {
	Meadow,
	Parting,
}

#[derive(Clone, Copy, Debug)]
pub struct CaptureShot {
	pub name: &'static str,
	pub camera: Vec3,
	pub look_at: Vec3,
	pub grass: bool,
	pub gust: bool,
	pub scene: CaptureScene,
	pub parting_mode: GrassPartingMode,
	pub capsule_distance_m: f32,
	pub rock_distance_m: f32,
	pub trail: bool,
	/// `0` keeps the settings default. Raster scale stills set 1/2/3.
	pub stamp_scale: f32,
	pub mask_overlay: bool,
	pub hide_occluders: bool,
}

impl CaptureShot {
	pub const fn grass(name: &'static str, pose: (Vec3, Vec3)) -> Self {
		Self {
			name,
			camera: pose.0,
			look_at: pose.1,
			grass: true,
			gust: false,
			scene: CaptureScene::Meadow,
			parting_mode: GrassPartingMode::Off,
			capsule_distance_m: 1.35,
			rock_distance_m: 0.95,
			trail: false,
			stamp_scale: 0.0,
			mask_overlay: false,
			hide_occluders: false,
		}
	}

	pub const fn turf(name: &'static str, pose: (Vec3, Vec3)) -> Self {
		Self {
			name,
			camera: pose.0,
			look_at: pose.1,
			grass: false,
			gust: false,
			scene: CaptureScene::Meadow,
			parting_mode: GrassPartingMode::Off,
			capsule_distance_m: 1.35,
			rock_distance_m: 0.95,
			trail: false,
			stamp_scale: 0.0,
			mask_overlay: false,
			hide_occluders: false,
		}
	}

	pub const fn parting(
		name: &'static str,
		pose: (Vec3, Vec3),
		mode: GrassPartingMode,
		capsule_distance_m: f32,
	) -> Self {
		Self {
			name,
			camera: pose.0,
			look_at: pose.1,
			grass: true,
			gust: false,
			scene: CaptureScene::Parting,
			parting_mode: mode,
			capsule_distance_m,
			rock_distance_m: 0.95,
			trail: false,
			stamp_scale: 0.0,
			mask_overlay: false,
			hide_occluders: false,
		}
	}

	pub const fn with_gust(mut self) -> Self {
		self.gust = true;
		self
	}

	pub const fn with_trail(mut self) -> Self {
		self.trail = true;
		self
	}

	pub const fn with_stamp_scale(mut self, stamp_scale: f32) -> Self {
		self.stamp_scale = stamp_scale;
		self
	}

	pub const fn with_mask_overlay(mut self) -> Self {
		self.mask_overlay = true;
		self
	}

	pub const fn hide_occluders(mut self) -> Self {
		self.hide_occluders = true;
		self
	}

	pub fn apply_world(self, world: &mut World) {
		match self.scene {
			CaptureScene::Parting => {
				scene::spawn_parting_scene_with_trail(
					world,
					self.capsule_distance_m,
					self.rock_distance_m,
					if self.trail { 4 } else { 0 },
				);
			}
			CaptureScene::Meadow if self.grass => {
				scene::clear_parting_props(world);
				if !scene::has_patches(world) {
					scene::spawn_default_scene_in_world(world);
				}
			}
			CaptureScene::Meadow => {
				scene::clear_parting_props(world);
				scene::clear_patches(world);
			}
		}
		let mut settings = world.resource_mut::<GrassFieldSettings>();
		settings.wind =
			if self.gust { WindInputs::botw_gust(1.85) } else { WindInputs::gentle_botw(0.85) };
		settings.freeze_time_s = Some(if self.gust { 1.85 } else { 0.85 });
		settings.parting.mode = self.parting_mode;
		settings.parting.stamp_scale = if self.stamp_scale > 0.0 {
			self.stamp_scale
		} else {
			GrassPartingSettings::default().stamp_scale
		};
		if let Some(mut debug) = world.get_resource_mut::<GrassPartingDebug>() {
			debug.mask_overlay = self.mask_overlay;
		}
		if self.hide_occluders {
			let ids: Vec<_> =
				world.query_filtered::<Entity, With<scene::PartingProp>>().iter(world).collect();
			for id in ids {
				if let Some(mut visibility) = world.get_mut::<Visibility>(id) {
					*visibility = Visibility::Hidden;
				}
			}
		}
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

const PARTING_NEAR: f32 = 0.85;
const PARTING_MID: f32 = 1.35;
const PARTING_FAR: f32 = 3.15;

pub const PARTING_CAPTURE_SHOTS: &[CaptureShot] = &[
	CaptureShot::parting(
		"parting-off-ground",
		scene::PARTING_GROUND,
		GrassPartingMode::Off,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-depth-ground",
		scene::PARTING_GROUND,
		GrassPartingMode::Depth,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-raster-ground",
		scene::PARTING_GROUND,
		GrassPartingMode::Raster,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-world-ground",
		scene::PARTING_GROUND,
		GrassPartingMode::World,
		PARTING_MID,
	),
	CaptureShot::parting("parting-off-ots", scene::PARTING_OTS, GrassPartingMode::Off, PARTING_MID),
	CaptureShot::parting(
		"parting-depth-ots",
		scene::PARTING_OTS,
		GrassPartingMode::Depth,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-raster-ots",
		scene::PARTING_OTS,
		GrassPartingMode::Raster,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-world-ots",
		scene::PARTING_OTS,
		GrassPartingMode::World,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-depth-near",
		scene::PARTING_GROUND,
		GrassPartingMode::Depth,
		PARTING_NEAR,
	),
	CaptureShot::parting(
		"parting-depth-far",
		scene::PARTING_GROUND,
		GrassPartingMode::Depth,
		PARTING_FAR,
	),
	CaptureShot::parting(
		"parting-world-near",
		scene::PARTING_GROUND,
		GrassPartingMode::World,
		PARTING_NEAR,
	),
	CaptureShot::parting(
		"parting-world-far",
		scene::PARTING_GROUND,
		GrassPartingMode::World,
		PARTING_FAR,
	),
	CaptureShot::parting(
		"parting-depth-wind",
		scene::PARTING_GROUND,
		GrassPartingMode::Depth,
		PARTING_MID,
	)
	.with_gust(),
	CaptureShot::parting(
		"parting-world-wind",
		scene::PARTING_GROUND,
		GrassPartingMode::World,
		PARTING_MID,
	)
	.with_gust(),
	CaptureShot::parting(
		"parting-off-topdown",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Off,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-depth-topdown",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Depth,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-raster-topdown",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-raster-mask-overlay-topdown",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_mask_overlay(),
	CaptureShot::parting(
		"parting-raster-topdown-nocapsule",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.hide_occluders(),
	CaptureShot::parting(
		"parting-world-topdown",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::World,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-raster-topdown-trail",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_trail(),
	CaptureShot::parting(
		"parting-raster-topdown-scale1",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_stamp_scale(1.0),
	CaptureShot::parting(
		"parting-raster-topdown-scale2",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_stamp_scale(2.0),
	CaptureShot::parting(
		"parting-raster-topdown-scale3",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_stamp_scale(3.0),
	CaptureShot::parting(
		"parting-raster-ots-oversized",
		scene::PARTING_OTS,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_stamp_scale(3.0),
];

pub const RASTER_STAMP_CAPTURE_SHOTS: &[CaptureShot] = &[
	CaptureShot::parting(
		"parting-off-topdown",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Off,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-raster-topdown-scale1",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_stamp_scale(1.0),
	CaptureShot::parting(
		"parting-raster-topdown-scale2",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_stamp_scale(2.0),
	CaptureShot::parting(
		"parting-raster-topdown-scale3",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_stamp_scale(3.0),
	CaptureShot::parting(
		"parting-raster-topdown-trail",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_trail(),
	CaptureShot::parting(
		"parting-raster-ots-oversized",
		scene::PARTING_OTS,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_stamp_scale(3.0),
	CaptureShot::parting(
		"parting-raster-topdown",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-raster-mask-overlay-topdown",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_mask_overlay(),
	CaptureShot::parting(
		"parting-raster-topdown-nocapsule",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.hide_occluders(),
];

pub const RASTER_DEBUG_CAPTURE_SHOTS: &[CaptureShot] = &[
	CaptureShot::parting(
		"parting-off-topdown",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Off,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-raster-mask-overlay-topdown",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_mask_overlay(),
	CaptureShot::parting(
		"parting-raster-topdown",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	),
	CaptureShot::parting(
		"parting-raster-topdown-nocapsule",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.hide_occluders(),
	CaptureShot::parting(
		"parting-raster-topdown-trail",
		scene::PARTING_TOPDOWN,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_trail(),
	CaptureShot::parting(
		"parting-raster-ots-oversized",
		scene::PARTING_OTS,
		GrassPartingMode::Raster,
		PARTING_MID,
	)
	.with_stamp_scale(3.0),
];

pub fn active_capture_shots() -> &'static [CaptureShot] {
	match std::env::var(CAPTURE_SET_ENV).as_deref() {
		Ok("parting") => PARTING_CAPTURE_SHOTS,
		Ok("parting-raster-stamp") => RASTER_STAMP_CAPTURE_SHOTS,
		Ok("parting-raster-debug") => RASTER_DEBUG_CAPTURE_SHOTS,
		_ => GRASS_CAPTURE_SHOTS,
	}
}

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
				let shots = active_capture_shots();
				view.apply_camera(&shots[0]);
				let shot = shots[0];
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
				if let Some(shot) = active_capture_shots().get(index).copied() {
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
			let Some(shot) = active_capture_shots().get(index) else {
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
				if next >= active_capture_shots().len() {
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
			*transform = camera_looking_at(shot.camera, shot.look_at);
			let (yaw, pitch) = yaw_pitch(transform.rotation);
			controller.yaw = yaw;
			controller.pitch = pitch;
		}
	}
}

fn camera_looking_at(from: Vec3, look_at: Vec3) -> Transform {
	let dir = (look_at - from).normalize_or(Vec3::NEG_Y);
	let up = if dir.dot(Vec3::Y).abs() > 0.95 { Vec3::NEG_Z } else { Vec3::Y };
	Transform::from_translation(from).looking_at(look_at, up)
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
		for name in [
			"parting-off-ground",
			"parting-depth-ground",
			"parting-raster-ground",
			"parting-world-ground",
			"parting-off-ots",
			"parting-depth-ots",
			"parting-world-near",
			"parting-world-far",
			"parting-depth-wind",
			"parting-world-wind",
		] {
			assert!(PARTING_CAPTURE_SHOTS.iter().any(|shot| shot.name == name), "missing {name}");
		}
		let off = PARTING_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "parting-off-ground")
			.unwrap();
		let on = PARTING_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "parting-depth-ground")
			.unwrap();
		assert_eq!(off.camera, on.camera);
		assert_eq!(off.look_at, on.look_at);
		assert_ne!(off.parting_mode, on.parting_mode);
		for name in [
			"parting-off-topdown",
			"parting-depth-topdown",
			"parting-raster-topdown",
			"parting-world-topdown",
			"parting-raster-topdown-trail",
		] {
			assert!(PARTING_CAPTURE_SHOTS.iter().any(|shot| shot.name == name), "missing {name}");
		}
		let topdown: Vec<_> = PARTING_CAPTURE_SHOTS
			.iter()
			.filter(|shot| shot.name.ends_with("-topdown") || shot.name.ends_with("-topdown-trail"))
			.collect();
		assert_eq!(topdown.len(), 6);
		for shot in &topdown {
			assert_eq!(shot.camera, scene::PARTING_TOPDOWN.0);
			assert_eq!(shot.look_at, scene::PARTING_TOPDOWN.1);
			assert_eq!(shot.capsule_distance_m, PARTING_MID);
		}
		let trail = PARTING_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "parting-raster-topdown-trail")
			.unwrap();
		assert!(trail.trail);
		assert_eq!(trail.parting_mode, GrassPartingMode::Raster);
		let raster = PARTING_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "parting-raster-topdown")
			.unwrap();
		assert!(!raster.trail);
		for (name, scale) in [
			("parting-raster-topdown-scale1", 1.0),
			("parting-raster-topdown-scale2", 2.0),
			("parting-raster-topdown-scale3", 3.0),
		] {
			let shot = PARTING_CAPTURE_SHOTS.iter().find(|shot| shot.name == name).unwrap();
			assert_eq!(shot.camera, scene::PARTING_TOPDOWN.0);
			assert_eq!(shot.look_at, scene::PARTING_TOPDOWN.1);
			assert_eq!(shot.parting_mode, GrassPartingMode::Raster);
			assert!((shot.stamp_scale - scale).abs() < 1e-5);
		}
		let oversized = PARTING_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "parting-raster-ots-oversized")
			.unwrap();
		assert_eq!(oversized.camera, scene::PARTING_OTS.0);
		assert_eq!(oversized.parting_mode, GrassPartingMode::Raster);
		assert!((oversized.stamp_scale - 3.0).abs() < 1e-5);
		assert_eq!(RASTER_STAMP_CAPTURE_SHOTS.len(), 9);
		assert_eq!(RASTER_STAMP_CAPTURE_SHOTS[0].parting_mode, GrassPartingMode::Off);
		let overlay = RASTER_DEBUG_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "parting-raster-mask-overlay-topdown")
			.unwrap();
		assert!(overlay.mask_overlay);
		assert!(!overlay.hide_occluders);
		let nocapsule = RASTER_DEBUG_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "parting-raster-topdown-nocapsule")
			.unwrap();
		assert!(nocapsule.hide_occluders);
		assert!(!nocapsule.mask_overlay);
		assert_eq!(RASTER_DEBUG_CAPTURE_SHOTS.len(), 6);
		let pose = camera_looking_at(scene::PARTING_TOPDOWN.0, scene::PARTING_TOPDOWN.1);
		assert!(pose.translation.is_finite());
		let dir = (scene::PARTING_TOPDOWN.1 - scene::PARTING_TOPDOWN.0).normalize();
		assert!(dir.dot(Vec3::NEG_Y) > 0.95, "expected a near-vertical look, dir {dir:?}");
		Ok(())
	}
}
