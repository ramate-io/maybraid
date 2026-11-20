//! In-game clap commands for the VFX playground.

use bevy::prelude::*;
use clap::Parser;
use game_commands::command::{CommandScript, GameCommand};
use maybraid_vfx::canonicalize_effect_name;

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
	/// Fixed world frames, isolated fields and timestamped actor snapshots.
	ContactRegions(crate::contact_regions::Config),
	/// Frozen CombinedExplosion diagnostic; saves composed R16 field and exact GPU payload.
	ExplosionReadback {
		path: std::path::PathBuf,
		#[arg(long, default_value_t = 0.)]
		after: f32,
	},
	/// Shared-region capsule herd, with distance-based kick-ups (not footsteps).
	#[command(name = "herd-dust")]
	Herd(crate::herd::HerdCommand),
	/// Live style change or emission gating; old dust retires independently.
	HerdControl {
		#[arg(value_parser=["cartoon","combined","none","off","on","overview","inside","cancel-first"])]
		action: String,
		#[arg(long, default_value_t = 0.)]
		after: f32,
	},
	/// Bounded GPU-readback sequence during normal playback (not a performance capture).
	Record {
		/// Offline dense acquisition at quarter host speed; encode by recorded host age.
		#[arg(long, requires = "replay")]
		sampled: bool,
		/// Warm up before acquisition; optionally restart the same seed then.
		#[arg(long, default_value_t = 0.)]
		after: f32,
		#[arg(long)]
		replay: bool,
		#[arg(long)]
		half_size: bool,
		path: std::path::PathBuf,
		#[arg(long, default_value_t = 5.)]
		seconds: f32,
	},
	FlowView {
		#[arg(value_parser=["normal","velocity","probes","solids","divergence","divergence-before","divergence-after"])]
		view: String,
	},
	Help,
	/// Stateful dust flow and proxy controls; emission mode applies on reset.
	Interaction(crate::interaction::InteractionCommand),
	DustAppearance(crate::dust_comparison::Appearance),
	/// GPU-readback screenshot, outside runtime assets.
	Screenshot {
		/// Hide the command drawer for an unobstructed evidence capture.
		#[arg(long)]
		clean: bool,
		#[arg(long)]
		geometry: bool,
		path: std::path::PathBuf,
		#[arg(long, default_value_t = 0.0)]
		after: f32,
	},
	/// Replay the last seed after pipeline warmup and freeze at the requested age.
	ReplayAt {
		age: f32,
		#[arg(long, default_value_t = 0.0)]
		after: f32,
	},
	/// Inspect Combined layers/scenario/quality without resetting particle history.
	Combined {
		#[arg(long,value_parser=["all","body","clouds","grains"])]
		layer: Option<String>,
		#[arg(long,value_parser=["normal","wind-turn"])]
		scenario: Option<String>,
		#[arg(long,value_parser=["default","low"])]
		quality: Option<String>,
	},
	/// Fixed inspection pose for ground effects.
	Inspect {
		#[arg(long, default_value_t = 0.)]
		after: f32,
		#[arg(value_parser=["main","side","top","close","boundary","inside","herd","explosion-front","explosion-three-quarter","explosion-close","explosion-inside","explosion-depth"])]
		view: String,
	},
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
		/// Authored smoke only: world heading in degrees; 0 maps local +X to world +X.
		#[arg(long)]
		heading: Option<f32>,
		#[arg(long,default_value="full",value_parser=["full","medium","low"])]
		lod: String,
		#[arg(long,default_value="high",value_parser=["high","medium","low"])]
		quality: String,
		#[arg(long,default_value="composed",value_parser=["composed","main","fine","spikes","section"])]
		diagnostic: String,
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
}

/// Queued `/show` requests. Applied once [`maybraid_vfx::VfxLibrary`] exists.
#[derive(Resource, Default)]
pub struct PendingVfxShows(pub Vec<VfxShowRequest>);

#[derive(Resource, Default)]
pub struct SpreadOnce(pub bool);

#[derive(Clone, Debug, Default)]
pub struct ExplosionPreview {
	pub heading: Option<f32>,
	pub lod: String,
	pub quality: String,
	pub diagnostic: String,
}

