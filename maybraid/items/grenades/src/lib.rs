//! Thrown grenade body, fuse, blast hits, and detonation message.

mod material;

use avian3d::prelude::*;
use bevy::prelude::*;
use character_items::{GrenadeSpec, GrenadeStats};
use damage::Hit;
use lod_avian::PhysicsInteractionLayer;
use std::f32::consts::PI;

pub use material::{grenade_material, GrenadeMaterial, GrenadeMaterialPlugin};

/// Palette-free explosion knobs copied onto the thrown body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrenadeEffect {
	pub scale: f32,
	pub intensity: f32,
	pub playback: f32,
	pub seed: Option<u64>,
}

impl GrenadeEffect {
	pub fn from_stats(stats: &GrenadeStats) -> Self {
		Self {
			scale: stats.effect_scale,
			intensity: stats.effect_intensity,
			playback: stats.effect_playback,
			seed: None,
		}
	}
}

/// Independent world entity after release.
#[derive(Component, Debug)]
pub struct ThrownGrenade {
	pub spec: GrenadeSpec,
	pub stats: GrenadeStats,
	pub effect: GrenadeEffect,
	pub source: Entity,
	pub detonated: bool,
}

#[derive(Component, Debug)]
pub struct GrenadeFuse {
	pub remaining: f32,
}

/// Authoritative detonation. VFX and blast both consume this.
#[derive(Message, Clone, Copy, Debug)]
pub struct GrenadeDetonated {
	pub position: Vec3,
	pub source: Entity,
	pub effect: GrenadeEffect,
	pub radius: f32,
	pub damage: f32,
}

/// Animated membership, contacts Fixed and other Animated (terrain and capsules).
pub fn grenade_layers() -> CollisionLayers {
	PhysicsInteractionLayer::animated_layers()
}

pub fn grenade_mesh() -> Mesh {
	Sphere::new(0.5).mesh().ico(3).expect("grenade sphere")
}

/// Full damage at the origin, zero at `radius`.
pub fn blast_amount(damage: f32, radius: f32, distance: f32) -> f32 {
	if damage <= 0.0 || radius <= 1e-4 || distance > radius {
		return 0.0;
	}
	damage * (1.0 - distance / radius).clamp(0.0, 1.0)
}

pub fn spawn_thrown_grenade(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<GrenadeMaterial>,
	origin: Vec3,
	velocity: Vec3,
	spec: GrenadeSpec,
	stats: GrenadeStats,
	effect: GrenadeEffect,
	source: Entity,
) -> Entity {
	let radius = stats.radius.max(0.08);
	let collider = Collider::sphere(radius);
	let volume = (4.0 / 3.0) * PI * radius * radius * radius;
	let density = (stats.mass / volume.max(1e-6)).max(0.05);
	let tumble = velocity.cross(Vec3::Y).normalize_or(Vec3::X) * 10.0;
	let body = commands
		.spawn((
			Name::new("thrown-grenade"),
			Transform {
				translation: origin,
				rotation: Quat::from_rotation_arc(Vec3::Y, velocity.normalize_or(Vec3::Y)),
				scale: Vec3::splat(radius * 2.0),
			},
			Visibility::default(),
			Mesh3d(meshes.add(grenade_mesh())),
			MeshMaterial3d(materials.add(grenade_material())),
			RigidBody::Dynamic,
			MassPropertiesBundle::from_shape(&collider, density),
			collider,
			grenade_layers(),
			LinearVelocity(velocity),
			AngularVelocity(tumble),
			GravityScale(1.0),
		))
		.id();
	commands.entity(body).insert((
		Restitution { coefficient: stats.restitution, combine_rule: CoefficientCombine::Average },
		Friction {
			dynamic_coefficient: stats.friction,
			static_coefficient: stats.friction,
			combine_rule: CoefficientCombine::Average,
		},
		ThrownGrenade { spec, stats, effect, source, detonated: false },
		GrenadeFuse { remaining: stats.fuse },
	));
	body
}

pub fn tick_grenade_fuses(
	time: Res<Time>,
	mut detonations: MessageWriter<GrenadeDetonated>,
	mut commands: Commands,
	mut grenades: Query<(Entity, &Transform, &mut ThrownGrenade, &mut GrenadeFuse)>,
) {
	let dt = time.delta_secs();
	for (entity, transform, mut grenade, mut fuse) in &mut grenades {
		if grenade.detonated {
			continue;
		}
		fuse.remaining -= dt;
		if fuse.remaining > 0.0 {
			continue;
		}
		grenade.detonated = true;
		detonations.write(GrenadeDetonated {
			position: transform.translation,
			source: grenade.source,
			effect: grenade.effect,
			radius: grenade.stats.blast_radius,
			damage: grenade.stats.blast_damage,
		});
		commands.entity(entity).try_despawn();
	}
}

