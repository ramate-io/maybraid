//! Offline reference sampling and native screenshot harness; links unchanged reference artifacts.
use bevy::prelude::*;
use game_commands::command::GameCommand;
use maybraid_vfx::parts::cartoon_landing::motion::Puff;
use vfx_playground::{PlaygroundCommand, VfxPlaygroundPlugin};
#[derive(Resource)]
struct Capture {
	index: usize,
	begun: bool,
	frames: u32,
	output: std::path::PathBuf,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut output = None;
	let mut assets = None;
	let mut samples_only = false;
	let mut args = std::env::args().skip(1);
	while let Some(arg) = args.next() {
		match arg.as_str() {
			"--output" => {
				output =
					Some(std::path::PathBuf::from(args.next().ok_or("--output requires PATH")?))
			}
			"--reference-assets" => {
				assets = Some(std::path::PathBuf::from(
					args.next().ok_or("--reference-assets requires PATH")?,
				))
			}
			"--samples-only" => samples_only = true,
			_ => return Err(format!("unknown argument: {arg}").into()),
		}
	}
	let output = std::path::absolute(output.ok_or("--output PATH required")?)?;
	let assets = std::path::absolute(
		assets
			.ok_or("--reference-assets PATH required (supplied by export_cartoon_reference.py)")?,
	)?;
	if !assets.is_dir() {
		return Err(format!("reference asset root does not exist: {}", assets.display()).into());
	}
	std::fs::create_dir_all(&output)?;
	let tracks=Puff::table(13).into_iter().map(|p| {
  let frames=(0..=288).map(|i|p.state(i as f64/240.).map(|s|serde_json::json!({"translation":s.transform.translation.to_array(),"rotation":s.transform.rotation.to_array(),"scale":s.transform.scale.to_array(),"cutoff":s.threshold}))).collect::<Vec<_>>();
  serde_json::json!({"mesh":format!("vfx/dust/bodies/{}.glb#Mesh0/Primitive0",["Shoulder","Column","Swept"][p.variant]),"seed":p.seed,"birth":p.birth,"lifetime":p.lifetime,"frames":frames})
 }).collect::<Vec<_>>();
	let clip = serde_json::json!({"surface":{"light":[-3.8,5.5,3.0],"thresholds":[0.04,0.62],"colors":[[0.34,0.19,0.085],[0.58,0.37,0.17],[0.84,0.59,0.31]]},"schema_version":1,"fps":240.,"duration":1.2,"tracks":tracks});
	std::fs::write(output.join("landing.mesh_clip.json"), serde_json::to_vec_pretty(&clip)?)?;
	if samples_only {
		return Ok(());
	}
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(AssetPlugin { file_path: assets.to_string_lossy().into_owned(), ..default() })
				.set(WindowPlugin {
					primary_window: Some(Window { resolution: (960, 540).into(), ..default() }),
					..default()
				}),
		)
		.add_plugins(VfxPlaygroundPlugin)
		.insert_resource(Capture { index: 0, begun: false, frames: 0, output })
		.add_systems(PostUpdate, camera.before(bevy::transform::TransformSystems::Propagate))
		.add_systems(Last, capture)
		.run();
	Ok(())
}
const AGES: [f32; 3] = [0.08, 0.30, 0.72];
fn command(text: &str, commands: &mut Commands) {
	match PlaygroundCommand::parse_line(text) {
		Ok(c) => c.react(commands, &mut String::new()),
		Err(e) => eprintln!("{e}"),
	}
}
fn camera(c: Res<Capture>, mut q: Query<&mut Transform, With<Camera3d>>) {
	let p = match c.index % 3 {
		0 => Vec3::new(2.4, 1.6, 3.2),
		1 => Vec3::new(1.3, 0.75, 1.5),
		_ => Vec3::new(0.1, 3.0, 0.2),
	};
	for mut t in &mut q {
		*t = Transform::from_translation(p).looking_at(Vec3::new(0., 0.12, 0.), Vec3::Y);
	}
}
fn capture(
	mut c: ResMut<Capture>,
	time: Res<Time<Virtual>>,
	mut commands: Commands,
	mut exit: MessageWriter<AppExit>,
) {
	if c.index >= 9 {
		c.frames += 1;
		if c.frames > 30 {
			exit.write(AppExit::Success);
		}
		return;
	}
	if !c.begun {
		command("show cartoon-landing --seed 13", &mut commands);
		command("noloop", &mut commands);
		command(&format!("freeze {}", AGES[c.index / 3]), &mut commands);
		c.begun = true;
		return;
	}
	if !time.is_paused() {
		return;
	}
	c.frames += 1;
	if c.frames == 15 {
		PlaygroundCommand::Screenshot {
			path: c.output.join(format!("reference-{}.png", c.index)),
			clean: true,
			geometry: false,
			after: 0.,
		}
		.react(&mut commands, &mut String::new());
	}
	if c.frames > 40 {
		command("cancel", &mut commands);
		c.index += 1;
		c.begun = false;
		c.frames = 0;
	}
}
