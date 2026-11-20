//! Placement/choreography host only. Runtime receives ordinary timestamped world inputs.
use bevy::prelude::*;
use maybraid_vfx::parts::{
	cartoon_flow::{
		region::{ContactFrame, ContactInputs, ContactRegionBudget, WorldContactSnapshot},
		CartoonFlow, CartoonFlowTick,
	},
	combined::interaction::InteractionScene,
};
use maybraid_vfx::{
	CartoonContact, CombinedLanding, CombinedWaveContact, Effect, VfxInstance, VfxSpawn,
};
#[derive(Clone, clap::Args)]
pub struct Config {
	#[arg(long,default_value="combined-wave-contact",value_parser=["cartoon-landing-contact","cartoon-wave-contact","combined-landing-contact","combined-wave-contact"])]
	pub effect: String,
	#[arg(long,num_args=3,allow_hyphen_values=true,default_values=["20","0","-15"])]
	pub position: Vec<f32>,
	#[arg(long, default_value_t = 37., allow_hyphen_values = true)]
	pub yaw: f32,
	#[arg(long,default_value_t=1,value_parser=clap::value_parser!(u32).range(1..=8))]
	pub count: u32,
	/// Region separation in local X; 1.5 overlaps, 12 separates caches.
	#[arg(long, default_value_t = 12.)]
	pub spacing: f32,
	#[arg(long, default_value_t = 2)]
	pub region_cap: usize,
	#[arg(long, default_value_t = 13)]
	pub seed: u64,
	#[arg(long, default_value_t = 1.)]
	pub playback: f32,
	#[arg(long, default_value_t = 0.)]
	pub cancel_first_at: f32,
	#[arg(long)]
	pub close: bool,
	#[arg(long)]
	pub side: bool,
	/// Keep unrelated effects alive to inspect coexistence.
	#[arg(long)]
	pub overlay: bool,
}
#[derive(Component)]
struct Actor {
	root: Entity,
	frame: ContactFrame,
	scene: InteractionScene,
	id: usize,
}
#[derive(Component)]
struct SceneItem;
#[derive(Component)]
struct Feed {
	frame: ContactFrame,
	scene: InteractionScene,
	submitted: i32,
	cancel_at: f32,
}
#[derive(Resource, Default)]
pub struct Last(pub Option<Config>);
impl Config {
	pub fn start(self, w: &mut World) {
		if self.position.iter().any(|v| !v.is_finite())
			|| !self.yaw.is_finite()
			|| !self.spacing.is_finite()
			|| !self.playback.is_finite()
		{
			warn!("Invalid contact scene placement");
			return;
		}
		if !self.overlay {
			clear(w);
			let roots = w.query_filtered::<Entity, With<VfxInstance>>().iter(w).collect::<Vec<_>>();
			for r in roots {
				w.despawn(r);
			}
		}
		w.resource_mut::<crate::commands::LoopEnabled>().0 = false;
		w.resource_mut::<crate::commands::CameraOrbit>().enabled = false;
		w.resource_mut::<crate::show::RepeatingVfxShow>().request = None;
		if !self.overlay {
			w.resource_mut::<crate::commands::PendingVfxShows>().0.clear();
		}
		w.resource_mut::<crate::commands::PreviewFreeze>().age = None;
		w.resource_mut::<Time<Virtual>>().unpause();
		w.resource_mut::<ContactRegionBudget>().maximum = self.region_cap;
		let base = Mat4::from_rotation_translation(
			Quat::from_rotation_y(self.yaw.to_radians()),
			Vec3::new(self.position[0], self.position[1], self.position[2]),
		);
		let landing = self.effect.contains("landing");
		let scene = InteractionScene {
			compact_landing: landing,
			cartoon_wave_timing: !landing,
			..default()
		};
		let material = w.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
			base_color: Color::srgb(0.15, 0.42, 0.62),
			perceptual_roughness: 0.7,
			..default()
		});
		let ground = w.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
			base_color: Color::srgb(0.47, 0.40, 0.30),
			perceptual_roughness: 1.,
			..default()
		});
		let capsule = w.resource_mut::<Assets<Mesh>>().add(Capsule3d::new(0.28, 0.8));
		let cube = w.resource_mut::<Assets<Mesh>>().add(Cuboid::default());
		let patch = w.resource_mut::<Assets<Mesh>>().add(Cuboid::new(7., 0.08, 7.));
		for i in 0..self.count {
			let matrix = base * Mat4::from_translation(Vec3::X * self.spacing * i as f32);
			let Ok(frame) = ContactFrame::new(matrix) else {
				continue;
			};
			let (s, r, p) = matrix.to_scale_rotation_translation();
			let spawn = VfxSpawn {
				transform: Transform { translation: p, rotation: r, scale: s },
				seed: Some(self.seed),
				playback: self.playback,
				..default()
			};
			let server = w.resource::<AssetServer>();
			let cartoon = if landing {
				CartoonContact::landing(server)
			} else {
				CartoonContact::wave(server)
			};
			let cl = CombinedLanding::contact(server);
			let cw = CombinedWaveContact::load(server);
			let mut commands = w.commands();
			let root = if self.effect.starts_with("cartoon") {
				commands.spawn_scene(Effect { recipe: cartoon, spawn }).id()
			} else if landing {
				commands.spawn_scene(Effect { recipe: cl, spawn }).id()
			} else {
				commands.spawn_scene(Effect { recipe: cw, spawn }).id()
			};
			commands.entity(root).insert((
				ContactInputs::default(),
				Feed {
					frame,
					scene: scene.clone(),
					submitted: -1,
					cancel_at: if i == 0 { self.cancel_first_at } else { 0. },
				},
			));
			for id in 0..3 {
				let c = scene.colliders(0.)[id];
				let scale = match c.shape {
					maybraid_vfx::parts::combined::interaction::Shape::Box { half } => {
						half.as_vec3() * 2.
					}
					_ => Vec3::ONE,
				};
				commands.spawn((
					SceneItem,
					Actor { root, frame, scene: scene.clone(), id },
					Mesh3d(if id == 0 { capsule.clone() } else { cube.clone() }),
					MeshMaterial3d(material.clone()),
					Transform::from_scale(scale),
				));
			}
			commands.spawn((
				SceneItem,
				Mesh3d(patch.clone()),
				MeshMaterial3d(ground.clone()),
				Transform::from_translation(matrix.transform_point3(Vec3::new(1., -0.04, 0.)))
					.with_rotation(r),
			));
		}
		let look = Vec3::new(1. + (self.count - 1) as f32 * self.spacing * 0.5, 1., 0.);
		let offset = if self.close {
			Vec3::new(-1.3, 0.8, 1.8)
		} else if self.side {
			Vec3::new(-6., 2., 0.)
		} else {
			Vec3::new(6., 4., 7.) + Vec3::new(0., 0., (self.count - 1) as f32 * self.spacing * 0.65)
		};
		let camera = Transform::from_translation(base.transform_point3(look + offset))
			.looking_at(base.transform_point3(look), Vec3::Y);
		for (mut t, mut c) in
			w.query::<(&mut Transform, &mut crate::camera::CameraController)>().iter_mut(w)
		{
			*t = camera;
			c.inspection = true;
		}
		w.insert_resource(Last(Some(self)));
	}
}
fn clear(w: &mut World) {
	let ids = w.query_filtered::<Entity, With<SceneItem>>().iter(w).collect::<Vec<_>>();
	for e in ids {
		w.despawn(e);
	}
}
pub fn reset_or_cancel(w: &mut World, reset: bool) -> bool {
	let config = w.resource::<Last>().0.clone();
	let Some(config) = config else {
		return false;
	};
	clear(w);
	let ids = w.query_filtered::<Entity, With<Feed>>().iter(w).collect::<Vec<_>>();
	for e in ids {
		w.despawn(e);
	}
	if reset {
		config.start(w);
	}
	true
}
pub fn install(app: &mut App) {
	app.init_resource::<Last>()
		.add_systems(
			Update,
			feed.after(maybraid_vfx::lifecycle::tick_vfx_instances).before(CartoonFlowTick),
		)
		.add_systems(Update, visuals.after(CartoonFlowTick));
}
fn feed(
	mut commands: Commands,
	mut roots: Query<(Entity, &VfxInstance, &mut Feed, &mut ContactInputs)>,
) {
	let sources = roots
		.iter()
		.map(|(entity, i, f, _)| {
			(entity, f.frame, f.scene.clone(), i.age as f64, i.playback as f64)
		})
		.collect::<Vec<_>>();
	for (root, instance, mut f, mut input) in &mut roots {
		if f.cancel_at > 0. && instance.age >= f.cancel_at {
			commands.entity(root).try_despawn();
			continue;
		}
		let target = (instance.age as f64 * 120.).ceil() as i32 + 2;
		if target < f.submitted - 4 {
			f.submitted = -1;
			*input = default();
		}
		while f.submitted < target {
			f.submitted += 1;
			let t = f.submitted as f64 / 120.;
			let mut colliders = Vec::new();
			// The owning actor is primary; other nearby actors remain additional solids.
			// Entity-derived IDs stay stable when another region is cancelled.
			for (source, frame, scene, age, playback) in sources
				.iter()
				.filter(|(e, _, _, _, _)| *e == root)
				.chain(sources.iter().filter(|(e, _, _, _, _)| *e != root))
			{
				let source_t = (*age
					+ (t - instance.age as f64) * playback / instance.playback as f64)
					.max(0.);
				for mut c in
					proxy_colliders(scene, source_t).into_iter().take(scene.collider_count())
				{
					c.id = (source.to_bits() as u32).wrapping_mul(4).wrapping_add(c.id);
					// Source paths use their own age clock; this snapshot uses the receiver's.
					let rate = playback / instance.playback as f64;
					c.linear *= rate;
					c.angular *= rate;
					let c = frame.world_collider(c);
					if f.frame.relevant(c) {
						colliders.push(c);
					}
				}
			}

			let wind = f.frame.world_vector(f.scene.wind);
			if let Err(e) = input.submit(WorldContactSnapshot {
				age: t,
				colliders,
				wind,
				discontinuous: f.submitted == 0,
			}) {
				warn!("Contact input rejected: {e}");
				break;
			}
		}
	}
}
fn visuals(
	mut commands: Commands,
	mut actors: Query<(Entity, &Actor, &mut Transform)>,
	roots: Query<&VfxInstance>,
	controllers: Query<(&CartoonFlow, &maybraid_vfx::membership::VfxMemberOf)>,
	mut gizmos: Gizmos,
) {
	for (e, a, mut t) in &mut actors {
		if roots.get(a.root).is_err() {
			commands.entity(e).try_despawn();
			continue;
		}
		let age = controllers
			.iter()
			.find(|(_, m)| m.root() == a.root)
			.map_or(0., |(c, _)| c.tick as f64 / 120.);
		let c = a.frame.world_collider(proxy_colliders(&a.scene, age)[a.id]);
		t.translation = c.position.as_vec3();
		t.rotation = c.rotation.as_quat();
		if a.id == 0 {
			let f = a.frame.world_from_local;
			let (_, r, _) = f.to_scale_rotation_translation();
			gizmos.cube(
				Transform::from_translation(f.transform_point3(Vec3::new(1., 1.5, 0.)))
					.with_rotation(r)
					.with_scale(Vec3::new(6., 3., 6.)),
				Color::srgb(0.2, 0.7, 0.7),
			);
		}
	}
}

