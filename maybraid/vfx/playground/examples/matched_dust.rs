//! Independent B/B+/C Houdini playback comparison; consumes `maybraid-vfx` dust APIs.
use bevy::{
	camera::visibility::NoFrustumCulling,
	core_pipeline::{prepass::DepthPrepass, tonemapping::Tonemapping},
	prelude::*,
	render::view::screenshot::{save_to_disk, Screenshot},
	window::{PresentMode, WindowResolution},
};
use clap::Parser;
use maybraid_vfx::dust::{
	DustBounds, DustBundle, DustDisturbance, DustExperiment, DustMultiView, DustPlayback,
	DustPlaybackMaterial, DustPlaybackPlugin, DustRepresentation,
};
use std::{path::PathBuf, time::Instant};

#[derive(Parser, Resource, Debug)]
struct Args {
	/// B = original fixed plane; B+ = eight-view baked ring; C = animated density.
	#[arg(default_value = "C", value_parser = ["B", "B+", "C"])]
	representation: String,
	/// Parent directory of wind_dust_demos and wind_dust_multiview.
	#[arg(long)]
	data_root: Option<PathBuf>,
	/// Accepted for the prebuilt Mac binary; the workspace example uses the embedded shader.
	#[arg(long, hide = true)]
	#[allow(dead_code)]
	shader_root: Option<PathBuf>,
	/// Start paused at an exact simulation timestamp (interactive inspection).
	#[arg(long)]
	at: Option<f32>,
	/// Save a single engine-rendered inspection image, then exit.
	#[arg(long)]
	still: Option<PathBuf>,
	#[arg(
		long,
		default_value = "medium",
		value_parser = ["medium", "distant", "orbit", "grazing", "entry", "inside", "moving", "stop", "elevated"]
	)]
	view: String,
	#[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(0..=10))]
	groups: u32,
	#[arg(long)]
	obstacles: bool,
	/// Diagnostic: keep obstacle masks active while hiding opaque wall/box geometry.
	#[arg(long, requires = "obstacles")]
	hide_obstacle_geometry: bool,
	#[arg(long)]
	interaction: bool,
	#[arg(long)]
	markers: bool,
	/// Fixed 60Hz simulation, 49 screenshots at the source's 15Hz; *not* real-time video.
	#[arg(long)]
	sequence: Option<PathBuf>,
	/// Real-time benchmark length; no fixed steps, screenshot work or visual warmup included.
	#[arg(long, default_value_t = 0.)]
	benchmark: f32,
	/// Bounded sampling budget: C 96/48 ray steps, B+ adjacent blend/nearest.
	#[arg(long, default_value = "default", value_parser = ["default", "lower"])]
	profile: String,
	#[arg(long, value_parser = clap::value_parser!(u32).range(24..=192))]
	steps: Option<u32>,
	#[arg(long, value_parser = ["blended", "nearest"])]
	angular: Option<String>,
	/// Freeze source density, while camera, influence and cleanup use advancing timeline.
	#[arg(long)]
	inspect_time: Option<f32>,
	#[arg(long, default_value = "default", value_parser = ["default", "sparse", "denser"])]
	density: String,
	#[arg(long, default_value = "dense", value_parser = ["dense", "spread"])]
	arrangement: String,
	/// Diagnostic: obstacle geometry still occludes, but density/plane exclusion is disabled.
	#[arg(long)]
	no_boundary_exclusion: bool,
	#[arg(long, default_value_t = 1)]
	repeats: u32,
	#[arg(long)]
	result: Option<PathBuf>,
}

