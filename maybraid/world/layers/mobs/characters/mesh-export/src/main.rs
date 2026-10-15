//! Assemble a character recipe in-process and write a Wavefront OBJ for DCC import.

mod camera;
mod capture;
mod diagnose;
mod export;
mod obj_preview;
mod obj_text;
mod reference;
mod skin;
mod validate;

use std::num::NonZeroU8;
use std::path::PathBuf;
use std::process::ExitCode;

use bevy::prelude::*;
use bevy::window::WindowPlugin;
use characters::{
	spawn_fixed_character_assembly,
	species::epiphant::EpiphantConfig,
	CharacterAssemblyBudget, CharacterHostSystems, CharacterHostsPlugin, CharacterRecipe,
};
use clap::{Parser, ValueEnum};
use diagnose::{build_diagnose_report, write_diagnose_json};
use export::{assembly_block_reason, assembly_ready, assets_root, collect_groups, write_obj_bundle};
use rigs::RigSystems;
use validate::{build_validation_report, write_validation_json};

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

	/// Output `.obj` path (companion `.mtl` is written alongside).
	#[arg(long)]
	output: PathBuf,

	/// Maximum frames to wait for GLB load + skin remap.
	#[arg(long, default_value_t = 600)]
	timeout_frames: u32,

	/// Directory for GPU runtime captures + OBJ silhouette compare artifacts.
	#[arg(long)]
	compare_dir: Option<PathBuf>,
}

#[derive(Resource)]
struct ExportJob {
	output: PathBuf,
	timeout_frames: u32,
	frames: u32,
	root: Option<Entity>,
	species: Species,
	compare_dir: Option<PathBuf>,
	exported: bool,
}

fn main() -> ExitCode {
	let cli = Cli::parse();
	let assets_path = assets_root();
	if !assets_path.is_dir() {
		eprintln!("assets root not found: {}", assets_path.display());
		return ExitCode::from(2);
	}

	let mut app = App::new();
	let compare_dir = cli.compare_dir.clone();
	app.add_plugins(
		DefaultPlugins
			.set(WindowPlugin {
				primary_window: if compare_dir.is_some() {
					Some(Window {
						title: "character-mesh-export".into(),
						resolution: (camera::CAPTURE_WIDTH, camera::CAPTURE_HEIGHT).into(),
						..default()
					})
				} else {
					None
				},
				exit_condition: bevy::window::ExitCondition::DontExit,
				..default()
			})
			.set(AssetPlugin {
				file_path: assets_path.to_string_lossy().into(),
				..default()
			}),
	)
	.add_plugins(CharacterHostsPlugin)
	.insert_resource(ExportJob {
		output: cli.output,
		timeout_frames: cli.timeout_frames,
		frames: 0,
		root: None,
		species: cli.species,
		compare_dir,
		exported: false,
	})
	.insert_resource(CharacterAssemblyBudget {
		spawn_weights_per_frame: 10_000,
		spawn_time_per_frame: std::time::Duration::from_secs(30),
	})
	.add_systems(Startup, spawn_character)
	.add_systems(
		PostUpdate,
		tick_export
			.after(TransformSystems::Propagate)
			.after(CharacterHostSystems::Fulfill)
			.after(RigSystems::Fulfill)
			.after(RigSystems::Pose),
	)
	.add_systems(Update, capture::tick_gpu_capture);

	let exit = app.run();
	match exit {
		AppExit::Success => ExitCode::SUCCESS,
		AppExit::Error(code) => ExitCode::from(code.get()),
	}
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
		let root = job.root.expect("spawned root");
		if !assembly_ready(world, root) {
			if job.frames >= job.timeout_frames {
				let reason = assembly_block_reason(world, root).unwrap_or("unknown");
				eprintln!(
					"timed out waiting for character assembly ({} frames; last blocker: {})",
					job.frames,
					reason
				);
				exit = AppExit::Error(NonZeroU8::new(3).expect("nonzero"));
				finished = true;
			}
			return;
		}

		if job.exported {
			if job.compare_dir.is_some() && capture::gpu_capture_finished(world) {
				if let Some(dir) = &job.compare_dir {
					println!("GPU compare artifacts: {}", dir.display());
					println!("  cameras: {}", dir.join("cameras.json").display());
					println!(
						"  silhouettes: run scripts/character-mesh-export/compare_silhouettes.py \"{}\" \"{}\"",
						dir.display(),
						job.output.display()
					);
				}
				finished = true;
			}
			return;
		}

		let groups = collect_groups(world, root);
		if groups.is_empty() {
			if job.frames >= job.timeout_frames {
				eprintln!("assembly ready but no mesh groups collected");
				exit = AppExit::Error(NonZeroU8::new(4).expect("nonzero"));
				finished = true;
			}
			return;
		}

		let output = job.output.clone();
		let obj_by_group: std::collections::HashMap<_, _> = groups
			.iter()
			.map(|g| (g.name.clone(), g.positions.clone()))
			.collect();
		let diagnose = build_diagnose_report(world, root, &obj_by_group);
		let diagnose_path = output.with_extension("diagnose.json");
		write_diagnose_json(&diagnose_path, &diagnose);

		let validation = build_validation_report(world, root, &groups);
		let validation_path = output.with_extension("validation.json");
		write_validation_json(&validation_path, &validation);
		let report = write_obj_bundle(&output, &groups);
		if !report.validation_ok {
			eprintln!("OBJ face/index validation failed");
			exit = AppExit::Error(NonZeroU8::new(5).expect("nonzero"));
			finished = true;
			return;
		}

		job.exported = true;
		if let Some(dir) = job.compare_dir.clone() {
			capture::init_gpu_capture(world, root, dir, output.clone());
			return;
		}
		println!("Wrote {}", output.display());
		println!("Wrote {}", validation_path.display());
		println!("Wrote {}", diagnose_path.display());
		if let Some(summary) = &diagnose.divergence_summary {
			eprintln!("diagnose: {}", summary);
		}
		println!(
			"groups={} vertices={} faces={} uvs={} normals={} bbox_min=({:.3}, {:.3}, {:.3}) bbox_max=({:.3}, {:.3}, {:.3}) obj_bytes={} mtl_bytes={}",
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
			report.mtl_bytes,
		);
		if job.compare_dir.is_none() {
			finished = true;
		}
	});

	if finished {
		world.write_message(exit);
	}
}
