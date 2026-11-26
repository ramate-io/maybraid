//! In-game clap commands for the VFX playground.

use bevy::prelude::*;
use clap::Parser;
use game_commands::command::{CommandScript, GameCommand};

pub const PLAYGROUND_CLI_NAME: &str = "vfx";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "vfx",
	version,
	about = "VFX playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Spawn the composed fiery explosion (flash + fireball + smoke + sparks).
	FieryExplosion {
		#[arg(long, default_value_t = 1.0)]
		scale: f32,
		#[arg(long, default_value_t = 1.0)]
		intensity: f32,
		/// World-space Z offset from the default burst origin, in meters.
		#[arg(long, default_value_t = 0.0)]
		distance: f32,
		#[arg(long)]
		seed: Option<u64>,
		/// 1.0 is the authored layer envelope. Higher plays the same ratios faster.
		#[arg(long, default_value_t = 1.0)]
		playback: f32,
	},
	/// Spawn a named definition. Repeat to overlay instances.
	Show {
		#[arg(default_value = "fiery-explosion")]
		effect: String,
		#[arg(long, default_value_t = 1.0)]
		scale: f32,
		#[arg(long, default_value_t = 1.0)]
		intensity: f32,
		#[arg(long, default_value_t = 0.0)]
		distance: f32,
		#[arg(long)]
		seed: Option<u64>,
		/// 1.0 is the authored layer envelope. Higher plays the same ratios faster.
		#[arg(long, default_value_t = 1.0)]
		playback: f32,
	},
	/// Spawn the last effect at near, mid, and far distances.
	Spread,
	/// Orbit the camera around the burst origin.
	Orbit,
	/// Freeze playback once the instance reaches `age` seconds.
	Freeze {
		#[arg(allow_hyphen_values = true)]
		age: f32,
	},
	/// Pause all current effect clocks.
	Pause,
	/// Cancel all instances and replay the last effect from frame zero.
	Reset,
	/// Cancel every instance and automatic replay.
	Cancel,
	/// Resume playback after `/freeze`.
	Play,
	/// Stop automatically repeating the last show.
	Noloop,
	/// Rebuild with only this zero-based authored layer (clears mute selections).
	Solo {
		layer: usize,
	},
	/// Rebuild without this zero-based authored layer.
	Mute {
		layer: usize,
	},
	/// Restore all layers excluded by solo.
	Unsolo,
	/// Restore every muted layer.
	Unmute,
	/// Advance a paused preview by seconds (default: 1/60 s); inspector shows flipbook frames.
	Step {
		#[arg(default_value = "0.016666667", value_parser = step_seconds)]
		seconds: f32,
	},
	/// Reload the last preview's manifests and images; reset to rebuild materials afterward.
	Reload,
}

/// Queued `/show` requests. Applied once [`maybraid_vfx::VfxLibrary`] exists.
#[derive(Resource, Default)]
pub struct PendingVfxShows(pub Vec<VfxShowRequest>);

#[derive(Resource, Default)]
pub struct SpreadOnce(pub bool);

#[derive(Clone, Debug)]
pub struct VfxShowRequest {
	pub effect: String,
	pub scale: f32,
	pub intensity: f32,
	pub distance: f32,
	pub seed: Option<u64>,
	pub playback: f32,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct PreviewFreeze {
	pub age: Option<f32>,
}

#[derive(Resource, Clone, Debug)]
pub struct LoopEnabled(pub bool);

impl Default for LoopEnabled {
	fn default() -> Self {
		Self(true)
	}
}

#[derive(Resource, Default, Debug)]
pub struct CameraOrbit {
	pub enabled: bool,
	pub yaw: f32,
	pub radius: f32,
	pub height: f32,
}

impl PlaygroundCommand {
	pub fn long_help_string() -> String {
		<Self as GameCommand>::long_help_string()
	}

	pub fn parse_startup_command() -> Result<Option<Self>, String> {
		<Self as GameCommand>::parse_startup_command()
	}