#[derive(Component)]
struct DemoCamera;
#[derive(Component)]
struct DemoGroup;
#[derive(Component)]
struct Hud;
#[derive(Component)]
struct Influence;
#[derive(Resource)]
struct State {
	material: Handle<DustPlaybackMaterial>,
	started: Instant,
	ready: bool,
	snapshot: bool,
	tick: u32,
	elapsed: f32,
	age: f32,
	paused: bool,
	strength: f32,
	previous: Vec3,
	frames: Vec<f64>,
	focused_samples: usize,
	captures: Vec<serde_json::Value>,
	cycle: u32,
	resource_samples: Vec<serde_json::Value>,
	groups: Vec<Entity>,
	mesh: Handle<Mesh>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();
	if args.inspect_time.is_some_and(|t| !DustBounds::contains_time(t))
		|| args.at.is_some_and(|t| !DustBounds::contains_time(t))
		|| (args.at.is_some() && (args.sequence.is_some() || args.benchmark > 0.))
		|| !args.benchmark.is_finite()
		|| args.benchmark < 0.
		|| args.benchmark > 120.
		|| args.repeats == 0
		|| args.repeats > 10
		|| (args.sequence.is_some() && args.benchmark > 0.)
	{
		return Err("Invalid capture/benchmark/repeat settings".into());
	}
	if let Some(dir) = &args.sequence {
		std::fs::create_dir_all(dir)?;
	}
	let representation =
		DustRepresentation::from_label(&args.representation).ok_or("unsupported representation")?;
	let data_root = args.data_root.clone().unwrap_or_else(DustBundle::showcase_data_root);
	let bundle = DustBundle::read(data_root.join("wind_dust_demos"))?;
	let atlas = if representation == DustRepresentation::Plane {
		bundle.atlas()?
	} else {
		DustBundle::unused_atlas()
	};
	// B binds a tiny unused 3D texture; C binds a tiny unused 2D texture. Neither pays for the other's animation.
	let density = if representation == DustRepresentation::Density {
		bundle.density(args.markers)?
	} else {
		DustBundle::unused_density()
	};
	let multiview = if representation == DustRepresentation::MultiView {
		DustMultiView::read(&bundle.multiview_root(), &bundle)?
	} else {
		DustMultiView::fallback()
	};
	let playback = DustPlayback::new(
		representation,
		args.groups,
		args.steps(),
		bundle.capture_basis()?,
		DustExperiment {
			nearest: args.nearest(),
			density_multiplier: args.density_multiplier(),
			boundary_exclusion: args.obstacles && !args.no_boundary_exclusion,
		},
	);
	let mut app = App::new();
	app.add_plugins(DefaultPlugins.set(WindowPlugin {
		primary_window: Some(Window {
			title: format!("Maybraid Dust {} — matched Houdini playback", args.representation),
			resolution: WindowResolution::new(2560, 1440).with_scale_factor_override(2.),
			present_mode: PresentMode::AutoNoVsync,
			..default()
		}),
		..default()
	}));
	app.add_plugins(DustPlaybackPlugin);
	let atlas = app.world_mut().resource_mut::<Assets<Image>>().add(atlas);
	let density = app.world_mut().resource_mut::<Assets<Image>>().add(density);
	let multiview = app.world_mut().resource_mut::<Assets<Image>>().add(multiview);
	let material = app
		.world_mut()
		.resource_mut::<Assets<DustPlaybackMaterial>>()
		.add(DustPlaybackMaterial::new(playback, atlas, density, multiview));
	let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Rectangle::new(2., 2.));
	app.insert_resource(State {
		material,
		mesh,
		started: Instant::now(),
		ready: false,
		snapshot: false,
		tick: 0,
		elapsed: 0.,
		age: 0.,
		paused: false,
		strength: 0.,
		previous: Vec3::new(-2., 0.4, 0.1),
		frames: Vec::with_capacity(8000),
		focused_samples: 0,
		captures: Vec::new(),
		cycle: 0,
		resource_samples: Vec::new(),
		groups: Vec::new(),
	})
	.insert_resource(args)
	.insert_resource(ClearColor(Color::srgb(0.13, 0.16, 0.19)))
	.add_systems(Startup, setup)
	.add_systems(Update, update)
	.run();
	Ok(())
}