#[derive(Clone, Debug)]
pub struct VfxShowRequest {
	pub explosion: ExplosionPreview,
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
			Self::ContactRegions(c) => {
				commands.queue(move |w: &mut World| c.start(w));
				*console = "world contact regions".into();
			}
			Self::ExplosionReadback { path, after } => {
				commands.spawn(crate::explosion_preview::ReadbackTimer { path, remaining: after });
			}
			Self::Herd(config) => {
				commands.queue(move |world: &mut World| config.start(world));
				*console = "herd distance-emission prototype".into();
			}
			Self::HerdControl { action, after } => {
				commands
					.queue(move |world: &mut World| crate::herd::schedule(world, action, after));
			}
			Self::FlowView { view } => {
				let view = view.clone();
				commands.queue(move |world: &mut World| {
					world.resource_mut::<crate::flow_inspection::FlowInspection>().0 = view;
				});
			}
			Self::DustAppearance(a) => {
				commands.queue(move |w: &mut World| a.apply(w));
			}
			Self::Combined { layer, scenario, quality } => {
				commands.queue(move |world: &mut World| {
					use maybraid_vfx::{CombinedLayers, CombinedQuality, CombinedScenario};
					if !world.contains_resource::<maybraid_vfx::VfxLibrary>() {
						world
							.resource_mut::<PendingControls>()
							.0
							.push(PlaygroundCommand::Combined { layer, scenario, quality });
						return;
					}
					let mut library = world.resource_mut::<maybraid_vfx::VfxLibrary>();
					let mut options = library.combined.options;
					if let Some(layer) = layer {
						options.layers = match layer.as_str() {
							"body" => CombinedLayers::Body,
							"clouds" => CombinedLayers::Clouds,
							"grains" => CombinedLayers::Grains,
							_ => CombinedLayers::All,
						};
					}
					if let Some(scenario) = scenario {
						options.scenario = if scenario == "wind-turn" {
							CombinedScenario::WindTurn
						} else {
							CombinedScenario::Normal
						};
					}
					if let Some(quality) = quality {
						options.quality = if quality == "low" {
							CombinedQuality::Low
						} else {
							CombinedQuality::Default
						};
					}
					library.combined.options = options;
					library.combined_landing.layer.part.options.layers = options.layers;
					library.combined_landing.layer.part.options.quality = options.quality;
					library.combined_landing_contact.layer.part.options.layers = options.layers;
					library.combined_landing_contact.layer.part.options.quality = options.quality;
					library.combined_wave_contact.layer.part.options.layers = options.layers;
					library.combined_wave_contact.layer.part.options.quality = options.quality;
					for mut o in world
						.query::<&mut maybraid_vfx::parts::combined_choreography::OpticalChoreography>(
						)
						.iter_mut(world)
					{
						o.part.options.layers = options.layers;
						o.part.options.quality = options.quality;
					}

					for mut state in world
						.query::<&mut maybraid_vfx::parts::combined::CombinedState>()
						.iter_mut(world)
					{
						let motion = state.options.motion;
						state.configure(maybraid_vfx::CombinedOptions { motion, ..options });
					}
				});
				*console =
					"Combined inspection settings applied; active particle history retained".into();
			}
			Self::Interaction(control) => {
				let c = control.clone();
				commands.queue(move |world: &mut World| {
					if world.contains_resource::<maybraid_vfx::VfxLibrary>() {
						c.apply(world);
					} else {
						world
							.resource_mut::<PendingControls>()
							.0
							.push(PlaygroundCommand::Interaction(c));
					}
				});
				*console =
					"Interactive settings updated; brief/continuous applies on next reset".into();
			}
			Self::Help => *console = Self::long_help_string(),
			Self::Script(script) => script.run(commands, console),
			Self::Record { path, seconds, half_size, after, replay, sampled } => {
				commands.spawn(crate::recording::PendingRecording {
					path,
					seconds,
					half_size,
					remaining: after.max(0.),
					replay,
					sampled,
				});
			}
			Self::Screenshot { path, after, geometry, clean } => {
				if clean {
					commands.queue(|w: &mut World| {
						if let Some(mut d) =
							w.get_resource_mut::<game_commands::GameCommandDrawerVisible>()
						{
							d.0 = false;
						}
					});
				}
				commands.spawn(PendingCapture { path, remaining: after.max(0.), geometry });
			}
			Self::ReplayAt { age, after } => {
				commands.spawn(PendingReplay { age, remaining: after.max(0.) });
			}
			Self::Inspect { view, after } => {
				let view = view.clone();
				commands.queue(move |world: &mut World| {
					if after > 0. {
						world.spawn(PendingInspectionTimer { view, remaining: after });
					} else {
						world.insert_resource(PendingInspection(view));
					}
				});
			}