	pub fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Help => *console = Self::long_help_string(),
			Self::Script(script) => script.run(commands, console),
			Self::FieryExplosion { scale, intensity, distance, seed, playback } => {
				queue_show(
					commands,
					console,
					"fiery_explosion",
					scale,
					intensity,
					distance,
					seed,
					playback,
				);
			}
			Self::Show { effect, scale, intensity, distance, seed, playback } => {
				queue_show(commands, console, &effect, scale, intensity, distance, seed, playback);
			}
			Self::Spread => {
				*console = "spread near / mid / far".into();
				commands.queue(|world: &mut World| {
					world.resource_mut::<SpreadOnce>().0 = true;
				});
			}
			Self::Orbit => {
				commands.queue(|world: &mut World| {
					let mut orbit = world.resource_mut::<CameraOrbit>();
					orbit.enabled = !orbit.enabled;
					if orbit.radius < 1.0 {
						orbit.radius = 4.2;
						orbit.height = 1.7;
					}
				});
				*console = "orbit toggled".into();
			}
			Self::Freeze { age } => {
				let hold = if age < 0.0 { None } else { Some(age) };
				commands.queue(move |world: &mut World| {
					world.resource_mut::<PreviewFreeze>().age = hold;
					world.resource_mut::<Time<Virtual>>().unpause();
				});
				*console = match hold {
					Some(age) => format!("freeze at {age:.2}s"),
					None => "freeze off".into(),
				};
			}
			Self::Pause => {
				commands.queue(|world: &mut World| world.resource_mut::<Time<Virtual>>().pause());
				*console = "paused".into();
			}
			Self::Reset | Self::Cancel => {
				let reset = matches!(self, Self::Reset);
				commands.queue(move |world: &mut World| restart_preview(world, reset));
				*console = if reset { "reset" } else { "cancelled" }.into();
			}
			Self::Play => {
				commands.queue(|world: &mut World| {
					world.resource_mut::<PreviewFreeze>().age = None;
					world.resource_mut::<Time<Virtual>>().unpause();
				});
				*console = "play".into();
			}
			Self::Solo { .. } | Self::Mute { .. } | Self::Unsolo | Self::Unmute => {
				commands.queue(move |world: &mut World| {
					if let Self::Solo { layer } | Self::Mute { layer } = self {
						if layer >= world.resource::<crate::inspector::PreviewInspector>().layers.len() {
							world.resource_mut::<crate::show::LastVfxShow>().label = format!("no layer {layer}; show an effect and use an index from its inspector");
							return;
						}
					}
					let mut selection = world.resource_mut::<crate::preview::PreviewSelection>();
					match self {
						Self::Solo { layer } => { selection.solo = Some(layer); selection.muted.clear(); }
						Self::Mute { layer } => { selection.muted.insert(layer); }
						Self::Unsolo => selection.solo = None,
						Self::Unmute => selection.muted.clear(),
						_ => {}
					}
					restart_preview(world, true);
				});
				*console = "rebuild preview with selected layers".into();
			}
			Self::Step { seconds } => {
				commands.queue(move |world: &mut World| {
					world.resource_mut::<Time<Virtual>>().pause();
					world.resource_mut::<PreviewFreeze>().age = None;
					world.resource_mut::<crate::preview::PreviewStep>().0 =
						Some(std::time::Duration::from_secs_f32(seconds));
				});
				*console = format!("step {seconds:.6}s (paused)");
			}
			Self::Reload => {
				commands.queue(|world: &mut World| {
					let inspector = world.resource::<crate::inspector::PreviewInspector>();
					let server = world.resource::<AssetServer>();
					let assets = world.resource::<Assets<maybraid_vfx::PackagedFlipbook>>();
					for path in inspector.layers.iter().filter_map(|row| row.path.as_ref()) {
						if let Some(asset) = server
							.get_handle::<maybraid_vfx::PackagedFlipbook>(path.clone())
							.and_then(|handle| assets.get(&handle))
						{
							if let Some(image) = server.get_path(asset.image.id()) {
								server.reload(image);
							}
						}
						server.reload(path.clone());
					}
				});
				*console = "reload requested; reset after loading to rebuild the recipe and materials (Rust edits require recompilation)".into();
			}
			Self::Noloop => {
				commands.queue(|world: &mut World| {
					world.resource_mut::<LoopEnabled>().0 = false;
				});
				*console = "loop off".into();
			}
		}
	}
}

impl GameCommand for PlaygroundCommand {
	const CLI_NAME: &'static str = PLAYGROUND_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}

fn queue_show(
	commands: &mut Commands,
	console: &mut String,
	name: &str,
	scale: f32,
	intensity: f32,
	distance: f32,
	seed: Option<u64>,
	playback: f32,
) {
	let seed_label = seed.map(|s| s.to_string()).unwrap_or_else(|| "auto".into());
	*console = format!(
		"show {name} --scale {scale} --intensity {intensity} --distance {distance} --seed {seed_label} --playback {playback}"
	);
	let name = name.to_string();
	commands.queue(move |world: &mut World| {
		*world.resource_mut::<crate::preview::PreviewSelection>() = default();
		world.resource_mut::<PreviewFreeze>().age = None;
		world.resource_mut::<LoopEnabled>().0 = true;
		world.resource_mut::<Time<Virtual>>().unpause();
		world.resource_mut::<PendingVfxShows>().0.push(VfxShowRequest {
			effect: name,
			scale,
			intensity,
			distance,
			seed,
			playback,
		});
	});
}

fn step_seconds(value: &str) -> Result<f32, String> {
	let seconds: f32 = value.parse().map_err(|_| "expected seconds")?;
	if !seconds.is_finite() || seconds <= 0. || seconds > 1. {
		return Err("step must be greater than zero and at most one second".into());
	}
	Ok(seconds)
}

fn restart_preview(world: &mut World, reset: bool) {
	let roots: Vec<_> = world
		.query_filtered::<Entity, With<maybraid_vfx::VfxInstance>>()
		.iter(world)
		.collect();
	for root in roots {
		world.despawn(root);
	}
	world.resource_mut::<PendingVfxShows>().0.clear();
	world.resource_mut::<crate::show::RepeatingVfxShow>().request = None;
	world.resource_mut::<SpreadOnce>().0 = false;
	world.resource_mut::<PreviewFreeze>().age = None;
	world.resource_mut::<crate::preview::PreviewStep>().0 = None;
	world.resource_mut::<LoopEnabled>().0 = reset;
	world.resource_mut::<Time<Virtual>>().unpause();
	let mut inspector = world.resource_mut::<crate::inspector::PreviewInspector>();
	inspector.root = None;
	inspector.failure = None;
	for row in &mut inspector.layers {
		if row.runtime_index.is_some() {
			row.status = "cancelled".into();
		}
	}
	if reset {
		if let Some(request) = world.resource::<crate::show::LastVfxShow>().request.clone() {
			world.resource_mut::<PendingVfxShows>().0.push(request);
		}
	}
}