fn setup(
	mut commands: Commands,
	state: Res<State>,
	args: Res<Args>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.insert_resource(GlobalAmbientLight { brightness: 180., ..default() });
	commands.spawn((
		DirectionalLight { illuminance: 4000., shadow_maps_enabled: true, ..default() },
		Transform::from_xyz(5., 8., 4.).looking_at(Vec3::ZERO, Vec3::Y),
	));
	// Reference floor: geometry/lighting outside the animation, identical for B and C.
	let tile = meshes.add(Cuboid::new(1., 0.04, 1.));
	let shades = [
		materials.add(Color::srgb(0.21, 0.23, 0.24)),
		materials.add(Color::srgb(0.28, 0.30, 0.31)),
	];
	let floor_half = if args.arrangement == "spread" { 12i32 } else { 8i32 };
	for z in -floor_half..floor_half {
		for x in -floor_half..floor_half {
			commands.spawn((
				Mesh3d(tile.clone()),
				MeshMaterial3d(shades[((x + z).rem_euclid(2)) as usize].clone()),
				Transform::from_xyz(x as f32 + 0.5, -0.025, z as f32 + 0.5),
			));
		}
	}
	for (size, pos, color) in [
		(Vec3::new(3.2, 0.015, 0.025), Vec3::new(1.6, 0.005, 0.), Color::srgb(0.65, 0.1, 0.08)),
		(Vec3::new(0.025, 0.015, 3.2), Vec3::new(0., 0.005, 1.6), Color::srgb(0.07, 0.17, 0.7)),
	] {
		commands.spawn((
			Mesh3d(meshes.add(Cuboid::from_size(size))),
			MeshMaterial3d(materials.add(StandardMaterial {
				base_color: color,
				unlit: true,
				..default()
			})),
			Transform::from_translation(pos),
		));
	}
	if args.obstacles && !args.hide_obstacle_geometry {
		for obstacle in DustBounds::OBSTACLES {
			commands.spawn((
				Mesh3d(meshes.add(Cuboid::from_size(obstacle.size()))),
				MeshMaterial3d(materials.add(Color::srgb(0.16, 0.28, 0.34))),
				Transform::from_translation(obstacle.centre),
			));
		}
	}
	commands.spawn((
		Mesh3d(meshes.add(Sphere::new(0.16))),
		MeshMaterial3d(materials.add(StandardMaterial {
			base_color: Color::srgb(0.15, 0.7, 0.7),
			unlit: true,
			..default()
		})),
		Transform::from_xyz(-2., 0.4, 0.),
		Influence,
		if args.obstacles { Visibility::Visible } else { Visibility::Hidden },
	));
	commands.spawn((
		Camera3d::default(),
		Msaa::Off,
		DepthPrepass,
		Tonemapping::None,
		Transform::from_xyz(6., 3.8, 7.2).looking_at(Vec3::new(0., 0.55, 0.), Vec3::Y),
		DemoCamera,
	));
	commands.spawn((
		Mesh3d(state.mesh.clone()),
		MeshMaterial3d(state.material.clone()),
		NoFrustumCulling,
		Transform::default(),
	));
	commands.spawn((
		Text::new("Preparing matched dust playback…"),
		TextFont { font_size: 16.0.into(), ..default() },
		TextColor(Color::srgb(0.92, 0.90, 0.8)),
		Node {
			position_type: PositionType::Absolute,
			top: Val::Px(12.),
			left: Val::Px(16.),
			..default()
		},
		Hud,
	));
}

impl State {
	fn restart(&mut self, commands: &mut Commands, args: &Args) {
		for entity in self.groups.drain(..) {
			commands.entity(entity).try_despawn();
		}
		self.age = args.at.unwrap_or(0.);
		self.paused = args.at.is_some();
		self.strength = 0.;
		self.previous = Vec3::new(-2., 0.4, 0.1);
		for _ in 0..args.groups {
			self.groups.push(commands.spawn(DemoGroup).id());
		}
	}

	fn camera(&self, args: &Args) -> Transform {
		args.camera(self.age)
	}
}

impl Args {
	fn camera(&self, t: f32) -> Transform {
		let args = self;
		let view = args.view.as_str();
		let target = DustBounds::QUAD_CENTRE;
		let mut pos = match view {
			"distant" => Vec3::new(12., 6.5, 14.4),
			"grazing" => Vec3::new(6., 0.22, 7.2),
			"elevated" => Vec3::new(3.5, 7., 4.2),
			"orbit" => {
				// The fixed-step clock can finish a few microseconds short of 3.2s.
				// Use the literal start pose at the endpoint, avoiding floor-edge raster differences.
				let phase = (t / DustBounds::DURATION).clamp(0., 1.);
				let a = if phase >= 1. - 0.00001 { 0. } else { std::f32::consts::TAU * phase };
				Vec3::new(
					a.sin() * 3.,
					0.55 + 3. * DustMultiView::ELEVATION_DEGREES.to_radians().tan(),
					a.cos() * 3.,
				)
			}
			"inside" => Vec3::new(0.05, 0.45, 0.08),
			"entry" => {
				let r = (1. - t / 2.4).max(1. / 150.);
				Vec3::new(6. * r, 0.45, 7.2 * r)
			}
			_ => Vec3::new(6., 3.8, 7.2),
		};
		if args.arrangement == "spread" && args.groups > 1 {
			// A separate readable arrangement, not the dense-overlap stress budget.
			pos *= if view == "orbit" { 4.5 } else { 2.0 };
		}
		Transform::from_translation(pos).looking_at(
			if view == "inside" || view == "entry" { Vec3::new(-1., 0.45, -1.2) } else { target },
			Vec3::Y,
		)
	}
}

