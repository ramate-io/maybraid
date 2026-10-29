//! Deterministic front/side/three-quarter captures of assembled character clips.
//!
//! ```sh
//! export WGPU_BACKEND=vulkan
//! export VK_ICD_FILENAMES=/path/to/sysroot/usr/share/vulkan/icd.d/lvp_icd.json
//! export VK_DRIVER_FILES="$VK_ICD_FILENAMES"
//! export WINIT_UNIX_BACKEND=x11 WAYLAND_DISPLAY=
//! xvfb-run -a cargo run -p characters-playground --example animation_capture -- \
//!   --output-dir /workspace/animation-pilot/captures
//! ```
//!
//! Arbitrary registered clips need no source changes or rebuild:
//! ```sh
//! xvfb-run -a cargo run -p characters-playground --example animation_capture -- \
//!   --output-dir /workspace/animation-pilot/captures --species braidman \
//!   --clip victory-wave --clip hand-clap --side both --progress 0.42,0.54,0.66 \
//!   --view front --view three-quarter
//! ```

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use character_rigs::rigs::{humanoid_v0::HumanoidV0Rig, quadruped_v0::QuadrupedV0Rig};
use characters::{AnimBone, AnimMailbox, AnimProgress, CharacterRig};
use characters_playground::{CharactersPlaygroundPlugin, PendingStartupCommand, PlaygroundCommand};
use clap::Parser;
use game_commands::command::GameCommand;
use game_commands::ui::GameCommandDrawerVisible;

const PROGRESS: [f32; 9] = [0.0, 0.15, 0.30, 0.45, 0.60, 0.78, 0.90, 0.98, 1.0];
const CLAP_PROGRESS: [f32; 12] =
	[0.0, 0.15, 0.30, 0.42, 0.45, 0.54, 0.60, 0.66, 0.78, 0.90, 0.98, 1.0];
const REFERENCE_PROGRESS: [f32; 1] = [0.0];
const TIMEOUT: Duration = Duration::from_secs(40);
const POSE_SETTLE: Duration = Duration::from_secs(2);
const ASSEMBLY_SETTLE: Duration = Duration::from_secs(8);

#[derive(Debug, Parser)]
#[command(about = "Capture assembled animation clips at repeatable poses")]
struct CaptureArgs {
	/// Clip name; may be repeated. With --species, use arbitrary registered clips.
	#[arg(long = "clip")]
	clips: Vec<String>,
	/// Character species for arbitrary clips (e.g. braidman, epiphant).
	#[arg(long)]
	species: Option<String>,
	/// Lead side for clips that use a side. "both" captures left and right.
	#[arg(long, value_parser = ["left", "right", "both"])]
	side: Option<String>,
	/// Normalized sample in [0,1]; repeat or give comma-separated values.
	#[arg(long = "progress")]
	progress: Vec<String>,
	/// Limit capture to a camera view; may be repeated.
	#[arg(long = "view", value_parser = ["front", "side", "three-quarter"])]
	views: Vec<String>,
	/// Output directory (defaults to ./animation-captures).
	#[arg(long)]
	output_dir: Option<PathBuf>,
	/// Save live binding and bone transforms alongside each screenshot.
	#[arg(long)]
	dump_pose: bool,
}

#[derive(Clone, Copy)]
struct Subject {
	name: &'static str,
	species: &'static str,
	clip: &'static str,
	sides: &'static [&'static str],
	progress: &'static [f32],
}

struct CaptureSubject {
	name: String,
	species: String,
	clip: String,
	sides: Vec<String>,
	progress: Vec<f32>,
}

const SUBJECTS: [Subject; 5] = [
	Subject {
		name: "epiphant-run-reference",
		species: "epiphant",
		clip: "run",
		sides: &["right"],
		progress: &REFERENCE_PROGRESS,
	},
	Subject {
		name: "epiphant-still-reference",
		species: "epiphant",
		clip: "still",
		sides: &["right"],
		progress: &REFERENCE_PROGRESS,
	},
	Subject {
		name: "victory-wave",
		species: "braidman",
		clip: "victory-wave",
		sides: &["left", "right"],
		progress: &PROGRESS,
	},
	Subject {
		name: "hand-clap",
		species: "braidman",
		clip: "hand-clap",
		sides: &["right"],
		progress: &CLAP_PROGRESS,
	},
	Subject {
		name: "quadruped-run-stop",
		species: "epiphant",
		clip: "quadruped-run-stop",
		sides: &["right"],
		progress: &PROGRESS,
	},
];

#[derive(Clone)]
struct Shot {
	species: String,
	clip: String,
	side: String,
	view: String,
	progress: f32,
	file: PathBuf,
}

#[derive(Resource)]
struct CapturePlan {
	shots: Vec<Shot>,
	index: usize,
	started: Duration,
	configured: bool,
	configured_species: Option<String>,
	species_changed: bool,
	pose_ready_at: Option<Duration>,
	requested: bool,
	dump_pose: bool,
}