			Self::FieryExplosion { scale, intensity, distance, seed, playback } => {
				commands.queue(crate::contact_regions::leave);
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
			Self::Show {
				effect,
				scale,
				intensity,
				distance,
				seed,
				playback,
				heading,
				lod,
				quality,
				diagnostic,
			} => {
				let Some(name) = canonicalize_effect_name(&effect) else {
					*console = format!(
						"unknown effect `{effect}` — try fiery-explosion, flash, fireball, smoke, sparks, dust, cartoon-landing, cartoon-wave, combined-dust"
					);
					return;
				};
				commands.queue(crate::contact_regions::leave);
				queue_show(commands, console, name, scale, intensity, distance, seed, playback);
				commands.queue(move |w: &mut World| {
					if let Some(r) = w.resource_mut::<PendingVfxShows>().0.last_mut() {
						r.explosion = ExplosionPreview { heading, lod, quality, diagnostic };
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
			Self::Pause => {
				commands.queue(|world: &mut World| world.resource_mut::<Time<Virtual>>().pause());
				*console = "paused".into();
			}
			Self::Reset | Self::Cancel => {
				let reset = matches!(self, Self::Reset);
				commands.queue(move |world: &mut World| {
					if crate::contact_regions::reset_or_cancel(world, reset) {
						return;
					}
					if crate::herd::reset_or_cancel(world, reset) {
						return;
					}
					let replays: Vec<_> =
						world.query_filtered::<Entity, With<PendingReplay>>().iter(world).collect();
					for entity in replays {
						world.despawn(entity);
					}
					let roots: Vec<_> = world
						.query_filtered::<Entity, With<maybraid_vfx::VfxInstance>>()
						.iter(world)
						.collect();
					for root in roots {
						world.despawn(root);
					}
					world.remove_resource::<crate::dust_comparison::Comparison>();
					world.resource_mut::<PendingVfxShows>().0.clear();
					world.resource_mut::<crate::show::RepeatingVfxShow>().request = None;
					world.resource_mut::<SpreadOnce>().0 = false;
					world.resource_mut::<PreviewFreeze>().age = None;
					world.resource_mut::<LoopEnabled>().0 = reset;
					if reset {
						let mut scene =
							world
								.resource_mut::<maybraid_vfx::parts::combined::interaction::InteractionScene>(
								);
						let p = scene.manual.back().map(|k| k.position);
						scene.manual.clear();
						if let Some(p) = p {
							scene.place(0., p, true);
						}
						scene.changed();
					}
					world.resource_mut::<Time<Virtual>>().unpause();
					if reset {
						if let Some(config) = world.resource::<crate::herd::LastHerd>().0.clone() {
							config.start(world);
							return;
						}
						if let Some(request) =
							world.resource::<crate::show::LastVfxShow>().request.clone()
						{
							world.resource_mut::<PendingVfxShows>().0.push(request);
						}
					}
				});
				*console = if reset { "reset" } else { "cancelled" }.into();
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
		world.resource_mut::<LoopEnabled>().0 = true;
		world.resource_mut::<Time<Virtual>>().unpause();
		world.resource_mut::<PendingVfxShows>().0.push(VfxShowRequest {
			effect: name,
			explosion: default(),
			scale,
			intensity,
			distance,
			seed,
			playback,
		});
	});
}

#[derive(Component)]
pub(crate) struct PendingCapture {
	geometry: bool,
	path: std::path::PathBuf,
	remaining: f32,
}
pub(crate) fn capture(
	recordings: Query<Entity, With<crate::recording::Recording>>,
	herds: Query<&maybraid_vfx::parts::herd::HerdState>,
	mut commands: Commands,
	time: Res<Time<Real>>,
	clock: Res<Time<Virtual>>,
	mut pending: Query<(Entity, &mut PendingCapture)>,
	instances: Query<&maybraid_vfx::VfxInstance>,
	combined: Query<(Entity, &maybraid_vfx::parts::combined::CombinedState)>,
	cartoon: Query<(Entity, &maybraid_vfx::parts::cartoon_flow::CartoonFlow)>,
	optical: Query<&maybraid_vfx::parts::combined_choreography::OpticalChoreography>,
	meshes: Res<Assets<Mesh>>,
	surfaces: Query<(&maybraid_vfx::parts::cartoon_flow::Surface, &Mesh3d)>,
	renderer: Option<Res<maybraid_vfx::parts::combined::render::CombinedRenderer>>,
	interaction: Res<maybraid_vfx::parts::combined::interaction::InteractionScene>,
) {
	for (entity, mut p) in &mut pending {
		p.remaining -= time.delta_secs();
		if !recordings.is_empty() {
			continue;
		}
		if p.remaining > 0. {
			continue;
		}
		let mut data = format!(
			"paused={}\n{:#?}",
			clock.is_paused(),
			instances.iter().cloned().collect::<Vec<_>>()
		);
		for (root, s) in &combined {
			data += &format!(
				"\nCombined {root:?}: options={:?}, tick={}, body_time={}, backlog={}, clouds={}, grains={}, rejected_births={:?}",
				s.options,
				s.simulation.tick,
				s.body_time,
				s.backlog,
				s.simulation.particles.iter().filter(|p| p.kind == 0 && p.age < p.life).count(),
				s.simulation.particles.iter().filter(|p| p.kind == 1 && p.age < p.life).count(),
				s.simulation.dropped
			);
		}
		for (root, s) in &combined {
			if let Some(i) = &s.simulation.interactive {
				data += &format!(
					"\nInteractive {root:?}: scenario={:?}, enabled={}, wake={}, wind={:?}, scene_version={}, stop_at={:?}, wakes={}, rejected_bodies={}, capsule={:?}",
					interaction.scenario,
					interaction.enabled,
					interaction.wake,
					interaction.wind,
					interaction.version,
					i.stop_at,
					i.wakes.len(),
					i.rejected_bodies,
					interaction.capsule(s.simulation.time())
				);
				if let Some(f) = &i.flow {
					data += &format!(
						"\nFlow: grid={:?}, h={}, steps={}, bytes={}, divergence_rms={} -> {}",
						maybraid_vfx::parts::combined::flow::N,
						maybraid_vfx::parts::combined::flow::H,
						f.steps,
						f.bytes(),
						f.before,
						f.after
					);
				}
				for p in i.puffs.iter().flatten() {
					if i.flow.is_some() {
						data += &format!(
							"\nAffine id={} matrix={:?} determinant={}",
							p.id,
							p.affine,
							p.affine.determinant()
						);
					}
					data += &format!(
						"\nPuff id={} P={:?} V={:?} age={} phase={} concentration={}",
						p.id, p.position, p.velocity, p.age, p.phase, p.concentration
					);
				}
			}
		}

		for (controller, c) in &cartoon {
			if let Ok(o) = optical.get(controller) {
				data+=&format!("\nChoreography: landing={}, contact={}, tick={}, bodies={}, clouds={}, grains={}, dropped={:?}, opaque_surfaces={}, body_cpu_ns={}, particle_cpu_ns={}, steps={} options={:?}",o.part.landing,o.part.contact,c.tick,c.bodies.iter().flatten().count(),o.particles.iter().filter(|p|p.kind==0).count(),o.particles.iter().filter(|p|p.kind==1).count(),o.dropped,surfaces.iter().filter(|(s,_)|s.controller==controller).count(),o.body_ns,o.particle_ns,o.steps,o.part.options);
			}

			if p.geometry {
				for (surface, handle) in &surfaces {
					if surface.controller != controller || !surface.has_surface() {
						continue;
					}
					let Some(b) = c.bodies[surface.index].as_ref() else {
						continue;
					};
					let Some(mesh) = meshes.get(&handle.0) else {
						continue;
					};
					let Some(points) =
						mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| a.as_float3())
					else {
						continue;
					};
					let indices = mesh
						.indices()
						.map(|i| i.iter().collect::<Vec<_>>())
						.unwrap_or_else(|| (0..points.len()).collect());
					let mut bytes = Vec::with_capacity(indices.len() * 12);
					for i in indices {
						let point = b
							.model()
							.transform_point3(b.pose.coefficients.map(Vec3::from_array(points[i])));
						for v in point.to_array() {
							bytes.extend_from_slice(&v.to_le_bytes());
						}
					}
					let name = format!(
						"{}_body-{}.tri",
						p.path.file_stem().unwrap_or_default().to_string_lossy(),
						surface.index
					);
					if let Err(e) = std::fs::write(p.path.with_file_name(name), bytes) {
						warn!("Geometry capture: {e}");
					}
				}
			}
			data += &format!(
				"\nCartoon landing={} tick={} bodies={} carry={} field_bytes={} steps={}",
				c.landing,
				c.tick,
				c.bodies.iter().flatten().count(),
				interaction.clump_carry,
				c.field.as_ref().map_or(0, |f| f.bytes()),
				c.field.as_ref().map_or(0, |f| f.steps)
			);
			for b in c.bodies.iter().flatten() {
				data += &format!(
					"\nBodyGeometry id={} variant={} model={:?} coefficients={:?}",
					b.id,
					b.pose.variant,
					b.model().to_cols_array(),
					b.pose.coefficients.0.to_array()
				);
				data += &format!(
					"\nBody id={} P={:?} V={:?} affine={:?} determinant={}",
					b.id,
					b.position,
					b.velocity,
					b.affine,
					b.affine.determinant()
				);
			}
		}

		for h in &herds {
			let (summary, geometry) = h.snapshot(&meshes, p.geometry);
			data += &summary;
			for (id, bytes) in geometry {
				let name = format!(
					"{}_herd-{id}.tri",
					p.path.file_stem().unwrap_or_default().to_string_lossy()
				);
				if let Err(e) = std::fs::write(p.path.with_file_name(name), bytes) {
					warn!("Herd geometry capture: {e}");
				}
			}
		}
		if let Some(r) = renderer.as_ref() {
			data += &format!(
				"\nOptical scene: primitives={}, grains={}, candidate_refs={}, largest_bin={}, payload_bytes={}, fixed_GPU_buffer_bytes={}, fixed_live_field_bytes={}",
				r.primitives,
				r.grain_count,
				r.payload.references,
				r.payload.max_bin,
				r.payload.values.len() * 16,
				maybraid_vfx::parts::combined::render::MAX_PAYLOAD_BYTES,
				maybraid_vfx::parts::combined::cache::BYTES
			);
		}
		if let Err(e) = std::fs::write(p.path.with_extension("txt"), data) {
			warn!("Capture metadata: {e}");
		}
		commands
			.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
			.observe(bevy::render::view::screenshot::save_to_disk(p.path.clone()));
		commands.entity(entity).try_despawn();
	}
}

#[derive(Component)]
pub(crate) struct PendingReplay {
	age: f32,
	remaining: f32,
}
pub(crate) fn replay(
	mut commands: Commands,
	time: Res<Time<Real>>,
	mut pending: Query<(Entity, &mut PendingReplay)>,
	mut views: Query<(Entity, &mut PendingInspectionTimer)>,
) {
	for (e, mut p) in &mut views {
		p.remaining -= time.delta_secs();
		if p.remaining <= 0. {
			commands.insert_resource(PendingInspection(p.view.clone()));
			commands.entity(e).try_despawn();
		}
	}
	for (e, mut p) in &mut pending {
		p.remaining -= time.delta_secs();
		if p.remaining <= 0. {
			let mut text = String::new();
			PlaygroundCommand::Reset.react(&mut commands, &mut text);
			PlaygroundCommand::Freeze { age: p.age }.react(&mut commands, &mut text);
			commands.entity(e).try_despawn();
		}
	}
}

#[derive(Resource)]
pub(crate) struct PendingInspection(pub(crate) String);
#[derive(Component)]
pub(crate) struct PendingInspectionTimer {
	view: String,
	remaining: f32,
}
pub(crate) fn inspect_pending(world: &mut World) {
	if world.query::<&crate::camera::CameraController>().iter(world).next().is_none() {
		return;
	}
	let Some(PendingInspection(view)) = world.remove_resource::<PendingInspection>() else {
		return;
	};
	world.resource_mut::<CameraOrbit>().enabled = false;

	if view.starts_with("explosion-") {
		crate::explosion_preview::view(world, &view);
		return;
	}
	if view == "herd" {
		crate::herd::overview(world);
		return;
	}
	if crate::contact_regions::inspect(world, &view) {
		return;
	}
	if crate::herd::inspect(world, &view) {
		return;
	}
	let wave = world
		.resource::<PendingVfxShows>()
		.0
		.last()
		.or(world.resource::<crate::show::LastVfxShow>().request.as_ref())
		.is_some_and(|r| {
			(r.effect == "cartoon-wave"
				|| r.effect == "cartoon-wave-contact"
				|| r.effect == "cartoon-landing-contact"
				|| r.effect == "cartoon-wave-interactive"
				|| r.effect == "cartoon-landing-interactive")
				|| matches!(
					r.effect.as_str(),
					"combined-wave" | "combined-wave-contact" | "combined-landing-contact"
				) || r.effect == "combined-dust"
				|| (r.effect == "interactive-dust" || r.effect == "flow-guided-dust")
		});
	let flow = world
		.resource::<PendingVfxShows>()
		.0
		.last()
		.or(world.resource::<crate::show::LastVfxShow>().request.as_ref())
		.is_some_and(|r| r.effect == "flow-guided-dust");
	let optical_center = if matches!(view.as_str(), "inside" | "boundary") {
		let scene = world
			.resource::<maybraid_vfx::parts::combined::interaction::InteractionScene>()
			.clone();
		world
			.query::<(
				&maybraid_vfx::parts::cartoon_flow::CartoonFlow,
				&maybraid_vfx::parts::combined_choreography::OpticalChoreography,
			)>()
			.iter(world)
			.flat_map(|(c, o)| {
				c.bodies.iter().flatten().map(move |b| (b, c.tick as f64 / 120., o.contact()))
			})
			.filter(|(b, t, contact)| {
				!*contact
					|| scene
						.colliders(*t)
						.into_iter()
						.take(scene.collider_count())
						.all(|c| c.query(b.position).distance > 0.04)
			})
			.max_by(|(a, _, _), (b, _, _)| a.scale.max_element().total_cmp(&b.scale.max_element()))
			.map(|(b, _, _)| {
				(
					b.model().transform_point3(b.pose.coefficients.map(Vec3::new(0.05, 0.66, 0.))),
					b.scale.max_element() as f32,
				)
			})
	} else {
		None
	};
	let eye = if let Some((center, scale)) = optical_center {
		center + if view == "boundary" { Vec3::Z * scale * 0.8 } else { Vec3::ZERO }
	} else if flow && view == "inside" {
		Vec3::new(0.2, 0.65, 0.1)
	} else if flow && view == "boundary" {
		Vec3::new(0.25, 0.65, -0.75)
	} else if wave {
		match view.as_str() {
			"side" => Vec3::new(0.6, 1.0, -4.1),
			"top" => Vec3::new(0.6, 4.3, 0.01),
			"close" => Vec3::new(0.8, 1.1, -2.7),
			"boundary" => Vec3::new(1.5, 0.65, -0.6),
			"inside" => Vec3::new(1.45, 0.65, 0.05),
			_ => Vec3::new(3.8, 2.4, 4.6),
		}
	} else {
		match view.as_str() {
			"side" => Vec3::new(3., 0.8, 0.),
			"top" => Vec3::new(0., 3., 0.01),
			"close" => Vec3::new(0.0, 0.55, 1.45),
			_ => Vec3::new(3.6, 2.8, 4.8),
		}
	};
	let t = Transform::from_translation(eye).looking_at(
		if optical_center.is_some() {
			eye + Vec3::new(0.15, 0.02, -1.)
		} else if wave {
			Vec3::new(0.6, 0.45, 0.)
		} else {
			Vec3::new(0., 0.15, 0.)
		},
		Vec3::Y,
	);
	let (yaw, pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
	for (mut transform, mut controller) in world
		.query::<(&mut Transform, &mut crate::camera::CameraController)>()
		.iter_mut(world)
	{
		*transform = t;
		controller.inspection = true;
		controller.yaw = yaw;
		controller.pitch = pitch;
	}
}

#[derive(Resource, Default)]
pub(crate) struct PendingControls(Vec<PlaygroundCommand>);
pub(crate) fn apply_pending_controls(world: &mut World) {
	if !world.contains_resource::<maybraid_vfx::VfxLibrary>() {
		return;
	}
	let pending = std::mem::take(&mut world.resource_mut::<PendingControls>().0);
	let mut status = String::new();
	for command in pending {
		command.react(&mut world.commands(), &mut status);
	}
	world.flush();
}
#[cfg(test)]
mod startup_tests {
	use crate::commands::*;
	#[test]
	fn configuration_waits_for_the_library() -> Result<(), String> {
		let mut world = World::new();
		world.init_resource::<PendingControls>();
		for line in ["interaction --emission continuous", "combined --layer clouds --quality low"] {
			let command = <PlaygroundCommand as GameCommand>::parse_line(line)?;
			command.react(&mut world.commands(), &mut String::new());
			world.flush();
		}
		assert_eq!(world.resource::<PendingControls>().0.len(), 2);
		apply_pending_controls(&mut world);
		assert_eq!(world.resource::<PendingControls>().0.len(), 2);
		Ok(())
	}
}