// Avoid per-frame disk I/O or asset construction. Only the uniform and HUD change during playback.
#[allow(clippy::too_many_arguments)]
fn update(
	mut commands: Commands,
	args: Res<Args>,
	mut state: ResMut<State>,
	time: Res<Time<Real>>,
	keys: Res<ButtonInput<KeyCode>>,
	mut materials: ResMut<Assets<DustPlaybackMaterial>>,
	mut camera: Query<&mut Transform, (With<DemoCamera>, Without<Influence>)>,
	mut influence: Query<&mut Transform, (With<Influence>, Without<DemoCamera>)>,
	mut hud: Query<&mut Text, With<Hud>>,
	groups: Query<Entity, With<DemoGroup>>,
	windows: Query<&Window>,
	images: Res<Assets<Image>>,
	mut exit: MessageWriter<AppExit>,
) {
	if !state.ready {
		if state.started.elapsed().as_secs_f32() < 3. {
			return;
		}
		state.ready = true;
		state.started = Instant::now();
		state.restart(&mut commands, &args);
	}
	if keys.just_pressed(KeyCode::Space) {
		state.paused = !state.paused;
	}
	if keys.just_pressed(KeyCode::KeyR) {
		state.restart(&mut commands, &args);
	}
	if state.paused {
		if keys.just_pressed(KeyCode::ArrowRight) {
			state.age = (state.age + 1. / DustBounds::FRAME_RATE).min(DustBounds::DURATION);
		}
		if keys.just_pressed(KeyCode::ArrowLeft) {
			state.age = (state.age - 1. / DustBounds::FRAME_RATE).max(0.);
		}
	}
	let dt = if args.sequence.is_some() { 1. / 60. } else { time.delta_secs().min(0.1) };
	let capture_age = state.age;
	let source_age = args.inspect_time.unwrap_or(capture_age);
	if let Some(mut cam) = camera.iter_mut().next() {
		*cam = state.camera(&args);
	}
	let mut pos = Vec3::new(-2. + (capture_age * 4.).min(4.), 0.4, 0.1);
	if args.view == "stop" && capture_age > 0.65 {
		pos.x = 0.6;
	}
	let velocity = (pos - state.previous) / dt.max(0.0001);
	state.previous = pos;
	let speed = velocity.length().min(DustDisturbance::MAX_SPEED);
	let target = DustDisturbance::target_strength(speed, args.interaction);
	if !state.paused {
		state.strength = DustDisturbance::smooth_strength(state.strength, target, dt);
	}
	if let Some(mut t) = influence.iter_mut().next() {
		t.translation = pos;
	}
	if let Some(mut mat) = materials.get_mut(&state.material) {
		mat.playback.set_source_time(source_age);
		mat.playback.set_active(capture_age <= DustBounds::DURATION);
		if !state.paused {
			mat.playback.set_disturbance(DustDisturbance {
				position: pos,
				velocity,
				strength: state.strength,
				radius: DustDisturbance::RADIUS,
			});
		}
		for g in 0..args.groups as usize {
			mat.playback.set_group_origin(g, args.origin(g as u32));
		}
	}
	if let Some(mut text) = hud.iter_mut().next() {
		**text = format!(
			"{} | {} | {} groups ({}) | timeline {:.3}s | source {:.3}s | {}\nSpace pause · R restart · arrows step when paused | {} / {} | C {} steps | density {} | boundary exclusion {}",
			args.representation,
			args.view,
			args.groups,
			args.arrangement,
			capture_age,
			source_age,
			if state.paused { "PAUSED" } else { "PLAYING" },
			args.profile,
			if args.nearest() { "nearest" } else { "adjacent blend" },
			args.steps(),
			args.density,
			args.obstacles && !args.no_boundary_exclusion
		);
	}
	if let Some(path) = &args.still {
		if !state.snapshot && state.started.elapsed().as_secs_f32() > 0.5 {
			commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
			state.snapshot = true;
		}
		if state.started.elapsed().as_secs_f32() > 1.5 {
			exit.write(AppExit::Success);
		}
	}
	// Sample the active *timeline* interval on every cycle, even if source time is frozen.
	// This is real renderer wall-clock delta; captures cannot run alongside benchmark mode.
	if args.benchmark > 0. && capture_age > 0.1 && capture_age < 1.8 {
		state.frames.push(time.delta_secs_f64() * 1000.);
		if windows.iter().any(|window| window.focused) {
			state.focused_samples += 1;
		}
	}
	if let Some(dir) = &args.sequence {
		if state.tick.is_multiple_of(4) && state.tick <= 192 {
			let frame = state.tick / 4;
			commands
				.spawn(Screenshot::primary_window())
				.observe(save_to_disk(dir.join(format!("frame-{frame:04}.png"))));
			let camera_position = state.camera(&args).translation.to_array();
			let angular_views = args.angular_views(Vec3::from_array(camera_position));
			state.captures.push(serde_json::json!({
				"frame": frame,
				"timestamp": capture_age,
				"source_timestamp": source_age,
				"source_frames": DustBounds::frames(source_age).as_tuple(),
				"camera": camera_position,
				"angular_views": angular_views
			}));
		}
	}
	if !state.paused {
		state.age += dt;
		state.elapsed += dt;
		state.tick += 1;
	}
	if state.age > DustBounds::DURATION && !state.groups.is_empty() {
		for entity in state.groups.drain(..) {
			commands.entity(entity).try_despawn();
		}
	}
	let timed = args.benchmark > 0. || args.sequence.is_some();
	if state.age > 4.2
		&& (!timed
			|| args.benchmark > 0.
			|| (args.sequence.is_some() && state.cycle + 1 < args.repeats))
	{
		state.resource_samples.push(serde_json::json!({
			"groups_after_tail": groups.iter().count(),
			"materials": materials.len(),
			"images": images.len()
		}));
		state.cycle += 1;
		state.restart(&mut commands, &args);
	}
	let finished = if args.benchmark > 0. {
		state.elapsed >= args.benchmark
	} else {
		args.sequence.is_some() && state.age > 4.1 && state.cycle + 1 >= args.repeats
	};
	if finished {
		let mut sorted = state.frames.clone();
		sorted.sort_by(f64::total_cmp);
		let n = sorted.len();
		let window = windows.iter().next();
		let representation = DustRepresentation::from_label(&args.representation)
			.unwrap_or(DustRepresentation::Density);
		let result = serde_json::json!({
			"representation": args.representation,
			"view": args.view,
			"groups": args.groups,
			"interaction": args.interaction,
			"obstacles": args.obstacles,
			"hide_obstacle_geometry": args.hide_obstacle_geometry,
			"steps": args.steps(),
			"profile": args.profile,
			"angular": if args.nearest() { "nearest" } else { "blended" },
			"density": args.density,
			"density_multiplier": args.density_multiplier(),
			"arrangement": args.arrangement,
			"inspect_time": args.inspect_time,
			"boundary_exclusion": args.obstacles && !args.no_boundary_exclusion,
			"mode": if args.sequence.is_some() { "fixed-step encoded render sequence" } else { "real-time no captures" },
			"mean_ms": if n > 0 { sorted.iter().sum::<f64>() / n as f64 } else { 0. },
			"p95_ms": if n > 0 { sorted[(n * 95 / 100).min(n - 1)] } else { 0. },
			"samples": n,
			"focused_samples": state.focused_samples,
			"end_groups": groups.iter().count(),
			"angular_views_at_end": args.angular_views(state.camera(&args).translation),
			"shared_materials": materials.len(),
			"shared_images": images.len(),
			"sequence": state.captures,
			"resource_samples": state.resource_samples,
			"resolution": window.map(|w| (w.physical_width(), w.physical_height())),
			"completed_cycles": state.cycle + 1,
			"animation_GPU_bytes": representation.animation_gpu_bytes(),
			"playback_upload_bytes": 0
		});
		println!("MATCHED_DUST_RESULT {result}");
		if let Some(path) = &args.result {
			if let Err(error) =
				std::fs::write(path, serde_json::to_vec_pretty(&result).unwrap_or_default())
			{
				eprintln!("result write failed: {error}");
			}
		}
		exit.write(AppExit::Success);
	}
}