pub fn inspect(w: &mut World, view: &str) -> bool {
	let Some(config) = w.resource::<Last>().0.clone() else {
		return false;
	};
	let placement = Mat4::from_rotation_translation(
		Quat::from_rotation_y(config.yaw.to_radians()),
		Vec3::new(config.position[0], config.position[1], config.position[2]),
	);
	let mut center = Vec3::new(0.7, 0.65, 0.);
	if matches!(view, "inside" | "boundary") {
		if let Some(c) = w.query::<&CartoonFlow>().iter(w).find(|c| {
			c.region
				.as_ref()
				.is_some_and(|r| r.frame.world_from_local.abs_diff_eq(placement, 1e-5))
		}) {
			if let Some(b) = c
				.bodies
				.iter()
				.flatten()
				.max_by(|a, b| a.scale.length_squared().total_cmp(&b.scale.length_squared()))
			{
				center = b.position.as_vec3();
			}
		}
	}
	let offset = match view {
		"main" => Vec3::new(6., 4., 7.),
		"side" => Vec3::new(-6., 1., 0.),
		"top" => Vec3::new(0.1, 7., 0.1),
		"close" => Vec3::new(-1.3, 0.8, 1.8),
		"boundary" => Vec3::new(0.1, 0., 0.45),
		"inside" => Vec3::new(0.03, 0.03, 0.04),
		_ => return false,
	};
	let t = Transform::from_translation(placement.transform_point3(center + offset))
		.looking_at(placement.transform_point3(center), Vec3::Y);
	for (mut transform, mut controller) in
		w.query::<(&mut Transform, &mut crate::camera::CameraController)>().iter_mut(w)
	{
		*transform = t;
		let (yaw, pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
		controller.yaw = yaw;
		controller.pitch = pitch;
		controller.inspection = true;
	}
	true
}

/// Leaving this validation scene retires only its owned actors, patches and regions.
pub fn leave(w: &mut World) {
	clear(w);
	let ids = w.query_filtered::<Entity, With<Feed>>().iter(w).collect::<Vec<_>>();
	for e in ids {
		w.despawn(e);
	}
	w.resource_mut::<Last>().0 = None;
}

// Continue the Wave proxy past the fixed domain after its approved contact crossing.
// This path belongs to the playground; no field/body/domain is recentered.
fn proxy_colliders(
	scene: &InteractionScene,
	t: f64,
) -> [maybraid_vfx::parts::combined::interaction::Interactor; 20] {
	let mut c = scene.colliders(t);
	if !scene.compact_landing && t >= 3.3 {
		c[0].position.z = (2. + 2. * (t - 3.3)).min(3.35);
		c[0].linear = if t < 3.975 { bevy::math::DVec3::Z * 2. } else { bevy::math::DVec3::ZERO };
	}
	c
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn proxy_preserves_crossing_then_leaves_fixed_region() {
		let scene = InteractionScene { cartoon_wave_timing: true, ..default() };
		for t in [0., 1.5, 2.2, 3.29] {
			assert_eq!(proxy_colliders(&scene, t)[0].position, scene.colliders(t)[0].position);
			assert_eq!(proxy_colliders(&scene, t)[0].linear, scene.colliders(t)[0].linear);
		}
		let c = proxy_colliders(&scene, 4.)[0];
		assert!(c.position.z - 0.28 > 3.);
		assert_eq!(c.linear, bevy::math::DVec3::ZERO);
		assert_eq!(c.position, proxy_colliders(&scene, 5.5)[0].position);
	}
}
