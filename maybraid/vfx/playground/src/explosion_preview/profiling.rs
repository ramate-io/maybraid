//! Opt-in buffered native flights. Timing excludes capture and readback.
use bevy::prelude::*;
use maybraid_vfx::parts::smoke_explosion::{ExplosionDiagnostics, ExplosionLayer};
use std::{fmt::Write, path::PathBuf, time::Instant};
#[derive(Resource)]
struct Flight {
	path: PathBuf,
	start: Instant,
	frame: Instant,
	rows: String,
	count: usize,
	diagnostics: String,
	armed: bool,
	instances: usize,
}
pub fn install(app: &mut App) {
	if let Some(path) = std::env::var_os("MAYBRAID_EXPLOSION_PROFILE") {
		app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin).insert_resource(Flight{armed:false,instances:std::env::var("MAYBRAID_EXPLOSION_PROFILE_INSTANCES").ok().and_then(|x|x.parse().ok()).unwrap_or(1).clamp(1,8),diagnostics:"real_s,path,value_ms\n".into(),path:path.into(),start:Instant::now(),frame:Instant::now(),rows:"real_s,age,frame_ms,main_schedule_wall_ms,focused,instances,visible_elements,meshes,images,rejected\n".into(),count:0}).add_systems(First,begin).add_systems(Last,end);
	}
}
fn begin(mut flight: ResMut<Flight>, last: Res<crate::show::LastVfxShow>, mut commands: Commands) {
	flight.frame = Instant::now();
	// Start a normal event group only after a full authored warmup cycle. No clock override.
	if !flight.armed && flight.start.elapsed().as_secs_f64() >= 16. {
		if let Some(base) = last.request.clone() {
			flight.armed = true;
			let count = flight.instances;
			commands.queue(move |w: &mut World| {
				let roots = w
					.query_filtered::<Entity, With<maybraid_vfx::VfxInstance>>()
					.iter(w)
					.collect::<Vec<_>>();
				for e in roots {
					w.despawn(e);
				}
				w.resource_mut::<crate::show::RepeatingVfxShow>().request = None;
				w.resource_mut::<crate::commands::LoopEnabled>().0 = false;
				w.resource_mut::<crate::commands::PendingVfxShows>().0.clear();
				for i in 0..count {
					let mut r = base.clone();
					r.distance = if count > 1 { i as f32 - 1. } else { 0. };
					w.resource_mut::<crate::commands::PendingVfxShows>().0.push(r);
				}
			});
		}
	}
}

fn end(
	time: Res<Time<Real>>,
	states: Query<&ExplosionLayer>,
	windows: Query<&Window>,
	meshes: Res<Assets<Mesh>>,
	images: Res<Assets<Image>>,
	diagnostics: Res<ExplosionDiagnostics>,
	mut flight: ResMut<Flight>,
	mut exit: MessageWriter<AppExit>,
	render: Option<Res<bevy::diagnostic::DiagnosticsStore>>,
) {
	let real = flight.start.elapsed().as_secs_f64();
	let age = states.iter().filter(|s| s.ready).map(|s| s.last_time).fold(0., f32::max);
	if flight.armed && age >= 0.2 && age < 13.5 {
		let wall = flight.frame.elapsed().as_secs_f64() * 1000.;
		let n = states
			.iter()
			.map(|s| s.frame.states.iter().filter(|s| s.visible()).count())
			.sum::<usize>();
		let _ = writeln!(
			flight.rows,
			"{real},{age},{},{wall},{},{},{n},{},{},{}",
			time.delta_secs_f64() * 1000.,
			windows.iter().all(|w| w.focused),
			states.iter().count(),
			meshes.len(),
			images.len(),
			diagnostics.rejected
		);
		flight.count += 1;
		if let Some(render) = render.as_ref() {
			for d in render.iter() {
				let path = d.path().to_string();
				if (path.contains("explosion_density") || path.contains("explosion_light"))
					&& path.contains("elapsed_cpu")
				{
					if let Some(v) = d.value() {
						let _ = writeln!(flight.diagnostics, "{real},{path},{v}");
					}
				}
			}
		}
	}
	if real > 40. {
		if let Some(parent) = flight.path.parent() {
			let _ = std::fs::create_dir_all(parent);
		}
		if let Err(e) = std::fs::write(&flight.path, &flight.rows) {
			warn!("explosion flight: {e}");
		}
		let _ = std::fs::write(flight.path.with_extension("diagnostics.csv"), &flight.diagnostics);
		exit.write(AppExit::Success);
	}
}