impl Args {
	fn angular_views(&self, camera: Vec3) -> Vec<serde_json::Value> {
		if self.representation != "B+" {
			return Vec::new();
		}
		(0..self.groups)
			.map(|g| {
				let delta = camera - self.origin(g) - DustBounds::QUAD_CENTRE;
				let theta = delta.x.atan2(delta.z);
				let (i, j, w) = DustMultiView::views(theta, self.nearest());
				serde_json::json!({
					"group": g,
					"azimuth_degrees": theta.to_degrees(),
					"elevation_degrees": delta.y.atan2(Vec2::new(delta.x, delta.z).length()).to_degrees(),
					"views": [i, j],
					"blend": w * w * (3. - 2. * w)
				})
			})
			.collect()
	}

	fn steps(&self) -> u32 {
		self.steps.unwrap_or(if self.profile == "lower" { 48 } else { 96 })
	}

	fn nearest(&self) -> bool {
		self.angular.as_deref().map_or(self.profile == "lower", |a| a == "nearest")
	}

	fn density_multiplier(&self) -> f32 {
		match self.density.as_str() {
			"sparse" => 0.65,
			"denser" => 1.35,
			_ => 1.,
		}
	}

	fn origin(&self, g: u32) -> Vec3 {
		if self.groups <= 1 {
			Vec3::ZERO
		} else if self.arrangement == "spread" {
			let columns = self.groups.min(4);
			let rows = self.groups.div_ceil(columns);
			let x = g % columns;
			let z = g / columns;
			Vec3::new(
				(x as f32 - (columns - 1) as f32 * 0.5) * 4.5,
				0.,
				(z as f32 - (rows - 1) as f32 * 0.5) * 4.5,
			)
		} else {
			let a = g as f32 * 2.399963;
			Vec3::new(a.cos(), 0., a.sin()) * (0.28 * (g as f32).sqrt())
		}
	}
}

