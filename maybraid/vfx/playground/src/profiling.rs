//! Opt-in, bounded diagnostic flights. No camera path, readbacks, or effect clock override.
use crate::commands::PlaygroundCommand;
use bevy::prelude::*;
use game_commands::command::GameCommand;
use maybraid_vfx::parts::{
	cartoon_flow::diagnostics::{Frame, Mode, Probe, STAGES},
	cartoon_landing::LandingMaterial,
};
use std::{
	fmt::Write,
	path::PathBuf,
	time::{Instant, SystemTime, UNIX_EPOCH},
};
#[derive(Clone)]
struct Case {
	name: String,
	effect: &'static str,
	recipe: &'static str,
	scenario: &'static str,
	response: f64,
	mode: Mode,
	paused: bool,
	repeat: usize,
}
#[derive(Resource)]
struct Flight {
	path: PathBuf,
	cases: Vec<Case>,
	index: usize,
	state: u8,
	started: Instant,
	segment: Instant,
	frame_start: Instant,
	epoch_ns: u128,
	rows: Vec<String>,
	seconds: f64,
	gate: bool,
	cold_cache_ns: u64,
}
pub fn install(app: &mut App) {
	let Some(path) = std::env::var_os("MAYBRAID_CARTOON_PROFILE") else {
		return;
	};
	let path = PathBuf::from(path);
	if let Err(e) = std::fs::create_dir_all(&path) {
		warn!("Profile output: {e}");
		return;
	}
	let filter = std::env::var("MAYBRAID_CARTOON_PROFILE_CASE").unwrap_or_default();
	let repeats = std::env::var("MAYBRAID_CARTOON_PROFILE_REPEATS")
		.ok()
		.and_then(|s| s.parse::<usize>().ok())
		.unwrap_or(3)
		.clamp(1, 5);
	let mut cases = Vec::new();
	for repeat in 0..repeats {
		for effect in ["wave", "landing"] {
			for (label, recipe, scenario, response, mode, paused) in [
				(
					"interactive",
					if effect == "wave" {
						"cartoon-wave-interactive"
					} else {
						"cartoon-landing-interactive"
					},
					"cross",
					1.,
					Mode::Normal,
					false,
				),
				(
					"contact",
					if effect == "wave" {
						"cartoon-wave-contact"
					} else {
						"cartoon-landing-contact"
					},
					"cross",
					1.,
					Mode::Normal,
					false,
				),
				(
					"none",
					if effect == "wave" {
						"cartoon-wave-contact"
					} else {
						"cartoon-landing-contact"
					},
					"none",
					1.,
					Mode::Normal,
					false,
				),
				(
					"response0",
					if effect == "wave" {
						"cartoon-wave-contact"
					} else {
						"cartoon-landing-contact"
					},
					"cross",
					0.,
					Mode::Normal,
					false,
				),
				(
					"paused",
					if effect == "wave" {
						"cartoon-wave-contact"
					} else {
						"cartoon-landing-contact"
					},
					"cross",
					1.,
					Mode::Normal,
					true,
				),
				(
					"no-rebuild",
					if effect == "wave" {
						"cartoon-wave-contact"
					} else {
						"cartoon-landing-contact"
					},
					"cross",
					1.,
					Mode::NoRebuild,
					false,
				),
				(
					"no-upload",
					if effect == "wave" {
						"cartoon-wave-contact"
					} else {
						"cartoon-landing-contact"
					},
					"cross",
					1.,
					Mode::NoUpload,
					false,
				),
			] {
				let name = format!("{effect}-{label}-r{}", repeat + 1);
				if filter.is_empty() || name.contains(&filter) {
					cases.push(Case {
						name,
						effect,
						recipe,
						scenario,
						response,
						mode,
						paused,
						repeat,
					});
				}
			}
		}
	}
	let now = Instant::now();
	let epoch_ns = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
	let seconds = std::env::var("MAYBRAID_CARTOON_PROFILE_SECONDS")
		.ok()
		.and_then(|v| v.parse::<f64>().ok())
		.filter(|v| v.is_finite())
		.unwrap_or(16.)
		.clamp(4., 40.);
	let gate = std::env::var_os("MAYBRAID_CARTOON_PROFILE_GATE").is_some();
	app.insert_resource(Probe::default())
		.insert_resource(Flight {
			path,
			cases,
			index: 0,
			state: 0,
			started: now,
			segment: now,
			frame_start: now,
			epoch_ns,
			rows: Vec::with_capacity(3000),
			seconds,
			gate,
			cold_cache_ns: 0,
		})
		.add_systems(First, begin)
		.add_systems(PostUpdate, events.after(bevy::asset::AssetEventSystems))
		.add_systems(Last, finish);
}
fn begin(mut p: ResMut<Probe>, mut f: ResMut<Flight>) {
	p.frame = Frame { age: -1., ..default() };
	f.frame_start = Instant::now();
}
fn events(
	mut p: ResMut<Probe>,
	mut meshes: MessageReader<AssetEvent<Mesh>>,
	mut materials: MessageReader<AssetEvent<LandingMaterial>>,
	changed_meshes: Query<Entity, Changed<Mesh3d>>,
) {
	p.frame.mesh_components = changed_meshes.iter().count();
	p.frame.mesh_events =
		meshes.read().filter(|e| matches!(e, AssetEvent::Modified { .. })).count();
	p.frame.material_events =
		materials.read().filter(|e| matches!(e, AssetEvent::Modified { .. })).count();
}
impl Flight {
	fn activate(&mut self, world: &mut World) {
		let Some(case) = self.cases.get(self.index) else {
			return;
		};
		{
			let mut p = world.resource_mut::<Probe>();
			p.mode = case.mode;
			p.affected.fill(false);
			p.near_solid.fill(false);
		}
		for mut window in world.query::<&mut Window>().iter_mut(world) {
			window.title = format!(
				"Maybraid VFX — PROFILE {}{}",
				case.name,
				if case.mode != Mode::Normal { " — VISUALLY INCORRECT DIAGNOSTIC" } else { "" }
			);
		}
		let mut lines = vec![
			"cancel".to_owned(),
			format!(
				"interaction --scenario {} --contact-response {} --clump-carry 0.15",
				case.scenario, case.response
			),
			format!("show {} --seed 13 --playback 1", case.recipe),
			"inspect close".into(),
		];
		if case.paused {
			lines.extend([
				"noloop".into(),
				format!("replay-at {} --after 1", if case.effect == "wave" { 2.3 } else { 0.45 }),
			]);
		}
		for line in lines {
			match PlaygroundCommand::parse_line(&line) {
				Ok(c) => c.react(&mut world.commands(), &mut String::new()),
				Err(e) => warn!("Profile command {line}: {e}"),
			}
		}
		self.segment = Instant::now();
		self.rows.clear();
		self.cold_cache_ns = 0;
		self.state = 1;
		let status = format!("case={}\nrepeat={}\nstate=warming\n", case.name, case.repeat + 1);
		let _ = std::fs::write(self.path.join("status.txt"), status);
	}
	fn flush(&self, world: &mut World) {
		let case = &self.cases[self.index];
		let mut csv=String::from("wall_s,main_schedule_ms,age,tick,ticks,flow_steps,debt,bodies,affected,near_solid,rebuilds,replacements,generated_vertices,generated_triangles,attribute_upload_bytes,mesh_modified,material_modified,paused,meshes,materials,images,focused,mesh3d_changed");
		for stage in STAGES {
			let _ = write!(csv, ",{stage}_ms");
		}
		csv.push('\n');
		for row in &self.rows {
			csv.push_str(row);
			csv.push('\n');
		}
		if let Err(e) = std::fs::write(self.path.join(format!("{}.csv", case.name)), csv) {
			warn!("Profile save: {e}");
		}
		let window = world
			.query::<&Window>()
			.iter(world)
			.next()
			.map(|w| {
				format!(
					"physical={}x{} logical={}x{} present_requested={:?} focused={}",
					w.physical_width(),
					w.physical_height(),
					w.width(),
					w.height(),
					w.present_mode,
					w.focused
				)
			})
			.unwrap_or_default();
		let meta=format!("case={}\nrecipe={}\nseed=13\nplayback=1\nscenario={}\ncontact_response={}\ndiagnostic={:?}\nepoch_ns={}\nmeasurement_wall_start_s={}\ncache_warm_max_ms={}\n{}\n",case.name,case.recipe,case.scenario,case.response,case.mode,self.epoch_ns,self.segment.duration_since(self.started).as_secs_f64(),self.cold_cache_ns as f64/1e6,window);
		let _ = std::fs::write(self.path.join(format!("{}.meta.txt", case.name)), meta);
	}
}
fn finish(world: &mut World) {
	let Some(mut f) = world.remove_resource::<Flight>() else {
		return;
	};
	if f.index >= f.cases.len() {
		world.insert_resource(f);
		return;
	}
	if f.state == 0 {
		f.activate(world);
		world.insert_resource(f);
		return;
	}
	let now = Instant::now();
	let p = world.resource::<Probe>().frame.clone();
	f.cold_cache_ns = f.cold_cache_ns.max(p.ns[1]);
	if f.state == 1 && f.segment.elapsed().as_secs_f64() >= 6. {
		let case = &f.cases[f.index];
		let should_gate = f.gate
			&& (f.index == 0
				|| case.effect != f.cases[f.index - 1].effect
				|| case.repeat != f.cases[f.index - 1].repeat);
		f.state = if should_gate { 2 } else { 3 };
		f.segment = now;
		let _ = std::fs::write(
			f.path.join("status.txt"),
			format!(
				"case={}\nstate={}\n",
				case.name,
				if should_gate { "ready" } else { "measuring" }
			),
		);
	}
	if f.state == 2 {
		if f.path.join(format!("{}.go", f.cases[f.index].name)).exists() {
			f.state = 3;
			f.segment = now;
			let _ = std::fs::write(
				f.path.join("status.txt"),
				format!("case={}\nstate=measuring\n", f.cases[f.index].name),
			);
		}
	} else if f.state == 3 {
		let paused = world.resource::<Time<Virtual>>().is_paused();
		let mesh_count = world.resource::<Assets<Mesh>>().len();
		let material_count = world.resource::<Assets<LandingMaterial>>().len();
		let image_count = world.resource::<Assets<Image>>().len();
		let focused = world.query::<&Window>().iter(world).next().is_some_and(|w| w.focused);
		let mut row = format!(
			"{:.9},{:.6},{:.9},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
			f.frame_start.duration_since(f.started).as_secs_f64(),
			now.duration_since(f.frame_start).as_secs_f64() * 1000.,
			p.age,
			p.tick,
			p.ticks,
			p.flow_steps,
			p.debt,
			p.bodies,
			p.affected,
			p.near_solid,
			p.rebuilds,
			p.replacements,
			p.vertices,
			p.triangles,
			p.upload_bytes,
			p.mesh_events,
			p.material_events,
			paused,
			mesh_count,
			material_count,
			image_count
		);
		let _ = write!(row, ",{focused},{}", p.mesh_components);
		for ns in p.ns {
			let _ = write!(row, ",{:.6}", ns as f64 / 1e6);
		}
		f.rows.push(row);
		let duration = if f.cases[f.index].paused { 6. } else { f.seconds };
		if f.segment.elapsed().as_secs_f64() >= duration || f.rows.len() >= 10000 {
			f.flush(world);
			f.index += 1;
			f.state = 0;
			if f.index == f.cases.len() {
				let _ = std::fs::write(f.path.join("status.txt"), "state=done\n");
				world.write_message(AppExit::Success);
			}
		}
	}
	world.insert_resource(f);
}