fn main() -> Result<(), String> {
	let args = CaptureArgs::parse();
	let output_dir = args.output_dir.clone().unwrap_or_else(|| PathBuf::from("animation-captures"));
	std::fs::create_dir_all(&output_dir).map_err(|error| error.to_string())?;
	let shots = build_shots(&args, &output_dir)?;
	if shots.is_empty() {
		return Err("no matching clips or views".into());
	}

	let mut app = App::new();
	app.add_plugins(
		DefaultPlugins
			.set(WindowPlugin {
				primary_window: Some(Window {
					title: "Character animation capture".into(),
					resolution: (1280, 720).into(),
					..default()
				}),
				..default()
			})
			.set(AssetPlugin {
				file_path: Path::new(env!("CARGO_MANIFEST_DIR"))
					.join("../../../../../assets")
					.to_string_lossy()
					.into(),
				..default()
			}),
	)
	.insert_resource(ClearColor(Color::srgb(0.82, 0.88, 0.92)))
	.insert_resource(PendingStartupCommand::<PlaygroundCommand>(None))
	.insert_resource(CapturePlan {
		shots,
		index: 0,
		started: Duration::ZERO,
		configured: false,
		configured_species: None,
		species_changed: false,
		pose_ready_at: None,
		requested: false,
		dump_pose: args.dump_pose,
	})
	.add_plugins(CharactersPlaygroundPlugin)
	.add_systems(Startup, hide_command_ui)
	.add_systems(Update, capture_sequence);
	app.run();
	Ok(())
}

fn build_shots(args: &CaptureArgs, output_dir: &Path) -> Result<Vec<Shot>, String> {
	let views: Vec<String> = if args.views.is_empty() {
		["front", "side", "three-quarter"].into_iter().map(str::to_string).collect()
	} else {
		args.views.clone()
	};
	let requested_progress = parse_progress(&args.progress)?;
	let mut subjects = Vec::new();
	if let Some(species) = &args.species {
		if args.clips.is_empty() {
			return Err("--species requires at least one --clip".into());
		}
		let sides = match args.side.as_deref().unwrap_or("right") {
			"left" => vec![String::from("left")],
			"right" => vec![String::from("right")],
			"both" => vec![String::from("left"), String::from("right")],
			_ => return Err("--side must be left, right, or both".into()),
		};
		let progress = requested_progress.unwrap_or_else(|| PROGRESS.to_vec());
		for clip in &args.clips {
			subjects.push(CaptureSubject {
				name: clip.clone(),
				species: species.clone(),
				clip: clip.clone(),
				sides: sides.clone(),
				progress: progress.clone(),
			});
		}
	} else {
		for subject in SUBJECTS.iter().filter(|subject| {
			args.clips.is_empty()
				|| args.clips.iter().any(|clip| clip == subject.name || clip == subject.clip)
		}) {
			let sides = match args.side.as_deref() {
				Some("left") => vec![String::from("left")],
				Some("right") => vec![String::from("right")],
				Some("both") => vec![String::from("left"), String::from("right")],
				_ => subject.sides.iter().map(|side| String::from(*side)).collect(),
			};
			subjects.push(CaptureSubject {
				name: subject.name.to_string(),
				species: subject.species.to_string(),
				clip: subject.clip.to_string(),
				sides,
				progress: requested_progress.clone().unwrap_or_else(|| subject.progress.to_vec()),
			});
		}
	}
	let mut shots = Vec::new();
	let mut destinations = HashSet::new();
	for subject in subjects {
		for side in subject.sides {
			for view in &views {
				for &progress in &subject.progress {
					let sample = format!("{progress:.2}").replace('.', "-");
					let file =
						output_dir.join(format!("{}-{side}-{view}-{sample}.png", subject.name));
					if !destinations.insert(file.clone()) {
						return Err(format!(
							"multiple capture samples map to {}; choose progress values with distinct two-decimal names",
							file.display()
						));
					}
					shots.push(Shot {
						species: subject.species.clone(),
						clip: subject.clip.clone(),
						side: side.clone(),
						view: view.clone(),
						progress,
						file,
					});
				}
			}
		}
	}
	for shot in &shots {
		if shot.file.exists() {
			std::fs::remove_file(&shot.file).map_err(|error| {
				format!("could not remove stale capture {}: {error}", shot.file.display())
			})?;
		}
	}
	Ok(shots)
}

fn parse_progress(values: &[String]) -> Result<Option<Vec<f32>>, String> {
	if values.is_empty() {
		return Ok(None);
	}
	let mut progress = Vec::new();
	for value in values {
		for sample in value.split(',').map(str::trim).filter(|sample| !sample.is_empty()) {
			let parsed = sample
				.parse::<f32>()
				.map_err(|error| format!("invalid --progress value {sample:?}: {error}"))?;
			if !parsed.is_finite() || !(0.0..=1.0).contains(&parsed) {
				return Err(format!("--progress must be finite and in [0,1], got {sample:?}"));
			}
			progress.push(parsed);
		}
	}
	if progress.is_empty() {
		return Err("--progress requires at least one sample".into());
	}
	Ok(Some(progress))
}

