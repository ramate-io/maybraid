//! Screenshot capture for debug validation (debug-commands feature only).
//!
//! Implements timed frame capture using Bevy's Screenshot API.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use player::Player;
use world_player;

use crate::commands::RequestCapture;

const WARMUP: Duration = Duration::from_millis(2000);
const SETTLE: Duration = Duration::from_millis(500);

/// Capture state tracking for timed screenshot sequences.
#[derive(Resource, Default)]
pub struct CaptureSession {
	pub dir: Option<PathBuf>,
	pub interval_secs: f32,
	pub count: u32,
	frame: u32,
	age: f32,
	warmed: bool,
	settle_from: Option<Instant>,
}

impl CaptureSession {
	pub fn start(dir: PathBuf, interval_secs: f32, count: u32) -> Self {
		Self {
			dir: Some(dir),
			interval_secs,
			count,
			frame: 0,
			age: 0.0,
			warmed: false,
			settle_from: None,
		}
	}

	fn is_active(&self) -> bool {
		self.dir.is_some()
	}
}

/// Apply capture requests from the command system.
pub fn apply_capture_requests(
	requests: Query<(Entity, &RequestCapture)>,
	mut session: ResMut<CaptureSession>,
	player: Query<Entity, (With<player::Player>, With<world_player::Player>)>,
	mut commands: Commands,
) {
	for (entity, request) in &requests {
		if session.is_active() {
			warn!("Capture already in progress, ignoring new request");
			commands.entity(entity).despawn();
			continue;
		}

		// Wait for world ready before starting capture
		if player.get_single().is_err() {
			warn!("Capture requested but world not ready (player not spawned), deferring");
			continue;
		}

		let _ = std::fs::create_dir_all(&request.dir);
		*session = CaptureSession::start(
			request.dir.clone(),
			request.interval_secs,
			request.count,
		);
		info!(
			"Starting capture: {} frames every {:.1}s to {}",
			request.count,
			request.interval_secs,
			request.dir.display()
		);
		commands.entity(entity).despawn();
	}
}

/// Tick capture session: warmup, settle, shoot at intervals.
pub fn tick_capture(
	time: Res<Time>,
	mut session: ResMut<CaptureSession>,
	mut commands: Commands,
) {
	let Some(dir) = session.dir.clone() else {
		return;
	};

	// Warmup phase
	if !session.warmed {
		session.age += time.delta_secs();
		if session.age >= WARMUP.as_secs_f32() {
			session.warmed = true;
			session.age = 0.0;
			session.settle_from = Some(Instant::now());
			info!("Capture warmup complete, starting sequence");
		}
		return;
	}

	// Settle phase (wall time, not game time)
	if let Some(from) = session.settle_from {
		if from.elapsed() < SETTLE {
			return;
		}
		session.settle_from = None;
	}

	// Check if we're done
	if session.frame >= session.count {
		info!("Capture complete: {} frames", session.count);
		session.dir = None;
		return;
	}

	// Check if it's time for next frame
	let due_time = session.frame as f32 * session.interval_secs;
	if session.age < due_time {
		session.age += time.delta_secs();
		return;
	}

	// Shoot!
	let path = dir.join(format!("frame_{:04}.png", session.frame));
	info!("Capturing frame {}/{}: {}", session.frame + 1, session.count, path.display());
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
	
	session.frame += 1;
	session.age += time.delta_secs();
	session.settle_from = Some(Instant::now());
}
