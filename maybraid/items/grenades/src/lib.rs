//! Thrown grenade body, fuse, and detonation message.

use avian3d::prelude::{
	AngularVelocity, Collider, CollisionLayers, Friction, GravityScale, LinearVelocity,
	MassPropertiesBundle, Restitution, RigidBody, SpatialQuery, SpatialQueryFilter,
};
use bevy::prelude::*;
use character_items::{GrenadeSpec, GrenadeStats};
use lod_avian::PhysicsInteractionLayer;

pub const CLEARANCE: f32 = 0.18;

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

/// Authoritative detonation. The VFX adapter consumes this; damage can later.
#[derive(Message, Clone, Copy, Debug)]
pub struct GrenadeDetonated {
	pub position: Vec3,
	pub source: Entity,
	pub effect: GrenadeEffect,
}

pub fn grenade_mesh() -> Mesh {
	Sphere::new(0.5).mesh().ico(2).expect("grenade sphere")
}

pub fn spawn_thrown_grenade(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	spatial: &SpatialQuery,
	origin: Vec3,
	velocity: Vec3,
	spec: GrenadeSpec,
	stats: GrenadeStats,
	effect: GrenadeEffect,
	source: Entity,
	exclude: impl IntoIterator<Item = Entity>,
) -> Entity {
	let collider = Collider::sphere(stats.radius);
	let spawn = cleared_origin(spatial, origin, velocity, stats.radius, exclude);
	let mut tumble = velocity.cross(Vec3::Y);
	if tumble.length_squared() < 1e-6 {
		tumble = Vec3::X;
	}
	let rotation = Quat::from_axis_angle(tumble.normalize(), 0.4);
	commands
		.spawn((
			Name::new("thrown-grenade"),
			Transform {
				translation: spawn,
				rotation,
				scale: Vec3::splat(stats.radius * 2.0),
			},
			Visibility::Visible,
			Mesh3d(meshes.add(grenade_mesh())),
			MeshMaterial3d(materials.add(StandardMaterial {
				base_color: Color::srgb(0.28, 0.34, 0.18),
				perceptual_roughness: 0.72,
				metallic: 0.18,
				..default()
			})),
			RigidBody::Dynamic,
			MassPropertiesBundle::from_shape(&collider, density(&stats)),
			collider,
			grenade_layers(),
			ThrownGrenade { spec, stats, effect, source, detonated: false },
			GrenadeFuse { remaining: stats.fuse },
		))
		.insert((
			LinearVelocity(velocity),
			AngularVelocity(tumble.normalize() * 6.0),
			GravityScale(1.0),
			Restitution::new(stats.restitution),
			Friction::new(stats.friction),
		))
		.id()
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
		});
		commands.entity(entity).try_despawn();
	}
}

pub struct GrenadesPlugin;

impl Plugin for GrenadesPlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<GrenadeDetonated>().add_systems(Update, tick_grenade_fuses);
	}
}

/// Animated membership, Fixed-only filter: terrain contact without character capsules.
pub fn grenade_layers() -> CollisionLayers {
	CollisionLayers::new(PhysicsInteractionLayer::Animated, PhysicsInteractionLayer::Fixed)
}

fn density(stats: &GrenadeStats) -> f32 {
	let volume = 4.0 / 3.0 * std::f32::consts::PI * stats.radius.powi(3);
	stats.mass.max(0.05) / volume.max(1e-8)
}

fn cleared_origin(
	spatial: &SpatialQuery,
	origin: Vec3,
	velocity: Vec3,
	radius: f32,
	exclude: impl IntoIterator<Item = Entity>,
) -> Vec3 {
	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)
		.with_excluded_entities(exclude);
	let dir = if velocity.length_squared() > 1e-6 { velocity.normalize() } else { Vec3::Y };
	if let Some(hit) = spatial.cast_ray(origin, Dir3::new(dir).unwrap_or(Dir3::Y), CLEARANCE, true, &filter)
	{
		return origin - dir * (radius + 0.04).min(hit.distance);
	}
	origin
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use super::*;

	#[test]
	fn fuse_expires_once() {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<Messages<GrenadeDetonated>>();
		let id = world
			.spawn((
				Transform::from_xyz(1.0, 0.5, 2.0),
				ThrownGrenade {
					spec: GrenadeSpec::standard(),
					stats: GrenadeStats::standard(),
					effect: GrenadeEffect::from_stats(&GrenadeStats::standard()),
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
}