/// Overlap every `Health` pool in the blast and write [`Hit`]s. `source` is the thrower
/// so player-thrown blasts feed hit markers.
pub fn apply_grenade_blasts(
	mut detonations: MessageReader<GrenadeDetonated>,
	health: Query<(Entity, &Transform, Option<&GlobalTransform>), With<damage::Health>>,
	mut hits: MessageWriter<Hit>,
) {
	for blast in detonations.read() {
		let mut damaged = Vec::new();
		for (entity, transform, global) in &health {
			if entity == blast.source {
				continue;
			}
			if damaged.contains(&entity) {
				continue;
			}
			let point = global.map(GlobalTransform::translation).unwrap_or(transform.translation);
			let amount = blast_amount(blast.damage, blast.radius, point.distance(blast.position));
			if amount <= 1e-3 {
				continue;
			}
			damaged.push(entity);
			hits.write(Hit { target: entity, source: Some(blast.source), amount, point });
		}
	}
}

pub struct GrenadesPlugin;

impl Plugin for GrenadesPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<GrenadeMaterialPlugin>() {
			app.add_plugins(GrenadeMaterialPlugin);
		}
		app.add_message::<GrenadeDetonated>().add_message::<Hit>().add_systems(
			Update,
			(tick_grenade_fuses, apply_grenade_blasts.after(tick_grenade_fuses)),
		);
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use damage::Health;

	use super::*;

	fn standard_blast() -> GrenadeDetonated {
		let stats = GrenadeStats::standard();
		GrenadeDetonated {
			position: Vec3::ZERO,
			source: Entity::from_bits(1),
			effect: GrenadeEffect::from_stats(&stats),
			radius: stats.blast_radius,
			damage: stats.blast_damage,
		}
	}

	#[test]
	fn effect_from_stats_forwards_scale() {
		let stats = GrenadeStats::standard();
		let effect = GrenadeEffect::from_stats(&stats);
		assert!((effect.scale - stats.effect_scale).abs() < 1e-4);
	}

	#[test]
	fn fuse_expires_once() {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<Messages<GrenadeDetonated>>();
		let stats = GrenadeStats::standard();
		let id = world
			.spawn((
				Transform::from_xyz(1.0, 0.5, 2.0),
				ThrownGrenade {
					spec: GrenadeSpec::standard(),
					stats,
					effect: GrenadeEffect::from_stats(&stats),
					source: Entity::PLACEHOLDER,
					detonated: false,
				},
				GrenadeFuse { remaining: 0.0 },
			))
			.id();
		world.run_system_once(tick_grenade_fuses).expect("tick");
		assert!(!world.entities().contains(id));
		assert_eq!(world.resource_mut::<Messages<GrenadeDetonated>>().drain().count(), 1);
	}

	#[test]
	fn thrown_body_contacts_terrain_and_characters() {
		let grenade = grenade_layers();
		let fixed = PhysicsInteractionLayer::fixed_layers();
		let animated = PhysicsInteractionLayer::animated_layers();
		assert!(grenade.interacts_with(fixed));
		assert!(fixed.interacts_with(grenade));
		assert!(grenade.interacts_with(animated));
	}

	#[test]
	fn blast_falls_off_to_the_rim() {
		assert!((blast_amount(80.0, 8.0, 0.0) - 80.0).abs() < 1e-4);
		assert!((blast_amount(80.0, 8.0, 4.0) - 40.0).abs() < 1e-4);
		assert_eq!(blast_amount(80.0, 8.0, 8.1), 0.0);
	}

	#[test]
	fn blast_hits_carry_the_thrower() {
		let mut world = World::new();
		world.init_resource::<Messages<GrenadeDetonated>>();
		world.init_resource::<Messages<Hit>>();
		let thrower = world.spawn_empty().id();
		let target =
			world.spawn((Health::from_max(100.0), Transform::from_xyz(2.0, 0.0, 0.0))).id();
		let mut blast = standard_blast();
		blast.source = thrower;
		blast.position = Vec3::ZERO;
		world.resource_mut::<Messages<GrenadeDetonated>>().write(blast);
		world.run_system_once(apply_grenade_blasts).expect("blast");
		let hits: Vec<Hit> = world.resource_mut::<Messages<Hit>>().drain().collect();
		assert_eq!(hits.len(), 1);
		assert_eq!(hits[0].target, target);
		assert_eq!(hits[0].source, Some(thrower));
		assert!(hits[0].amount > 0.0);
	}

	#[test]
	fn blast_does_not_hit_the_thrower() {
		let mut world = World::new();
		world.init_resource::<Messages<GrenadeDetonated>>();
		world.init_resource::<Messages<Hit>>();
		let thrower =
			world.spawn((Health::from_max(100.0), Transform::from_xyz(0.5, 0.0, 0.0))).id();
		let mut blast = standard_blast();
		blast.source = thrower;
		world.resource_mut::<Messages<GrenadeDetonated>>().write(blast);
		world.run_system_once(apply_grenade_blasts).expect("blast");
		assert_eq!(world.resource_mut::<Messages<Hit>>().drain().count(), 0);
	}
}