#[cfg(test)]
mod gallery_tests {
	use super::*;

	#[test]
	fn profiles_and_optical_intent_are_bounded() -> Result<(), Box<dyn std::error::Error>> {
		let default = Args::try_parse_from(["demo", "C"])?;
		assert_eq!(default.steps(), 96);
		assert!(!default.nearest());
		let lower =
			Args::try_parse_from(["demo", "B+", "--profile", "lower", "--density", "sparse"])?;
		assert!(lower.nearest());
		assert_eq!(lower.steps(), 48);
		assert_eq!(lower.density_multiplier(), 0.65);
		let override_args = Args::try_parse_from([
			"demo",
			"C",
			"--profile",
			"lower",
			"--steps",
			"72",
			"--angular",
			"blended",
		])?;
		assert_eq!(override_args.steps(), 72);
		assert!(!override_args.nearest());
		Ok(())
	}

	#[test]
	fn spread_offsets_are_separate_and_orbit_closes() -> Result<(), Box<dyn std::error::Error>> {
		let args =
			Args::try_parse_from(["demo", "C", "--groups", "10", "--arrangement", "spread"])?;
		let origins: Vec<_> = (0..10).map(|g| args.origin(g)).collect();
		let orbit = Args::try_parse_from(["demo", "B+", "--view", "orbit"])?;
		assert_eq!(orbit.camera(0.), orbit.camera(DustBounds::DURATION));
		assert_eq!(orbit.camera(0.), orbit.camera(3.1999977));
		assert!(orbit.camera(0.).translation.distance(orbit.camera(1.6).translation) > 5.9);
		for i in 0..10 {
			for j in 0..i {
				assert!(origins[i].distance(origins[j]) >= 4.5 - 0.0001);
			}
		}
		Ok(())
	}
}
