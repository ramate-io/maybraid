//! In-game clap commands for the VFX playground.

use bevy::prelude::*;
use clap::Parser;
use game_commands::command::{CommandScript, GameCommand};
use maybraid_vfx::{canonicalize_effect_name, VfxDrive, GOD_BEAM, SONIC_WAVE};

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
	/// 7 m heavenly column, 10 s active. Width is diameter in meters.
	GodBeam {
		#[arg(long, default_value_t = 1.0)]
		scale: f32,
		#[arg(long, default_value_t = 1.0)]
		intensity: f32,
		#[arg(long, default_value_t = 0.0)]
		distance: f32,
		#[arg(long)]
		seed: Option<u64>,
		#[arg(long, default_value_t = 1.0)]
		playback: f32,
		/// Column diameter in meters. B04 convention is 7.
		#[arg(long, default_value_t = 7.0)]
		width: f32,
		#[arg(long, default_value_t = 96.0)]
		height: f32,
		#[arg(long, default_value_t = 10.0)]
		lifetime: f32,
		#[arg(long, default_value_t = 0.4)]
		fade_in: f32,
		#[arg(long, default_value_t = 0.55)]
		fade_out: f32,
	},
	/// Mouth-emitted vortex rings at constant size. The playground driver writes poses.
	SonicWave {
		#[arg(long, default_value_t = 1.0)]
		scale: f32,
		#[arg(long, default_value_t = 1.0)]
		intensity: f32,
		#[arg(long, default_value_t = 0.0)]
		distance: f32,
		#[arg(long)]
		seed: Option<u64>,
		#[arg(long, default_value_t = 1.0)]
		playback: f32,
		#[arg(long, default_value_t = 20.0)]
		range: f32,
		/// Fan cone in radians. `0` is a single path.
		#[arg(long, default_value_t = 0.0)]
		spread: f32,
		#[arg(long, default_value_t = 22.0)]
		speed: f32,
		#[arg(long, default_value_t = 3)]
		count: u32,
		#[arg(long, default_value_t = 0.55)]
		radius: f32,
		#[arg(long, default_value_t = 0.14)]
		thickness: f32,
		#[arg(long, default_value_t = 0.28)]
		interval: f32,
	},
	/// Fade and clean up every live VFX instance.
	Cancel,
	/// Spawn the last effect at near, mid, and far distances.
	Spread,
	/// Orbit the camera around the burst origin.
	Orbit,
	/// Freeze playback once the instance reaches `age` seconds.
	Freeze {
		#[arg(allow_hyphen_values = true)]
		age: f32,
	},
	/// Resume playback after `/freeze`.
	Play,
	/// Stop automatically repeating the last show.
	Noloop,
	/// Toggle the bottom command drawer (same as F1). Hide it for captures.
	Hud,
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
	pub drive: VfxDrive,
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
					VfxShowRequest {
						effect: "fiery_explosion".into(),
						scale,
						intensity,
						distance,
						seed,
						playback,
						drive: VfxDrive::default(),
					},
				);
			}
			Self::Show { effect, scale, intensity, distance, seed, playback } => {
				let Some(name) = canonicalize_effect_name(&effect) else {
					*console = format!(
						"unknown effect `{effect}` — try fiery-explosion, god-beam, sonic-wave, flash, fireball, smoke, sparks"
					);
					return;
				};
				queue_show(
					commands,
					console,
					VfxShowRequest {
						effect: name.into(),
						scale,
						intensity,
						distance,
						seed,
						playback,
						drive: default_drive(name),
					},
				);
			}
			Self::GodBeam {
				scale,
				intensity,
				distance,
				seed,
				playback,
				width,
				height,
				lifetime,
				fade_in,
				fade_out,
			} => {
				queue_show(
					commands,
					console,
					VfxShowRequest {
						effect: GOD_BEAM.into(),
						scale,
						intensity,
						distance,
						seed,
						playback,
						drive: VfxDrive::god_beam()
							.with_size(width)
							.with_reach(height)
							.with_lifetime(lifetime)
							.with_fade_in(fade_in)
							.with_fade_out(fade_out),
					},
				);
			}
			Self::SonicWave {
				scale,
				intensity,
				distance,
				seed,
				playback,
				range,
				spread,
				speed,
				count,
				radius,
				thickness,
				interval,
			} => {
				queue_show(
					commands,
					console,
					VfxShowRequest {
						effect: SONIC_WAVE.into(),
						scale,
						intensity,
						distance,
						seed,
						playback,
						drive: VfxDrive::sonic_wave()
							.with_size(range)
							.with_reach(spread)
							.with_speed(speed)
							.with_count(count)
							.with_radius(radius)
							.with_thickness(thickness)
							.with_interval(interval),
					},
				);
			}
			Self::Cancel => {
				*console = "cancel live vfx".into();
				commands.queue(|world: &mut World| {
					let instances = {
						let mut query =
							world.query_filtered::<Entity, With<maybraid_vfx::VfxInstance>>();
						query.iter(world).collect::<Vec<_>>()
					};
					for entity in instances {
						maybraid_vfx::apply_cancel(world, entity);
					}
				});
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
			Self::Play => {
				commands.queue(|world: &mut World| {
					world.resource_mut::<PreviewFreeze>().age = None;
					world.resource_mut::<Time<Virtual>>().unpause();
				});
				*console = "play".into();
			}
			Self::Noloop => {
				commands.queue(|world: &mut World| {
					world.resource_mut::<LoopEnabled>().0 = false;
				});
				*console = "loop off".into();
			}
			Self::Hud => {
				commands.queue(|world: &mut World| {
					let mut visible =
						world.resource_mut::<game_commands::GameCommandDrawerVisible>();
					visible.0 = !visible.0;
				});
				*console = "drawer toggled".into();
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

fn default_drive(name: &str) -> VfxDrive {
	match name {
		GOD_BEAM => VfxDrive::god_beam(),
		SONIC_WAVE => VfxDrive::sonic_wave(),
		_ => VfxDrive::default(),
	}
}

fn queue_show(commands: &mut Commands, console: &mut String, request: VfxShowRequest) {
	let seed_label = request.seed.map(|s| s.to_string()).unwrap_or_else(|| "auto".into());
	*console = format!(
		"show {} --scale {} --intensity {} --distance {} --seed {seed_label} --playback {}",
		request.effect, request.scale, request.intensity, request.distance, request.playback
	);
	commands.queue(move |world: &mut World| {
		world.resource_mut::<LoopEnabled>().0 = true;
		world.resource_mut::<Time<Virtual>>().unpause();
		world.resource_mut::<PendingVfxShows>().0.push(request);
	});
}
