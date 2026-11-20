//! Opt-in buffered flights, absent from ordinary playback. No capture or host time changes.
use bevy::prelude::*;
use maybraid_vfx::parts::{
	cartoon_flow::diagnostics::{Frame, Probe},
	herd::material::HerdMaterial,
	herd::HerdState,
};
use std::{fmt::Write, path::PathBuf};
#[derive(Resource)]
struct Flight {
	path: PathBuf,
	rows: String,
	last_rebuilds: u64,
	frames: usize,
}
pub fn install(app: &mut App) {
	let Some(path) = std::env::var_os("MAYBRAID_HERD_PROFILE") else {
		return;
	};
	app.insert_resource(Probe::default()).insert_resource(Flight {path:path.into(),rows:"age,frame_ms,focused,ticks,flow_steps,flow_ms,rebuilds,reconstruction_ms,cpu_tick_ms,bytes,bodies,near,clouds,grains,rejected,accepted,debt,mesh_modified,material_modified,meshes,materials,images,herds,shifts,tiles,terrain_misses,regions,optical_primitives,field_bytes,buffer_bytes\n".into(),last_rebuilds:0,frames:0}).add_systems(First,clear).add_systems(Last,collect);
}
fn clear(mut probe: ResMut<Probe>) {
	probe.frame = Frame::default();
}
fn collect(
	time: Res<Time<Real>>,
	states: Query<&HerdState>,
	windows: Query<&Window>,
	probe: Res<Probe>,
	renderer: Res<maybraid_vfx::parts::combined::render::CombinedRenderer>,
	mut flight: ResMut<Flight>,
	meshes: Res<Assets<Mesh>>,
	materials: Res<Assets<HerdMaterial>>,
	images: Res<Assets<Image>>,
	mut mesh_events: MessageReader<AssetEvent<Mesh>>,
	mut material_events: MessageReader<AssetEvent<HerdMaterial>>,
	mut exit: MessageWriter<AppExit>,
) {
	let mesh_count =
		mesh_events.read().filter(|e| matches!(e, AssetEvent::Modified { .. })).count();
	let mat_count = material_events
		.read()
		.filter(|e| matches!(e, AssetEvent::Modified { .. }))
		.count();
	let Some(s) = states.iter().next() else {
		return;
	};
	let age = s.time();

	let f = &probe.frame;
	if age >= 8. && age < 29. {
		let (clouds, grains) = states
			.iter()
			.map(|s| s.fine_counts())
			.fold((0, 0), |(a, b), (c, d)| (a + c, b + d));
		let focused = windows.iter().all(|w| w.focused);
		let ns = &f.ns;
		let reconstruction = (ns[9] + ns[10] + ns[11] + ns[12] + ns[13]) as f64 / 1e6;
		let rebuilds = states.iter().map(|s| s.rebuilds).sum::<u64>() - flight.last_rebuilds;
		let (regions, primitives, field_bytes, buffer_bytes) = renderer.region_resources();
		let _ = writeln!(
			flight.rows,
			"{age},{},{focused},{},{},{},{rebuilds},{reconstruction},{},{},{},{},{clouds},{grains},{},{},{},{mesh_count},{mat_count},{},{},{},{},{},{},{},{regions},{primitives},{field_bytes},{buffer_bytes}",
			time.delta_secs_f64() * 1000.,
			f.ticks,
			f.flow_steps,
			ns[2] as f64 / 1e6,
			ns[0] as f64 / 1e6,
			states.iter().map(|s|s.geometry_bytes).sum::<usize>(),
			states.iter().map(|s|s.active()).sum::<usize>(),
			f.near_solid,
			states.iter().map(|s|s.rejected).sum::<u64>(),
			states.iter().map(|s|s.accepted).sum::<u64>(),
			f.debt,
			meshes.len(),
			materials.len(),
			images.len(),states.iter().count(),states.iter().map(|s|s.field.shifts as u64).sum::<u64>(),states.iter().map(|s|s.tiles.iter().flatten().count()).sum::<usize>(),states.iter().map(|s|s.terrain_misses).sum::<u64>()
		);
		flight.frames += 1;
	}
	flight.last_rebuilds = states.iter().map(|s| s.rebuilds).sum();
	if age >= 29. {
		if let Some(parent) = flight.path.parent() {
			let _ = std::fs::create_dir_all(parent);
		}
		if let Err(e) = std::fs::write(&flight.path, &flight.rows) {
			warn!("herd flight save: {e}");
		}
		exit.write(AppExit::Success);
	}
}
