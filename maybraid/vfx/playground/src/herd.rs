//! Deterministic capsule choreography only. Dust receives fixed-time external input.
mod profiling;
use bevy::{
	math::{DQuat, DVec3},
	prelude::*,
};
use clap::Args;
use maybraid_vfx::{
	composition::EffectRecipe,
	parts::{
		combined::interaction::{Interactor, Shape, DT},
		herd::{
			emission::{EmitterInput, Ground},
			Frame, HerdState, HerdTick, Inputs, Settings, Style,
		},
	},
	spawn_vfx, VfxInstance, VfxSpawn,
};
#[derive(Clone, Debug, Args)]
pub struct HerdCommand {
	#[arg(long,default_value="travel",value_parser=["travel","flat"])]
	pub course: String,
	#[arg(long,num_args=3,allow_hyphen_values=true,default_values=["20","4","-15"])]
	pub start: Vec<f64>,
	#[arg(long, default_value_t = 37.)]
	pub heading: f64,
	#[arg(long,default_value_t=1,value_parser=clap::value_parser!(u32).range(1..=2))]
	pub herds: u32,
	#[arg(long)]
	pub diagnostics: bool,
	#[arg(long, default_value_t = 0.)]
	pub teleport_at: f64,
	#[arg(long, default_value_t = 0.12)]
	pub settling_accel: f64,
	#[arg(long, default_value_t = 0.12)]
	pub settling_speed: f64,

