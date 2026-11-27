//! Offline-only readback harness. Link against the unchanged reference's compiled libraries.
mod combined_reference_args;
use bevy::{
	prelude::*,
	render::gpu_readback::{Readback, ReadbackComplete, ReadbackOnce},
};
use game_commands::command::GameCommand;
use maybraid_vfx::parts::combined::{
	CombinedState,
	render::{CombinedMaterial, CombinedRenderer},
};
use vfx_playground::{PlaygroundCommand, VfxPlaygroundPlugin};
#[derive(Resource)]
struct Export {
	index: u32,
	settled: u32,
	pending: bool,
	begun: bool,
	output: std::path::PathBuf,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = combined_reference_args::ReaderArgs::parse()?;
	let output = args.output;
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(AssetPlugin {
					file_path: args.assets.to_string_lossy().into_owned(),
					..default()
				})
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Combined offline reference export".into(),
						resolution: (640, 360).into(),
						..default()
					}),
					..default()
				}),
		)
		.add_plugins(VfxPlaygroundPlugin)
		.insert_resource(Export { index: 1, settled: 0, pending: false, begun: false, output })
		.add_systems(Last, capture)
		.run();
	Ok(())
}
fn command(text: &str, commands: &mut Commands) {
	match PlaygroundCommand::parse_line(text) {
		Ok(c) => c.react(commands, &mut String::new()),
		Err(e) => {
			eprintln!("{e}");
			std::process::exit(1)
		}
	}
}
fn capture(
	mut commands: Commands,
	mut export: ResMut<Export>,
	clock: Res<Time<Virtual>>,
	states: Query<&CombinedState>,
	materials: Res<Assets<CombinedMaterial>>,
	quads: Query<&MeshMaterial3d<CombinedMaterial>>,
	renderer: Res<CombinedRenderer>,
	mut exit: MessageWriter<AppExit>,
) {
	if export.index > 46 {
		exit.write(AppExit::Success);
		return;
	}
	if !export.begun {
		command("show combined-wave --seed 13", &mut commands);
		command("noloop", &mut commands);
		command(&format!("freeze {}", export.index as f64 / 10.), &mut commands);
		export.begun = true;
		return;
	}
	if export.pending || !clock.is_paused() || states.iter().next().is_none_or(|s| !s.ready) {
		return;
	}
	export.settled += 1;
	if export.settled < 12 {
		return;
	}
	let Some(material) = quads.iter().next().and_then(|q| materials.get(&q.0)) else {
		return;
	};
	let p = &renderer.payload.values;
	if p.len() < 22 {
		return;
	}
	let Some(state) = states.iter().next() else {
		return;
	};
	let grains=state.simulation.particles.iter().filter(|p|p.kind==1).map(|p|serde_json::json!({"id":p.id,"birth":p.birth,"life":p.life,"position":p.position.to_array(),"velocity":p.velocity.to_array(),"scale":p.scale.to_array(),"color":p.color.to_array()})).collect::<Vec<_>>();
	let metadata = serde_json::json!({"time":export.index as f64/10.,"bounds_min":p[1].to_array(),"bounds_max":p[2].to_array(),"style":p[4].to_array(),"dimensions":[192,96,160],"grains":grains});
	let index = export.index;
	let folder = export.output.clone();
	export.pending = true;
	commands
		.spawn((Readback::texture(material.field.clone()), ReadbackOnce))
		.observe(
			move |event: On<ReadbackComplete>,
			      mut export: ResMut<Export>,
			      mut commands: Commands| {
				let write = || -> Result<(), Box<dyn std::error::Error>> {
					std::fs::write(folder.join(format!("{index:02}.rgba16")), &event.data)?;
					std::fs::write(
						folder.join(format!("{index:02}.json")),
						serde_json::to_vec_pretty(&metadata)?,
					)?;
					Ok(())
				};
				if let Err(e) = write() {
					eprintln!("{e}");
					std::process::exit(1)
				}
				println!("exported {index}");
				commands.entity(event.entity).try_despawn();
				command("cancel", &mut commands);
				command("play", &mut commands);
				export.index += 1;
				export.pending = false;
				export.begun = false;
				export.settled = 0;
			},
		);
}
