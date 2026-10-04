//! Thrown grenade body, fuse, and detonation message.

use bevy::prelude::*;
use character_items::{GrenadeSpec, GrenadeStats};
use projectiles::{spawn_tossed, TossedSpec};

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
	origin: Vec3,
	velocity: Vec3,
	spec: GrenadeSpec,
	stats: GrenadeStats,
	effect: GrenadeEffect,
	source: Entity,
) -> Entity {
	let tossed = spawn_tossed(
		commands,
		meshes,
		materials,
		origin,
		velocity,
		TossedSpec {
			radius: stats.radius.max(0.08),
			gravity: 1.0,
			color: Color::srgb(0.28, 0.34, 0.18),
		},
	);
	commands.entity(tossed).insert((
		Name::new("thrown-grenade"),
		ThrownGrenade { spec, stats, effect, source, detonated: false },
		GrenadeFuse { remaining: stats.fuse },
	));
	tossed
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