	#[arg(long,default_value="cartoon",value_parser=["cartoon","combined","none"])]
	pub style: String,
	#[arg(long,default_value="trail",value_parser=["trail","cloud"])]
	pub mode: String,
	#[arg(long, default_value_t = 1.)]
	pub actor_scale: f64,
	#[arg(long)]
	pub mixed_heights: bool,
	#[arg(long)]
	pub compact: bool,
	#[arg(long, default_value_t = 1.)]
	pub coverage: f64,
	#[arg(long, default_value_t = 1.)]
	pub body_density: f32,
	#[arg(long, default_value_t = 1.)]
	pub fine_density: f32,
	#[arg(long,default_value_t=10,value_parser=clap::value_parser!(u32).range(1..=20))]
	pub count: u32,
	#[arg(long, default_value_t = 13)]
	pub seed: u64,
	#[arg(long,default_value="cross",value_parser=["straight","turn","stop","cross"])]
	pub path: String,
	#[arg(long, default_value_t = 0.58)]
	pub spacing: f64,
	#[arg(long, default_value_t = 0.18)]
	pub size: f64,
	#[arg(long, default_value_t = 0.8)]
	pub lifetime: f64,
	#[arg(long, default_value_t = 1.)]
	pub strength: f64,
	#[arg(long,default_value_t=32,value_parser=clap::value_parser!(u32).range(1..=32))]
	pub budget: u32,
	#[arg(long,default_value_t=64,value_parser=clap::value_parser!(u32).range(0..=64))]
	pub cloud_budget: u32,
	#[arg(long,default_value_t=64,value_parser=clap::value_parser!(u32).range(0..=64))]
	pub grain_budget: u32,
}
impl HerdCommand {
	fn style(&self) -> Style {
		match self.style.as_str() {
			"combined" => Style::Combined,
			"none" => Style::None,
			_ => Style::Cartoon,
		}
	}
	pub fn start(self, world: &mut World) {
		self.start_owned(world, false);
	}
	fn start_owned(self, world: &mut World, overlay: bool) {
		crate::explosion_preview::scene(world, false);
		if !self.actor_scale.is_finite()
			|| !(0.7..=1.15).contains(&self.actor_scale)
			|| !self.coverage.is_finite()
			|| !(0.5..=1.3).contains(&self.coverage)
		{
			warn!("actor scale must be .7–1.15 and coverage .5–1.3 for this fixed region");
			return;
		}
		let initial = world.resource::<LastHerd>().0.is_none();
		let server = world.resource::<AssetServer>();
		let recipe = maybraid_vfx::effects::herd::HerdDust::load(
			server,
			Settings {
				group: self.mode == "cloud",
				travelling: self.course == "travel",
				settling_accel: self.settling_accel,
				settling_speed: self.settling_speed,
				body_density: self.body_density,
				fine_density: self.fine_density,
				coverage: self.coverage,
				spacing: self.spacing,
				size: self.size,
				lifetime: self.lifetime,
				strength: self.strength,
				max_puffs: self.budget as usize,
				max_fine: self.cloud_budget as usize,
				max_grains: self.grain_budget as usize,
				style: self.style(),
			},
		);
		if let Err(e) = recipe.layer.part.settings.validate() {
			warn!("{e}");
			return;
		}
		crate::contact_regions::leave(world);
		// Scene replacement affects only the previous herd's owned root.
		let old = world
			.query_filtered::<Entity, With<HerdActors>>()
			.iter(world)
			.collect::<Vec<_>>();
		if !overlay {
			for e in old {
				world.despawn(e);
			}
		}
		world.resource_mut::<crate::show::RepeatingVfxShow>().request = None;
		world.resource_mut::<crate::show::LastVfxShow>().request = None;
		world.resource_mut::<crate::show::LastVfxShow>().label.clear();
		let definition = recipe.effect_definition();
		let mesh = world.resource_mut::<Assets<Mesh>>().add(Capsule3d::new(0.11, 0.38));
		let material = world.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
			base_color: Color::srgb(0.13, 0.27, 0.37),
			perceptual_roughness: 0.8,
			..default()
		});

		let mut queue = bevy::ecs::world::CommandQueue::default();
		let mut commands = Commands::new(&mut queue, world);
		let root =
			spawn_vfx(&mut commands, &definition, VfxSpawn { seed: Some(self.seed), ..default() });

		commands.entity(root).insert((
			Inputs::default(),
			HerdActors {
				config: self.clone(),
				submitted: 0,
				style: self.style(),
				emitting: true,
				follow: true,
			},
		));
		for i in 0..self.count {
			commands.spawn((
				ChildOf(root),
				Proxy { index: i },
				Mesh3d(mesh.clone()),
				MeshMaterial3d(material.clone()),
				Transform::from_xyz(0., -20., 0.),
			));
		}

		queue.apply(world);
		if self.course == "travel" {
			let center = self.travel_pose(0, 0.).0;
			let terrain = self.heightfield(center, 0);
			if let Ok(h) = terrain {
				world.entity_mut(root).insert(maybraid_vfx::parts::herd::terrain::HerdTerrain(
					std::sync::Arc::new(h),
				));
			}
		}
		if !overlay && self.herds > 1 {
			let mut second = self.clone();
			second.herds = 1;
			second.start[0] += 8.;
			second.seed = second.seed.wrapping_add(37);
			second.start_owned(world, true);
		}
		world.resource_mut::<LastHerd>().0 = Some(self);
		world.resource_mut::<Time<Virtual>>().unpause();
		world.resource_mut::<crate::commands::PreviewFreeze>().age = None;
		world.resource_mut::<crate::commands::CameraOrbit>().enabled = false;
		if initial {
			world.insert_resource(crate::commands::PendingInspection("herd".into()));
		}
	}
}
#[derive(Resource, Default)]
pub struct LastHerd(pub Option<HerdCommand>);
#[derive(Component)]
pub struct HerdActors {
	config: HerdCommand,
	submitted: u32,
	style: Style,
	emitting: bool,
	follow: bool,
}
#[derive(Component)]
struct Proxy {
	index: u32,
}
pub fn control(world: &mut World, action: String) {
	if action == "cancel-first" {
		if let Some(root) = world
			.query_filtered::<Entity, With<HerdActors>>()
			.iter(world)
			.min_by_key(|e| e.index_u32())
		{
			world.despawn(root);
		}
		return;
	}
	if action == "inside" {
		world.insert_resource(crate::commands::PendingInspection("inside".into()));
		return;
	}
	if action == "overview" {
		overview(world);
		return;
	}
	if action == "reset" {
		if let Some(c) = world.resource::<LastHerd>().0.clone() {
			c.start(world);
		}
		return;
	}
	for mut s in world.query::<&mut HerdActors>().iter_mut(world) {
		match action.as_str() {
			"cartoon" => s.style = Style::Cartoon,
			"combined" => s.style = Style::Combined,
			"none" => s.style = Style::None,
			"off" => s.emitting = false,
			"on" => s.emitting = true,
			_ => {}
		}
	}
}
pub(crate) fn overview(world: &mut World) {
	for mut a in world.query::<&mut HerdActors>().iter_mut(world) {
		a.follow = true;
	}
	let compact = world
		.resource::<LastHerd>()
		.0
		.as_ref()
		.is_some_and(|c| c.mode == "cloud" || c.compact);
	let eye = if compact { Vec3::new(3.5, 2.9, 4.) } else { Vec3::new(5., 4.6, 7.) };
	for (mut transform, mut controller) in world
		.query::<(&mut Transform, &mut crate::camera::CameraController)>()
		.iter_mut(world)
	{
		*transform = Transform::from_translation(eye).looking_at(Vec3::new(0.8, 0.15, 0.), Vec3::Y);
		let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
		controller.yaw = yaw;
		controller.pitch = pitch;
		controller.inspection = true;
	}
}
/// Compact-cloud inspection avoids placing the camera inside the moving actors.
pub(crate) fn inspect(world: &mut World, view: &str) -> bool {
	let compact = world
		.resource::<LastHerd>()
		.0
		.as_ref()
		.is_some_and(|c| c.mode == "cloud" || c.compact);
	let travelling = world.resource::<LastHerd>().0.as_ref().is_some_and(|c| c.course == "travel");
	if (!compact && !travelling) || world.query::<&HerdActors>().iter(world).next().is_none() {
		return false;
	}
	let interior = world
		.query::<&HerdState>()
		.iter(world)
		.find_map(|s| s.volume_inspection_point());
	let mut center = world
		.query::<(&HerdActors, &VfxInstance)>()
		.iter(world)
		.next()
		.filter(|(a, _)| a.config.course == "travel")
		.map(|(a, i)| a.config.travel_pose(0, i.age as f64).0.as_vec3())
		.unwrap_or(Vec3::ZERO);
	let mut side = Vec3::X;
	if travelling {
		center = world
			.query::<&HerdState>()
			.iter(world)
			.find_map(HerdState::inspection_center)
			.unwrap_or(center);
		let direction = world
			.query::<(&HerdActors, &VfxInstance)>()
			.iter(world)
			.next()
			.map(|(a, i)| a.config.travel_pose(0, i.age as f64).1.as_vec3())
			.unwrap_or(Vec3::Z);
		side = Vec3::Y.cross(direction).normalize_or(Vec3::X);
	}
	for mut a in world.query::<&mut HerdActors>().iter_mut(world) {
		a.follow = false;
	}
	let eye = match view {
		"inside" => interior.unwrap_or(if travelling { center } else { Vec3::new(0.85, 0.2, 0.) }),
		"close" => center + Vec3::new(2.5, 1.1, 2.6),
		"side" => {
			center + if travelling { side * 4. + Vec3::Y * 0.9 } else { Vec3::new(3., 0.8, 0.) }
		}
		"top" => center + Vec3::new(0.85, 4., 0.01),
		_ => {
			overview(world);
			return true;
		}
	};
	for (mut transform, mut controller) in world
		.query::<(&mut Transform, &mut crate::camera::CameraController)>()
		.iter_mut(world)
	{
		*transform = Transform::from_translation(eye).looking_at(
			if view == "inside" {
				eye + Vec3::Z
			} else if travelling {
				center
			} else {
				center + Vec3::new(0.85, 0.2, 0.)
			},
			Vec3::Y,
		);
		let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
		controller.yaw = yaw;
		controller.pitch = pitch;
		controller.inspection = true;
	}
	true
}
// Retained fixed-domain regression choreography.
impl HerdCommand {
	fn pose(&self, id: u32, t: f64) -> (DVec3, DVec3, bool) {
		let h = maybraid_vfx::parts::combined::simulation::mix32(
			self.seed as u32 ^ id.wrapping_mul(0x9e3779b9),
		);
		let f = (h & 0xffff) as f64 / 65536.;
		let speed = 0.36 + f * 0.10;
		// Integral of a repeating run/decelerate/hold/accelerate envelope.
		fn travel(t: f64) -> f64 {
			let cycles = (t / 12.).floor();
			let u = t.rem_euclid(12.);
			cycles * 8.
				+ if u < 4. {
					u
				} else if u < 6. {
					let x = u - 4.;
					4. + x - x * x / 4.
				} else if u < 8. {
					5.
				} else if u < 10. {
					let x = u - 8.;
					5. + x * x / 4.
				} else {
					6. + u - 10.
				}
		}
		let clock = if self.path == "stop" { travel(t) } else { t };
		let position = |clock: f64| {
			if self.mode == "cloud" || self.compact {
				let phase = clock * 0.45;
				let (centre, direction) = match self.path.as_str() {
					"straight" => {
						(DVec3::new(0.85, 0.3, (clock * 0.8).rem_euclid(3.) - 1.5), DVec3::Z)
					}
					"cross" => (
						DVec3::new(0.85 + 1.1 * phase.sin(), 0.3, 1.2 * (2. * phase).sin()),
						DVec3::new(1.1 * phase.cos(), 0., 2.4 * (2. * phase).cos())
							.normalize_or(DVec3::Z),
					),
					_ => (
						DVec3::new(0.85 + 1.1 * phase.sin(), 0.3, 1.2 * phase.cos()),
						DVec3::new(1.1 * phase.cos(), 0., -1.2 * phase.sin())
							.normalize_or(DVec3::Z),
					),
				};
				let side = DVec3::Y.cross(direction);
				let lateral = (id % 4) as f64 * 0.5 - 0.75 + (f - 0.5) * 0.14;
				let longitudinal =
					(id / 4) as f64 * 0.45 - (self.count.div_ceil(4) - 1) as f64 * 0.225
						+ (f - 0.5) * 0.12 + 0.06 * (phase + f * 6.).sin();
				return centre + side * lateral + direction * longitudinal;
			}

			let phase = clock * speed + f * 6.28 + id as f64 * 0.39;
			match self.path.as_str() {
				"straight" => DVec3::new(
					-0.65 + (id % 5) as f64 * 0.65,
					0.30,
					(clock * speed * 2.5 + f * 4.2).rem_euclid(4.2) - 2.1,
				),
				"cross" => DVec3::new(0.85 + 1.55 * phase.sin(), 0.30, 1.75 * (phase * 2.).sin()),
				_ => DVec3::new(0.85 + 1.55 * phase.sin(), 0.30, 1.75 * phase.cos()),
			}
		};
		let p = position(clock);
		let prev_clock =
			if self.path == "stop" { travel((t - DT).max(0.)) } else { (t - DT).max(0.) };
		let previous = position(prev_clock);
		let discontinuous = p.distance(previous) > 0.15;
		(p, if discontinuous { DVec3::ZERO } else { (p - previous) / DT }, discontinuous)
	}
}
pub fn install(app: &mut App) {
	profiling::install(app);
	app.add_systems(Update, delayed.before(feed));
	app.init_resource::<LastHerd>()
		.add_systems(
			Update,
			feed.before(HerdTick).after(maybraid_vfx::lifecycle::tick_vfx_instances),
		)
		.add_systems(Update, visuals.after(HerdTick))
		.add_systems(Update, travel_scene.after(HerdTick));
}
fn feed(
	mut roots: Query<(
		&VfxInstance,
		&mut HerdActors,
		&mut Inputs,
		Option<&mut maybraid_vfx::parts::herd::terrain::HerdTerrain>,
	)>,
) {
	for (instance, mut actors, mut inputs, mut terrain) in &mut roots {
		if !instance.armed {
			continue;
		}
		if actors.config.course == "travel" {
			let center = actors.config.travel_pose(0, instance.age as f64).0;
			if let Some(h) = terrain.as_deref_mut() {
				let origin =
					bevy::math::DVec2::new((center.x - 16.).floor(), (center.z - 16.).floor());
				if (h.0.origin - origin).abs().max_element() >= 1. {
					if let Ok(next) = actors.config.heightfield(center, h.0.version + 1) {
						h.0 = std::sync::Arc::new(next);
					}
				}
			}
		}

		let target = (instance.age as f64 / DT + 1e-5).floor() as u32;
		while actors.submitted < target && inputs.frames.len() < 512 {
			let time = (actors.submitted + 1) as f64 * DT;
			let mut emitters = Vec::with_capacity(20);
			let mut colliders = Vec::with_capacity(20);
			let mut heights = Vec::with_capacity(20);
			for id in 0..actors.config.count {
				let (mut position, mut velocity, discontinuous) =
					if actors.config.course == "travel" {
						actors.config.travel_pose(id, time)
					} else {
						actors.config.pose(id, time)
					};
				let scale = actors.config.actor_scale
					* if actors.config.mixed_heights { 0.72 + 0.14 * (id % 5) as f64 } else { 1. };
				let course =
					if actors.config.mode == "cloud" || actors.config.compact { 0.6 } else { 1. };
				let mut normal = DVec3::Y;
				if actors.config.course == "flat" {
					position.x = 0.85 + (position.x - 0.85) * course * actors.config.actor_scale;
					position.z *= course * actors.config.actor_scale;
					position.y = 0.30 * scale;
					velocity *= course * actors.config.actor_scale;
				} else if let Some(g) = terrain.as_ref().and_then(|h| h.0.sample(position)) {
					normal = g.normal;
					position = g.point + normal * (0.30 * scale);
				}

				if actors.config.course == "travel" && !discontinuous {
					let previous = actors.config.travel_pose(id, (time - DT).max(0.)).0;
					if let Some(g) = terrain.as_ref().and_then(|h| h.0.sample(previous)) {
						velocity = (position - (g.point + g.normal * (0.30 * scale))) / DT;
					}
				}
				let world = Transform::from_translation(position.as_vec3())
					.with_scale(Vec3::splat(scale as f32))
					.with_rotation(DQuat::from_rotation_arc(DVec3::Y, normal).as_quat())
					.to_matrix();
				heights.push((
					id,
					maybraid_vfx::parts::herd::group::BodyExtent::transformed_capsule(
						0.11, 0.19, world, normal, None,
					)
					.height,
				));
				emitters.push(EmitterInput {
					id,
					position: position - normal * (0.30 * scale),
					velocity,
					ground: Ground { normal, grounded: true },
					discontinuous: discontinuous || actors.submitted == 0,
				});
				colliders.push(Interactor {
					id,
					shape: Shape::Capsule { radius: 0.11 * scale, half_segment: 0.19 * scale },
					position,
					rotation: DQuat::from_rotation_arc(DVec3::Y, normal),
					linear: velocity,
					angular: DVec3::ZERO,
					influence: 0.42
						* if actors.config.mode == "cloud" || actors.config.compact {
							scale
						} else {
							1.
						},
					strength: 1.,
					entrainment: 0.65,
					slide: 0.12,
				});
			}

			let frame = Frame {
				heights,
				time,
				emitters,
				colliders,
				births: Vec::new(),
				style: actors.style,
				emitting: actors.emitting,
			};
			if inputs.push(frame).is_err() {
				break;
			}
			actors.submitted += 1;
		}
	}
}
fn visuals(
	roots: Query<(
		&HerdActors,
		&VfxInstance,
		Option<&maybraid_vfx::parts::herd::terrain::HerdTerrain>,
	)>,
	mut proxies: Query<(&ChildOf, &Proxy, &mut Transform)>,
	states: Query<(&HerdState, &maybraid_vfx::membership::VfxMemberOf)>,
) {
	for (parent, proxy, mut transform) in &mut proxies {
		let Ok((actors, instance, terrain)) = roots.get(parent.parent()) else {
			continue;
		};
		let time = states
			.iter()
			.find(|(_, m)| m.root() == parent.parent())
			.map_or(instance.age as f64, |(s, _)| s.time());
		let (position, _, _) = if actors.config.course == "travel" {
			actors.config.travel_pose(proxy.index, time)
		} else {
			actors.config.pose(proxy.index, time)
		};
		let scale = actors.config.actor_scale
			* if actors.config.mixed_heights { 0.72 + 0.14 * (proxy.index % 5) as f64 } else { 1. };
		let course = if actors.config.mode == "cloud" || actors.config.compact { 0.6 } else { 1. };
		let mut position = position;
		if actors.config.course == "flat" {
			position.x = 0.85 + (position.x - 0.85) * course * actors.config.actor_scale;
			position.z *= course * actors.config.actor_scale;
			position.y = 0.30 * scale;
		} else {
			let g = terrain
				.and_then(|h| h.0.sample(position))
				.unwrap_or_else(|| actors.config.ground(position.x, position.z));
			position = g.point + g.normal * (0.30 * scale);
			transform.rotation = DQuat::from_rotation_arc(DVec3::Y, g.normal).as_quat();
		}
		transform.translation = position.as_vec3();
		transform.scale = Vec3::splat(scale as f32);
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn choreography_stays_in_region() -> Result<(), String> {
		use game_commands::command::GameCommand;
		let crate::commands::PlaygroundCommand::Herd(config) =
			crate::commands::PlaygroundCommand::parse_line("herd-dust --count 20 --seed 13")?
		else {
			return Err("wrong command".into());
		};
		for path in ["straight", "turn", "stop", "cross"] {
			let mut c = config.clone();
			c.path = path.into();
			for tick in 0..2880 {
				for id in 0..20 {
					let (p, v, _) = c.pose(id, tick as f64 * DT);
					assert!(p.x >= -1. && p.x <= 3. && p.z.abs() < 2.2);
					assert!(v.is_finite() && v.length() < 8.);
				}
			}
		}
		Ok(())
	}
	#[test]
	fn compact_cloud_mixed_scales_remain_inside_shared_region() -> Result<(), String> {
		use game_commands::command::GameCommand;
		let crate::commands::PlaygroundCommand::Herd(mut c) =
			crate::commands::PlaygroundCommand::parse_line(
				"herd-dust --mode cloud --count 20 --mixed-heights",
			)?
		else {
			return Err("wrong command".into());
		};
		for path in ["straight", "turn", "stop", "cross"] {
			c.path = path.into();
			for scale in [0.7, 1., 1.15] {
				for tick in 0..2880 {
					for id in 0..20 {
						let (mut p, v, _) = c.pose(id, tick as f64 * DT);
						p.x = 0.85 + (p.x - 0.85) * 0.6 * scale;
						p.z *= 0.6 * scale;
						let r = (0.72 + 0.14 * (id % 5) as f64) * scale * 0.11;
						assert!(p.x - r > -2. && p.x + r < 4. && p.z.abs() + r < 3.);
						assert!(v.is_finite());
					}
				}
			}
		}
		Ok(())
	}
}

#[derive(Component)]
pub(crate) struct DelayedHerdControl {
	pub action: String,
	pub remaining: f32,
}
pub(crate) fn schedule(world: &mut World, action: String, after: f32) {
	if after <= 0. {
		control(world, action);
		return;
	}
	if let Some(root) = world
		.query_filtered::<Entity, With<HerdActors>>()
		.iter(world)
		.min_by_key(|e| e.index_u32())
	{
		world.spawn((ChildOf(root), DelayedHerdControl { action, remaining: after }));
	}
}
fn delayed(
	mut commands: Commands,
	time: Res<Time<Real>>,
	mut pending: Query<(Entity, &ChildOf, &mut DelayedHerdControl)>,
) {
	for (e, parent, mut p) in &mut pending {
		p.remaining -= time.delta_secs();
		if p.remaining <= 0. {
			let action = p.action.clone();
			let root = parent.parent();
			commands.queue(move |w: &mut World| {
				if w.get::<HerdActors>(root).is_some() {
					control(w, action);
				}
			});
			commands.entity(e).try_despawn();
		}
	}
}

/// The existing controls target this scene while it is selected. Other VFX roots
/// remain owned by their callers; the legacy all-instance behavior is unchanged.
pub(crate) fn reset_or_cancel(world: &mut World, reset: bool) -> bool {
	let Some(config) = world.resource::<LastHerd>().0.clone() else {
		return false;
	};
	let roots = world
		.query_filtered::<Entity, With<HerdActors>>()
		.iter(world)
		.collect::<Vec<_>>();
	for root in roots {
		world.despawn(root);
	}
	let pending = world
		.query_filtered::<Entity, With<crate::commands::PendingReplay>>()
		.iter(world)
		.collect::<Vec<_>>();
	for e in pending {
		world.despawn(e);
	}
	world.resource_mut::<crate::commands::PreviewFreeze>().age = None;
	world.resource_mut::<Time<Virtual>>().unpause();
	if let Some(mut status) = world.get_resource_mut::<game_commands::ui::GameCommandStatusText>() {
		status.0 = if reset {
			"herd restarting".into()
		} else {
			"herd cancelled · reset to replay".into()
		};
	}
	if reset {
		config.start(world);
	}
	true
}
#[cfg(test)]
mod ownership_tests {
	use super::*;
	use game_commands::command::GameCommand;
	#[test]
	fn herd_cancel_preserves_other_vfx_roots() -> Result<(), String> {
		let crate::commands::PlaygroundCommand::Herd(config) =
			crate::commands::PlaygroundCommand::parse_line("herd-dust")?
		else {
			return Err("command".into());
		};
		let mut world = World::new();
		world.insert_resource(LastHerd(Some(config.clone())));
		world.init_resource::<Time<Virtual>>();
		world.init_resource::<crate::commands::PreviewFreeze>();
		let other = world
			.spawn(VfxInstance {
				name: "unrelated".into(),
				age: 0.,
				duration: 10.,
				playback: 1.,
				seed: 7,
				armed: true,
			})
			.id();
		let root = world
			.spawn(HerdActors {
				config,
				submitted: 0,
				style: Style::Cartoon,
				emitting: true,
				follow: true,
			})
			.id();
		let child = world.spawn(ChildOf(root)).id();
		assert!(reset_or_cancel(&mut world, false));
		assert!(world.get_entity(root).is_err());
		assert!(world.get_entity(child).is_err());
		assert!(world.get::<VfxInstance>(other).is_some());
		Ok(())
	}
}

#[cfg(test)]
mod cloud_command_tests {
	use super::*;
	use game_commands::command::GameCommand;
	#[test]
	fn parses_cloud_and_baseline_comparisons() -> Result<(), String> {
		for command in [
			"herd-dust --mode cloud --mixed-heights --actor-scale 0.8 --count 20 --style combined",
			"herd-dust --mode trail --compact --count 10 --style cartoon",
		] {
			crate::commands::PlaygroundCommand::parse_line(command)?;
		}
		Ok(())
	}
}

impl HerdCommand {
	fn travel_pose(&self, id: u32, t: f64) -> (DVec3, DVec3, bool) {
		fn distance(t: f64) -> f64 {
			if t < 12. {
				t
			} else if t < 14. {
				let d = t - 12.;
				12. + d - 0.25 * d * d
			} else if t < 16. {
				13.
			} else {
				13. + t - 16.
			}
		}
		let pose = |time: f64| {
			let d = if self.path == "stop" || self.path == "cross" { distance(time) } else { time };
			let theta = ((d - 6.) / 4.).clamp(0., 1.5);
			let (x, z) = if self.path == "straight" || d <= 6. {
				(0., d)
			} else if d < 12. {
				(4. * (1. - theta.cos()), 6. + 4. * theta.sin())
			} else {
				(
					4. * (1. - 1.5f64.cos()) + (d - 12.) * 1.5f64.sin(),
					6. + 4. * 1.5f64.sin() + (d - 12.) * 1.5f64.cos(),
				)
			};
			let side = (id % 4) as f64 * 0.28 - 0.42;
			let back = (id / 4) as f64 * 0.32;
			let q = DQuat::from_rotation_y(self.heading.to_radians());
			let mut p = DVec3::new(self.start[0], self.start[1], self.start[2])
				+ q * DVec3::new(x + side, 0., z - back);
			if self.teleport_at > 0. && time >= self.teleport_at {
				p += q * DVec3::X * 10.;
			}
			p.y = self.ground(p.x, p.z).point.y;
			p
		};
		let p = pose(t);
		let old = pose((t - DT).max(0.));
		let jump = p.distance(old) > 0.2;
		(p, if jump { DVec3::ZERO } else { (p - old) / DT }, jump)
	}
	fn ground(&self, x: f64, z: f64) -> maybraid_vfx::parts::herd::terrain::GroundSample {
		let height = |x: f64, z: f64| {
			let p = DVec3::new(x - 20., 0., z + 15.);
			let u = ((p.z - 5.3) / 0.8).clamp(0., 1.);
			self.start[1] + 0.35 * (p.z * 0.4).sin() + p.x * 0.03 - 0.22 * u * u * (3. - 2. * u)
		};
		let e = 0.01;
		let dx = (height(x + e, z) - height(x - e, z)) / (2. * e);
		let dz = (height(x, z + e) - height(x, z - e)) / (2. * e);
		maybraid_vfx::parts::herd::terrain::GroundSample {
			point: DVec3::new(x, height(x, z), z),
			normal: DVec3::new(-dx, 1., -dz).normalize(),
		}
	}
	fn heightfield(
		&self,
		center: DVec3,
		version: u64,
	) -> Result<maybraid_vfx::parts::herd::terrain::Heightfield, &'static str> {
		let origin = bevy::math::DVec2::new((center.x - 16.).floor(), (center.z - 16.).floor());
		let h = (0..65 * 65)
			.map(|i| {
				self.ground(origin.x + (i % 65) as f64 * 0.5, origin.y + (i / 65) as f64 * 0.5)
					.point
					.y
			})
			.collect();
		maybraid_vfx::parts::herd::terrain::Heightfield::new(
			origin,
			0.5,
			[65, 65],
			version,
			h,
			vec![true; 65 * 65],
		)
	}
}
#[derive(Component)]
struct TravelGround {
	root: Entity,
	version: u64,
}
fn travel_scene(
	mut commands: Commands,
	roots: Query<(
		Entity,
		&HerdActors,
		&VfxInstance,
		&maybraid_vfx::parts::herd::terrain::HerdTerrain,
	)>,
	states: Query<&HerdState>,
	mut grounds: Query<(Entity, &mut TravelGround, &Mesh3d)>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut cameras: Query<(&mut Transform, &mut crate::camera::CameraController), With<Camera3d>>,
	mut gizmos: Gizmos,
) {
	for (root, actors, _instance, terrain) in &roots {
		if actors.config.course != "travel" {
			continue;
		}
		let existing = grounds
			.iter()
			.find(|(_, g, _)| g.root == root)
			.map(|(e, g, m)| (e, g.version, m.0.clone()));
		if existing.as_ref().is_none_or(|(_, v, _)| *v != terrain.0.version) {
			let h = &terrain.0;
			let mut positions = Vec::new();
			let mut normals = Vec::new();
			let mut indices = Vec::new();
			for z in 0..65 {
				for x in 0..65 {
					let p =
						DVec3::new(h.origin.x + x as f64 * 0.5, 0., h.origin.y + z as f64 * 0.5);
					let g = h.sample(p);
					positions.push([p.x as f32, g.map_or(0., |g| g.point.y as f32), p.z as f32]);
					normals.push(g.map_or(Vec3::Y, |g| g.normal.as_vec3()).to_array());
					if x < 64 && z < 64 {
						let i = (x + 65 * z) as u32;
						indices.extend([i, i + 65, i + 1, i + 1, i + 65, i + 66]);
					}
				}
			}
			let mut mesh = Mesh::new(
				bevy::mesh::PrimitiveTopology::TriangleList,
				bevy::asset::RenderAssetUsages::default(),
			);
			mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
			mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
			mesh.insert_indices(bevy::mesh::Indices::U32(indices));
			if let Some((e, _, handle)) = existing {
				let _ = meshes.insert(handle.id(), mesh);
				commands.entity(e).insert(TravelGround { root, version: h.version });
			} else {
				let handle = meshes.add(mesh);
				let mat = materials.add(StandardMaterial {
					base_color: Color::srgb(0.34, 0.28, 0.18),
					perceptual_roughness: 1.,
					..default()
				});
				commands.spawn((
					ChildOf(root),
					TravelGround { root, version: h.version },
					Mesh3d(handle),
					MeshMaterial3d(mat),
					Transform::IDENTITY,
				));
			}
		}
		if actors.config.diagnostics {
			for state in &states {
				let c = state.field.origin + DVec3::new(3., 1.5, 3.);
				gizmos.cube(
					Transform::from_translation(c.as_vec3()).with_scale(Vec3::new(6., 3., 6.)),
					Color::srgb(0.1, 0.8, 0.9),
				);
				for key in state.tiles.iter().flatten() {
					let center = DVec3::new(key[0] as f64, key[1] as f64, key[2] as f64) * 4.
						+ DVec3::splat(2.);
					gizmos.cube(
						Transform::from_translation(center.as_vec3()).with_scale(Vec3::splat(4.)),
						Color::srgb(0.9, 0.5, 0.1),
					);
				}
			}
		}
		// Explicit opt-in follow for travelling demonstrations; camera transforms only.
		if roots.iter().next().is_some_and(|(r, _, _, _)| r == root) {
			let centers = roots
				.iter()
				.map(|(_, a, i, _)| a.config.travel_pose(0, i.age as f64).0)
				.collect::<Vec<_>>();
			let center = centers.iter().copied().sum::<DVec3>() / centers.len() as f64;
			for (mut t, mut c) in &mut cameras {
				if c.inspection && actors.follow {
					let eye = center
						+ if centers.len() > 1 {
							DVec3::new(10., 7., 12.)
						} else {
							DVec3::new(4., 3., 5.)
						};
					*t = Transform::from_translation(eye.as_vec3())
						.looking_at((center + DVec3::Y * 0.3).as_vec3(), Vec3::Y);
					let (yaw, pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
					c.yaw = yaw;
					c.pitch = pitch;
				}
			}
		}
	}
	for (e, g, _) in &mut grounds {
		if roots.get(g.root).is_err() {
			commands.entity(e).try_despawn();
		}
	}
}

#[cfg(test)]
mod travelling_tests {
	use super::*;
	use game_commands::command::GameCommand;
	fn config() -> Result<HerdCommand, String> {
		let crate::commands::PlaygroundCommand::Herd(c)=crate::commands::PlaygroundCommand::parse_line("herd-dust --course travel --start 20 4 -15 --heading 37 --herds 2 --teleport-at 22")? else {return Err("command".into());};
		Ok(c)
	}
	#[test]
	fn travelling_course_is_world_placed_non_recycling_and_stops() -> Result<(), String> {
		let c = config()?;
		let p0 = c.travel_pose(0, 0.).0;
		let p30 = c.travel_pose(0, 30.).0;
		assert!(p30.distance(p0) > 15.);
		assert!(p0.y > 3.);
		assert!(c.travel_pose(0, 14.5).0.distance(c.travel_pose(0, 15.5).0) < 1e-12);
		assert!(c.travel_pose(0, 17.).1.length() > 0.8);
		assert!(c.travel_pose(0, 22.).2);
		let mut rotated = c.clone();
		rotated.heading += 90.;
		// Path offsets rotate about the arbitrary host start; ground remains world-owned.
		let offset = |p: DVec3| DVec3::new(p.x - 20., 0., p.z + 15.);
		assert!(
			(offset(rotated.travel_pose(0, 5.).0)
				- DQuat::from_rotation_y(std::f64::consts::FRAC_PI_2)
					* offset(c.travel_pose(0, 5.).0))
			.length() < 1e-10
		);
		Ok(())
	}
	#[test]
	fn independent_cancel_preserves_second_herd() -> Result<(), String> {
		let c = config()?;
		let mut w = World::new();
		let spawn = |w: &mut World| {
			w.spawn(HerdActors {
				config: c.clone(),
				submitted: 0,
				style: Style::Combined,
				emitting: true,
				follow: true,
			})
			.id()
		};
		let first = spawn(&mut w);
		let second = spawn(&mut w);
		let child = w.spawn(ChildOf(first)).id();
		control(&mut w, "cancel-first".into());
		assert!(w.get_entity(first).is_err());
		assert!(w.get_entity(child).is_err());
		assert!(w.get_entity(second).is_ok());
		Ok(())
	}
}
