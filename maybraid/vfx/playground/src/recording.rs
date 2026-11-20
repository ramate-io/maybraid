//! Bounded normal or dense offline evidence through Bevy GPU readback.
use bevy::{
	prelude::*,
	render::view::screenshot::{Screenshot, ScreenshotCaptured},
};
use std::{
	path::PathBuf,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
	time::Instant,
};
#[derive(Component)]
pub(crate) struct PendingRecording {
	pub path: PathBuf,
	pub seconds: f32,
	pub half_size: bool,
	pub remaining: f32,
	pub replay: bool,
	pub sampled: bool,
}
pub(crate) fn pending(
	mut commands: Commands,
	time: Res<Time<Real>>,
	mut pending: Query<(Entity, &mut PendingRecording)>,
) {
	for (e, mut p) in &mut pending {
		p.remaining -= time.delta_secs();
		if p.remaining > 0. {
			continue;
		}
		if p.replay {
			crate::commands::PlaygroundCommand::Reset.react(&mut commands, &mut String::new());
		}
		let (path, seconds, half_size) = (p.path.clone(), p.seconds, p.half_size);
		let sampled = p.sampled;
		commands.queue(move |w: &mut World| {
			w.resource_mut::<crate::commands::LoopEnabled>().0 = false;
			start(w, path, seconds, half_size, sampled);
		});
		commands.entity(e).try_despawn();
	}
}
#[derive(Component)]
pub(crate) struct Recording {
	path: PathBuf,
	elapsed: f32,
	duration: f32,
	index: usize,
	rows: String,
	inflight: Arc<AtomicUsize>,
	original_window: Option<(Entity, Vec2)>,
	drawer_visible: Option<bool>,
	sampled: bool,
	original_show: Option<crate::show::LastVfxShow>,
	started: Instant,
	max_frames: usize,
}
pub(crate) fn start(
	world: &mut World,
	path: PathBuf,
	seconds: f32,
	half_size: bool,
	sampled: bool,
) {
	if world.query::<&Recording>().iter(world).next().is_some() {
		warn!("Only one recording at a time");
		return;
	}
	if path.exists() && std::fs::read_dir(&path).is_ok_and(|mut d| d.next().is_some()) {
		warn!("Recording directory must be empty");
		return;
	}
	if let Err(e) = std::fs::create_dir_all(&path) {
		warn!("Recording folder: {e}");
		return;
	}
	let original_window = if half_size {
		world
			.query_filtered::<(Entity, &mut Window), With<bevy::window::PrimaryWindow>>()
			.iter_mut(world)
			.next()
			.map(|(e, mut w)| {
				let old = Vec2::new(w.resolution.width(), w.resolution.height());
				w.resolution.set(old.x * 0.5, old.y * 0.5);
				(e, old)
			})
	} else {
		None
	};
	let drawer_visible =
		world
			.get_resource_mut::<game_commands::GameCommandDrawerVisible>()
			.map(|mut d| {
				let old = d.0;
				d.0 = false;
				old
			});
	if let Some(mut f) = world.get_resource_mut::<game_commands::command::TextEntryFocus>() {
		f.0 = false;
	}
	// Offline acquisition only: keep Bevy's real clock and render path untouched.
	// The encode uses recorded host ages to restore authored speed; normal clips
	// and performance runs never select this option.
	let original_show = if sampled {
		let old = world.resource::<crate::show::LastVfxShow>().clone();
		for request in &mut world.resource_mut::<crate::commands::PendingVfxShows>().0 {
			request.playback = 0.25;
		}
		Some(old)
	} else {
		None
	};
	let authored = world.resource::<crate::show::LastVfxShow>().request.as_ref().is_some_and(|r| {
		matches!(
			r.effect.as_str(),
			"cartoon-explosion" | "combined-explosion" | "explosion-compare"
		)
	});
	let travelling = world.query::<&crate::herd::HerdActors>().iter(world).next().is_some()
		&& world
			.resource::<crate::herd::LastHerd>()
			.0
			.as_ref()
			.is_some_and(|c| c.course == "travel");
	world.spawn(Recording {
		max_frames: if travelling {
			1200
		} else if authored {
			480
		} else {
			240
		},
		sampled,
		original_show,
		started: Instant::now(),
		drawer_visible,
		original_window,
		path,
		elapsed: 0.,
		duration: (if seconds.is_finite() {
			seconds.clamp(
				0.1,
				if travelling {
					40.
				} else if authored {
					15.
				} else {
					6.
				},
			)
		} else {
			5.
		}) * if sampled { 4. } else { 1. },
		index: 0,
		rows: "wall_seconds,host_age,cartoon_tick,file\n".into(),
		inflight: Arc::new(AtomicUsize::new(0)),
	});
}
pub(crate) fn capture(
	mut commands: Commands,
	time: Res<Time<Real>>,
	mut recordings: Query<(Entity, &mut Recording)>,
	instances: Query<&maybraid_vfx::VfxInstance>,
	cartoon: Query<&maybraid_vfx::parts::cartoon_flow::CartoonFlow>,
	mut windows: Query<&mut Window>,
	mut drawer: Option<ResMut<game_commands::GameCommandDrawerVisible>>,
) {
	for (e, mut r) in &mut recordings {
		r.elapsed += time.delta_secs();
		let done = if r.sampled {
			r.elapsed > r.duration || r.index >= 1440
		} else {
			r.elapsed > r.duration || r.index >= r.max_frames
		};
		if done && r.inflight.load(Ordering::Relaxed) == 0 {
			if let Err(e) = std::fs::write(r.path.join("frames.csv"), &r.rows) {
				warn!("Recording metadata: {e}");
			}
			if let Some(v) = r.drawer_visible {
				if let Some(d) = drawer.as_deref_mut() {
					d.0 = v;
				}
			}
			if let Some((e, size)) = r.original_window {
				if let Ok(mut w) = windows.get_mut(e) {
					w.resolution.set(size.x, size.y);
				}
			}
			if let Some(show) = r.original_show.take() {
				commands.queue(move |world: &mut World| {
					world.insert_resource(show);
				});
			}
			commands.entity(e).try_despawn();
			continue;
		}
		if done {
			continue;
		}
		if r.inflight.load(Ordering::Relaxed) >= 4 {
			continue;
		}
		// Bounded ordinary-speed evidence: faster rendering must not exhaust the
		// 240-frame cap before a five-second event reaches contact/expiry.
		// Sample wall time only; never change the host or effect clock.
		if !r.sampled && r.started.elapsed().as_secs_f32() < r.index as f32 / 30. {
			continue;
		}
		let name = format!("frame-{:04}.ppm", r.index);
		let path = r.path.join(&name);
		let elapsed = r.started.elapsed().as_secs_f32();
		let age = instances.iter().next().map_or(-1., |i| i.age);
		let tick = cartoon.iter().next().map_or(0, |c| c.tick);
		r.rows += &format!("{elapsed},{age},{tick},{name}\n");
		r.index += 1;
		let inflight = r.inflight.clone();
		inflight.fetch_add(1, Ordering::Relaxed);
		commands.spawn(Screenshot::primary_window()).observe(
			move |shot: On<ScreenshotCaptured>| {
				let image = shot.image.clone();
				let path = path.clone();
				let inflight = inflight.clone();
				let _ = std::thread::spawn(move || {
					match image.try_into_dynamic() {
						Ok(img) => {
							let rgb = img.into_rgb8();
							let w = rgb.width() as usize;
							let h = rgb.height() as usize;
							let step = if w > 1280 { 2 } else { 1 };
							let mut bytes =
								format!("P6\n{} {}\n255\n", w.div_ceil(step), h.div_ceil(step))
									.into_bytes();
							bytes.reserve(w.div_ceil(step) * h.div_ceil(step) * 3);
							for y in (0..h).step_by(step) {
								for x in (0..w).step_by(step) {
									let i = (y * w + x) * 3;
									bytes.extend_from_slice(&rgb.as_raw()[i..i + 3]);
								}
							}
							if let Err(e) = std::fs::write(path, bytes) {
								warn!("Recording frame: {e}");
							}
						}
						Err(e) => warn!("Recording format: {e}"),
					}
					inflight.fetch_sub(1, Ordering::Relaxed);
				});
			},
		);
	}
}
