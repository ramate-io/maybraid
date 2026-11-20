//! Opt-in native normal-speed timing. Buffered rows; no captures or clock changes.
use bevy::prelude::*;
use maybraid_vfx::{
	parts::{
		cartoon_flow::CartoonFlow,
		combined::{render::CombinedRenderer, CombinedState},
		combined_choreography::OpticalChoreography,
	},
	VfxInstance,
};
use std::{fmt::Write, path::PathBuf, time::Instant};
#[derive(Resource)]
struct Flight {
	path: PathBuf,
	start: Instant,
	frame: Instant,
	rows: String,
	next: usize,
	armed: bool,
}
pub(crate) fn install(app: &mut App) {
	if let Some(path) = std::env::var_os("MAYBRAID_DUST_VARIANT_PROFILE") {
		app.init_resource::<maybraid_vfx::parts::cartoon_flow::diagnostics::Probe>().add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin).insert_resource(Flight{path:path.into(),start:Instant::now(),frame:Instant::now(),rows:"real_s,age,frame_ms,main_wall_ms,focused,body_cpu_ms,particle_cpu_ms,body_steps,field_steps,bodies,clouds,grains,mesh_assets,image_assets,primitives,volume_refs,payload_bytes,region_count,region_primitives,region_field_bytes,region_buffer_bytes,controller_cpu_ms,flow_cpu_ms,reconstruction_cpu_ms,rebuilds\n".into(),next:0,armed:false}).add_systems(First,begin).add_systems(Last,end);
	}
}
fn begin(
	mut f: ResMut<Flight>,
	mut probe: ResMut<maybraid_vfx::parts::cartoon_flow::diagnostics::Probe>,
	last: Res<crate::show::LastVfxShow>,
	contact: Res<crate::contact_regions::Last>,
	mut commands: Commands,
) {
	f.frame = Instant::now();
	probe.frame = default();
	let real = f.start.elapsed().as_secs_f64();
	if f.next < 3 && real >= 8. + f.next as f64 * 7. {
		if let Some(config) = contact.0.clone() {
			f.next += 1;
			f.armed = true;
			commands.queue(move |w: &mut World| config.start(w));
		} else if let Some(request) = last.request.clone() {
			f.next += 1;
			f.armed = true;
			commands.queue(move |w: &mut World| {
				let roots =
					w.query_filtered::<Entity, With<VfxInstance>>().iter(w).collect::<Vec<_>>();
				for e in roots {
					w.despawn(e);
				}
				w.resource_mut::<crate::commands::PreviewFreeze>().age = None;
				w.resource_mut::<crate::commands::LoopEnabled>().0 = false;
				w.resource_mut::<crate::show::RepeatingVfxShow>().request = None;
				w.resource_mut::<crate::commands::PendingVfxShows>().0.push(request);
			});
		}
	}
}
fn end(
	mut f: ResMut<Flight>,
	time: Res<Time<Real>>,
	windows: Query<&Window>,
	roots: Query<&VfxInstance>,
	flow: Query<&CartoonFlow>,
	optical: Query<&OpticalChoreography>,
	combined: Query<&CombinedState>,
	meshes: Res<Assets<Mesh>>,
	images: Res<Assets<Image>>,
	renderer: Res<CombinedRenderer>,
	probe: Res<maybraid_vfx::parts::cartoon_flow::diagnostics::Probe>,
	mut exit: MessageWriter<AppExit>,
) {
	let age = roots.iter().map(|i| i.age).fold(0., f32::max);
	let real = f.start.elapsed().as_secs_f64();
	if f.armed && !roots.is_empty() && age > 0.04 {
		let mut body = 0u64;
		let mut particles = 0u64;
		let mut steps = 0;
		let mut nb = 0;
		let mut nc = 0;
		let mut ng = 0;
		for o in &optical {
			body += o.body_ns;
			particles += o.particle_ns;
			steps += o.steps;
			nc += o.particles.iter().filter(|p| p.kind == 0).count();
			ng += o.particles.iter().filter(|p| p.kind == 1).count();
		}
		for c in &flow {
			nb += c.bodies.iter().flatten().count();
		}
		for c in &combined {
			nb += (0..15)
				.filter(|id| c.simulation.pose(c.body_time as f64, *id).is_some())
				.count();
			nc += c.simulation.particles.iter().filter(|p| p.kind == 0).count();
			ng += c.simulation.particles.iter().filter(|p| p.kind == 1).count();
		}
		let field_steps =
			flow.iter().filter_map(|c| c.field.as_ref()).map(|s| s.steps).sum::<u32>();
		let frame = time.delta_secs_f64() * 1000.;
		let wall = f.frame.elapsed().as_secs_f64() * 1000.;
		let focused = windows.iter().all(|w| w.focused);
		let region = renderer.region_resources();
		use maybraid_vfx::parts::cartoon_flow::diagnostics::Stage;
		let controller_cpu = probe.frame.ns[Stage::Tick as usize] as f64 / 1e6;
		let flow_cpu = if optical.iter().next().is_some() {
			-1.
		} else {
			probe.frame.ns[Stage::Flow as usize] as f64 / 1e6
		};
		let reconstruction_cpu =
			[Stage::ScalarSetup, Stage::Scalar, Stage::Gradient, Stage::Tetra, Stage::Package]
				.iter()
				.map(|s| probe.frame.ns[*s as usize] as f64 / 1e6)
				.sum::<f64>();
		let rebuilds = probe.frame.rebuilds;
		let _=writeln!(f.rows,"{real},{age},{frame},{wall},{focused},{},{},{steps},{field_steps},{nb},{nc},{ng},{},{},{},{},{},{},{},{},{},{controller_cpu},{flow_cpu},{reconstruction_cpu},{rebuilds}",body as f64/1e6,particles as f64/1e6,meshes.len(),images.len(),renderer.primitives,renderer.payload.references,renderer.payload.values.len()*16,region.0,region.1,region.2,region.3);
	}
	if real > 30. {
		if let Some(p) = f.path.parent() {
			let _ = std::fs::create_dir_all(p);
		}
		if let Err(e) = std::fs::write(&f.path, &f.rows) {
			warn!("dust timing: {e}");
		}
		exit.write(AppExit::Success);
	}
}