fn hide_command_ui(mut visible: ResMut<GameCommandDrawerVisible>) {
	visible.0 = false;
}

fn capture_sequence(
	mut commands: Commands,
	time: Res<Time>,
	mut plan: ResMut<CapturePlan>,
	meshes: Query<(), With<Mesh3d>>,
	rigs: Query<(), With<CharacterRig>>,
	posed: Query<&AnimMailbox>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
	progress: Query<&AnimProgress>,
	humanoids: Query<(Entity, &HumanoidV0Rig, &GlobalTransform)>,
	quadrupeds: Query<(Entity, &QuadrupedV0Rig, &GlobalTransform)>,
	bones: Query<(Entity, &AnimBone, &Transform, &GlobalTransform, Option<&ChildOf>)>,
) {
	if plan.index >= plan.shots.len() {
		exit.write(AppExit::Success);
		return;
	}
	let now = time.elapsed();
	if !plan.configured {
		plan.started = now;
		let shot = plan.shots[plan.index].clone();
		plan.species_changed = plan.configured_species.as_deref() != Some(shot.species.as_str());
		let assemble = if plan.species_changed {
			format!(
				"character assemble --species {} --animation {} --side {}",
				shot.species, shot.clip, shot.side
			)
		} else {
			format!("character animate {} --side {}", shot.clip, shot.side)
		};
		let commands_to_run = [
			assemble,
			format!("character camera {}", shot.view),
			format!("character playback --progress {} --pause --once", shot.progress),
		];
		for line in commands_to_run {
			match PlaygroundCommand::parse_line(&line) {
				Ok(command) => command.react(&mut commands, &mut String::new()),
				Err(error) => {
					error!("cannot parse capture command {line:?}: {error}");
					exit.write(AppExit::error());
					return;
				}
			}
		}
		plan.configured = true;
		plan.configured_species = Some(shot.species);
		return;
	}
	if now.saturating_sub(plan.started) > TIMEOUT {
		error!("timed out preparing capture {}", plan.shots[plan.index].file.display());
		exit.write(AppExit::error());
		return;
	}
	let expected_progress = plan.shots[plan.index].progress;
	let sample_ready = progress.iter().any(|sample| (sample.0 - expected_progress).abs() < 0.001);
	let rig_ready = !meshes.is_empty()
		&& !rigs.is_empty()
		&& posed.iter().any(AnimMailbox::settled)
		&& sample_ready;
	if !rig_ready {
		plan.pose_ready_at = None;
		return;
	}
	if !plan.requested {
		// Body rig readiness precedes separately assembled head/clothing meshes and pipelines.
		let settle = if plan.species_changed { ASSEMBLY_SETTLE } else { POSE_SETTLE };
		let ready_at = *plan.pose_ready_at.get_or_insert(now);
		if now.saturating_sub(ready_at) < settle {
			return;
		}
		let path = plan.shots[plan.index].file.clone();
		if plan.dump_pose {
			let mut dump = String::new();
			for (entity, rig, _) in &humanoids {
				let _ = writeln!(dump, "FOREARM_TERMINALS {entity:?}: {:?}", rig.forearm_endpoints);
			}
			let bindings = humanoids
				.iter()
				.map(|(entity, rig, global)| (entity, &rig.binding, &rig.pose, global))
				.chain(
					quadrupeds
						.iter()
						.map(|(entity, rig, global)| (entity, &rig.binding, &rig.pose, global)),
				);
			for (entity, binding, pose, global) in bindings {
				let _ = writeln!(dump, "BODY {entity:?}: {global:?}");
				for name in &binding.definition.names {
					let Some(id) = binding.definition.id(name) else {
						continue;
					};
					let _ = writeln!(
						dump,
						"BIND {name}: parent={:?} rest={:?} pose={:?} rest_point={:?} pose_point={:?}",
						binding.definition.parent(id),
						binding.effective_rest.get(id),
						pose.get(id),
						binding.definition.translation_in_character(&binding.effective_rest, id),
						binding.definition.translation_in_character(pose, id)
					);
				}
			}
			for (entity, bone, local, global, parent) in &bones {
				let _ = writeln!(
					dump,
					"LIVE {} {entity:?}: parent={parent:?} rest={:?} local={local:?} global={global:?} local_y_half_probe={:?}",
					bone.name,
					bone.rest,
					global.transform_point(Vec3::Y * 0.5)
				);
			}
			if let Err(error) = std::fs::write(path.with_extension("pose.txt"), dump) {
				error!("cannot save pose beside {}: {error}", path.display());
				exit.write(AppExit::error());
				return;
			}
		}
		commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
		plan.requested = true;
		return;
	}
	if plan.shots[plan.index].file.is_file() && capturing.is_empty() {
		info!("captured {}", plan.shots[plan.index].file.display());
		plan.index += 1;
		plan.configured = false;
		plan.pose_ready_at = None;
		plan.requested = false;
	}
}
