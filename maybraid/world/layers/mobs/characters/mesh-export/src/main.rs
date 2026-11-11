//! Assemble a character recipe in-process and write a Wavefront OBJ for DCC import.

use std::num::NonZeroU8;
use std::path::PathBuf;
use std::process::ExitCode;

use bevy::prelude::*;
use bevy::window::WindowPlugin;
use character_mesh_export::export::{
	assembly_block_reason, assembly_ready, assets_root, collect_groups, write_import_zip,
	write_obj_bundle,
};
use characters::{
	spawn_fixed_character_assembly, species::epiphant::EpiphantConfig, CharacterAssemblyBudget,
	CharacterHostsPlugin, CharacterRecipe,
};
use clap::{Parser, ValueEnum};
use rigs::RigSystems;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Species {
	Epiphant,
}

#[derive(Parser)]
#[command(name = "character-mesh-export")]
#[command(about = "Export an assembled Maybraid character to OBJ (rest pose, Y-up)")]
struct Cli {
	#[arg(long, value_enum, default_value_t = Species::Epiphant)]
	species: Species,

	/// Output `.obj` path (mesh only: no MTL or textures).
	#[arg(long)]
	output: PathBuf,

	/// Optional zip of the OBJ plus a short README for handing to an iPad.
	#[arg(long)]
	zip: Option<PathBuf>,

	/// Maximum frames to wait for GLB load + skin remap.
	#[arg(long, default_value_t = 600)]
	timeout_frames: u32,
}

#[derive(Resource)]
struct ExportJob {
	output: PathBuf,
	zip: Option<PathBuf>,
	timeout_frames: u32,
	frames: u32,
	root: Option<Entity>,
	species: Species,
}

fn main() -> ExitCode {
	let cli = Cli::parse();
	let assets_path = assets_root();
	if !assets_path.is_dir() {
		eprintln!("assets root not found: {}", assets_path.display());
		return ExitCode::from(2);
	}

	let mut app = App::new();
	app.add_plugins(
		DefaultPlugins
			.set(WindowPlugin {
				primary_window: None,
				exit_condition: bevy::window::ExitCondition::DontExit,
				..default()
			})
			.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() }),
	)
	.add_plugins(CharacterHostsPlugin)
	.insert_resource(ExportJob {
		output: cli.output,
		zip: cli.zip,
		timeout_frames: cli.timeout_frames,
		frames: 0,
		root: None,
		species: cli.species,
	})
	.insert_resource(CharacterAssemblyBudget {
		spawn_weights_per_frame: 10_000,
		spawn_time_per_frame: std::time::Duration::from_secs(30),
	})
	.add_systems(Startup, spawn_character)
	.add_systems(
		PostUpdate,
		tick_export.after(TransformSystems::Propagate).after(RigSystems::Pose),
	);

	let exit = app.run();
	match exit {
		AppExit::Success => ExitCode::SUCCESS,
		AppExit::Error(code) => ExitCode::from(code.get()),
	}
}

fn app_error(code: u8) -> AppExit {
	AppExit::Error(NonZeroU8::new(code).unwrap_or(NonZeroU8::MIN))
}

fn spawn_character(mut commands: Commands, mut job: ResMut<ExportJob>) {
	let recipe = match job.species {
		Species::Epiphant => CharacterRecipe::clothed(&EpiphantConfig::default_preview()),
	};
	let root = spawn_fixed_character_assembly(&mut commands, &recipe, Transform::IDENTITY);
	job.root = Some(root);
}

fn tick_export(world: &mut World) {
	let mut finished = false;
	let mut exit = AppExit::Success;

	world.resource_scope(|world, mut job: Mut<ExportJob>| {
		job.frames += 1;
		let Some(root) = job.root else {
			return;
		};
		if !assembly_ready(world, root) {
			if job.frames >= job.timeout_frames {
				let reason = assembly_block_reason(world, root).unwrap_or("unknown");
				eprintln!(
					"timed out waiting for character assembly ({} frames; last blocker: {})",
					job.frames, reason
				);
				exit = app_error(3);
				finished = true;
			}
			return;
		}

		let groups = collect_groups(world, root);
		if groups.is_empty() {
			if job.frames >= job.timeout_frames {
				eprintln!("assembly ready but no mesh groups collected");
				exit = app_error(4);
				finished = true;
			}
			return;
		}

		let output = job.output.clone();
		match write_obj_bundle(&output, &groups) {
			Ok(report) => {
				if !report.validation_ok {
					eprintln!("OBJ / UV validation failed");
					exit = app_error(5);
					finished = true;
					return;
				}
				if let Some(zip_path) = &job.zip {
					if let Err(error) = write_import_zip(zip_path, &output) {
						eprintln!("zip failed: {error}");
						exit = app_error(6);
						finished = true;
						return;
					}
					println!("Wrote {}", zip_path.display());
				}
				println!("Wrote {}", output.display());
				println!(
					"groups={} vertices={} faces={} uvs={} normals={} bbox_min=({:.3}, {:.3}, {:.3}) bbox_max=({:.3}, {:.3}, {:.3}) obj_bytes={}",
					report.groups,
					report.vertices,
					report.faces,
					report.has_uvs,
					report.has_normals,
					report.bounds_min.x,
					report.bounds_min.y,
					report.bounds_min.z,
					report.bounds_max.x,
					report.bounds_max.y,
					report.bounds_max.z,
					report.obj_bytes,
				);
				finished = true;
			}
			Err(error) => {
				eprintln!("export failed: {error}");
				exit = app_error(5);
				finished = true;
			}
		}
	});

	if finished {
		world.write_message(exit);
	}
}
