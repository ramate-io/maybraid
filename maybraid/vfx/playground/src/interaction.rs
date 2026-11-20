//! Playground-only capsule input/visuals. The VFX receives timestamped analytic snapshots.
use bevy::{math::DVec3, prelude::*};
use clap::Args;
use game_commands::command::TextEntryFocus;
use maybraid_vfx::parts::combined::{
	interaction::{InteractionScene, Scenario, Shape},
	CombinedState,
};
#[derive(Clone, Args)]
pub struct InteractionCommand {
	/// Large-body axial carry:1 reproduces the approved baseline.
	#[arg(long)]
	pub clump_carry: Option<f64>,
	#[arg(long)]
	pub contact_response: Option<f64>,
	#[arg(long)]
	pub deformation: Option<f32>,
	#[arg(long)]
	pub flow_response: Option<f32>,
	#[arg(long,value_parser=["manual","static","cross","stop","fast","none","grazing"])]
	pub scenario: Option<String>,
	#[arg(long,value_parser=["on","off"])]
	pub enabled: Option<String>,
	#[arg(long,value_parser=["on","off"])]
	pub wake: Option<String>,
	#[arg(long, num_args = 3, allow_hyphen_values = true)]
	pub wind: Option<Vec<f64>>,
	#[arg(long)]
	pub strength: Option<f64>,
	#[arg(long,value_parser=["brief","continuous","off"])]
	pub emission: Option<String>,
	#[arg(long, num_args = 3, allow_hyphen_values = true)]
	pub place: Option<Vec<f64>>,
	#[arg(long)]
	pub proxy_reset: bool,
}
impl InteractionCommand {
	pub fn apply(self, world: &mut World) {
		let t = world
			.query::<&CombinedState>()
			.iter(world)
			.find(|s| s.options.motion.interactive())
			.map(|s| s.simulation.time())
			.unwrap_or_else(|| {
				world
					.query::<&maybraid_vfx::parts::cartoon_flow::CartoonFlow>()
					.iter(world)
					.next()
					.map_or(0., |s| s.tick as f64 / 120.)
			});
		if self.emission.as_deref() == Some("off") {
			let mut query = world.query::<&mut CombinedState>();
			for mut s in query.iter_mut(world) {
				let time = s.simulation.time();
				if let Some(i) = &mut s.simulation.interactive {
					i.stop_at = Some(time);
				}
			}
		}
		let mut scene = world.resource_mut::<InteractionScene>();
		if let Some(v) = self.contact_response.filter(|v| v.is_finite()) {
			scene.contact_response = v.clamp(0., 2.);
		}
		if let Some(v) = self.clump_carry.filter(|v| v.is_finite()) {
			scene.clump_carry = v.clamp(0., 1.);
		}
		if let Some(v) = self.deformation.filter(|v| v.is_finite()) {
			scene.flow_limit = v.clamp(0., 0.4);
		}
		if let Some(v) = self.flow_response.filter(|v| v.is_finite()) {
			scene.flow_response = v.clamp(0., 2.);
		}
		if let Some(s) = self.scenario {
			scene.scenario = match s.as_str() {
				"manual" => Scenario::Manual,
				"grazing" => Scenario::Grazing,
				"static" => Scenario::Static,
				"stop" => Scenario::Stop,
				"fast" => Scenario::Fast,
				"none" => Scenario::None,
				_ => Scenario::Cross,
			};
		}
		if let Some(s) = self.enabled {
			scene.enabled = s == "on";
		}
		if let Some(s) = self.wake {
			scene.wake = s == "on";
		}
		if let Some(w) = self.wind {
			if w.iter().all(|v| v.is_finite()) {
				scene.wind = DVec3::new(w[0], w[1], w[2]).clamp_length_max(2.);
			}
		}
		if let Some(s) = self.strength {
			if s.is_finite() {
				scene.strength = s.clamp(0., 2.);
			}
		}
		if self.proxy_reset {
			scene.scenario = Scenario::Manual;
			scene.reset_proxy(t);
		}
		if let Some(p) = self.place {
			if p.iter().all(|v| v.is_finite()) {
				scene.scenario = Scenario::Manual;
				scene.place(
					t,
					DVec3::new(p[0], p[1].max(0.68), p[2])
						.clamp(DVec3::splat(-8.), DVec3::splat(8.)),
					true,
				);
			}
		}
		scene.changed();
	}
}
#[derive(Component)]
pub struct Proxy(pub u32);
#[derive(Component)]
pub struct ReferenceObstacle;
pub fn setup(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let capsule = meshes.add(Capsule3d::new(0.28, 0.8));
	let cube = meshes.add(Cuboid::default());
	let player = materials.add(StandardMaterial {
		base_color: Color::srgb(0.15, 0.42, 0.62),
		perceptual_roughness: 0.65,
		..default()
	});
	let solid = materials.add(StandardMaterial {
		base_color: Color::srgb(0.34, 0.39, 0.42),
		perceptual_roughness: 0.9,
		..default()
	});
	for (id, mesh, material) in
		[(1, capsule, player), (2, cube.clone(), solid.clone()), (3, cube, solid)]
	{
		commands.spawn((
			Name::new(format!("dust-interactor-{id}")),
			Mesh3d(mesh),
			MeshMaterial3d(material),
			Transform::default(),
			Visibility::Hidden,
			Proxy(id),
		));
	}
}
pub fn manual_input(
	keys: Res<ButtonInput<KeyCode>>,
	focus: Res<TextEntryFocus>,
	time: Res<Time<Virtual>>,
	states: Query<(&CombinedState, &maybraid_vfx::VfxInstance)>,
	cartoon: Query<(
		&maybraid_vfx::parts::cartoon_flow::CartoonFlow,
		&maybraid_vfx::membership::VfxMemberOf,
	)>,
	instances: Query<&maybraid_vfx::VfxInstance>,
	mut scene: ResMut<InteractionScene>,
) {
	if focus.0 || time.is_paused() {
		return;
	}
	let selection = states
		.iter()
		.find(|(s, _)| s.options.motion.interactive())
		.map(|(s, i)| (s.simulation.time(), i.age, i.playback))
		.or_else(|| {
			cartoon
				.iter()
				.find(|(s, _)| {
					s.ready && s.contact && s.region.as_ref().is_none_or(|r| r.inputs.is_none())
				})
				.and_then(|(s, m)| {
					instances.get(m.root()).ok().map(|i| (s.tick as f64 / 120., i.age, i.playback))
				})
		});
	let Some((sim_time, age, playback)) = selection else {
		return;
	};
	let mut direction = DVec3::ZERO;
	for (key, axis) in [
		(KeyCode::ArrowRight, DVec3::X),
		(KeyCode::ArrowLeft, -DVec3::X),
		(KeyCode::ArrowUp, -DVec3::Z),
		(KeyCode::ArrowDown, DVec3::Z),
		(KeyCode::PageUp, DVec3::Y),
		(KeyCode::PageDown, -DVec3::Y),
	] {
		if keys.pressed(key) {
			direction += axis;
		}
	}
	if direction == DVec3::ZERO {
		return;
	}
	let t = age as f64;
	let from = scene.capsule(sim_time).position;
	if scene.scenario != Scenario::Manual {
		scene.scenario = Scenario::Manual;
		scene.place(sim_time, from, true);
	}
	let p = (from + direction.normalize() * time.delta_secs_f64() * playback as f64 * 1.5)
		.clamp(DVec3::new(-8., 0.68, -8.), DVec3::splat(8.));
	scene.place(t, p, false);
}
pub fn visuals(
	herds: Query<&maybraid_vfx::parts::herd::HerdState>,
	scene: Res<InteractionScene>,
	states: Query<&CombinedState>,
	cartoon: Query<&maybraid_vfx::parts::cartoon_flow::CartoonFlow>,
	mut proxies: Query<(&Proxy, &mut Transform, &mut Visibility), Without<ReferenceObstacle>>,
	mut reference: Query<&mut Visibility, (With<ReferenceObstacle>, Without<Proxy>)>,
) {
	let active = states.iter().find(|s| s.options.motion.interactive());
	let surface = cartoon
		.iter()
		.find(|s| s.ready && s.contact && s.region.as_ref().is_none_or(|r| r.inputs.is_none()));
	let present = active.is_some() || surface.is_some();
	for mut v in &mut reference {
		*v = if present
			|| cartoon.iter().any(|s| s.ready && s.region.is_some())
			|| !herds.is_empty()
		{
			Visibility::Hidden
		} else {
			Visibility::Visible
		};
	}
	let time = active
		.map(|s| s.simulation.time())
		.or_else(|| surface.map(|s| s.tick as f64 / 120.))
		.unwrap_or(0.);
	let local = surface.and_then(|s| s.region.as_ref().map(|r| r.scene(time)));
	let scene = local.as_ref().unwrap_or(&scene);
	let boxes = scene.boxes();
	let capsule = scene.capsule(time);
	for (id, mut transform, mut visible) in &mut proxies {
		*visible = if present { Visibility::Visible } else { Visibility::Hidden };
		let c = match id.0 {
			1 => capsule,
			2 => boxes[0],
			_ => boxes[1],
		};
		transform.translation = c.position.as_vec3();
		transform.rotation = c.rotation.as_quat();
		transform.scale = match c.shape {
			Shape::Box { half } => half.as_vec3() * 2.,
			_ => Vec3::ONE,
		};
	}
}
