//! Cheap local puttering through the movement engine.
//!
//! Writes [`MovementObjective::Reach`] inside a disk around a home origin.
//! Meandering stays preferred: this brain does not run while a [`PoiGoal`] is
//! active.

use bevy::prelude::*;
use movement_intelligence::{
	MovementIntelligence, MovementIntelligenceSystems, MovementLocation, MovementObjective,
	ReplanMovement,
};
use poi_intelligence::{PoiGoal, PoiSystems};
use std::f32::consts::TAU;
use tether_intelligence::TetherSystems;

const REFRESH_DISTANCE: f32 = 0.8;

/// Local mill around a home origin. Presence assigns the duty; [`Self::enabled`]
/// is the higher-order grant.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct IdlingIntelligenceUser {
	pub origin: Vec3,
	pub radius: f32,
	pub arrival_radius: f32,
	pub interval: f32,
	/// Higher-order grant. When false, this brain does not write movement.
	pub enabled: bool,
	next_at: f32,
	driving: bool,
}

impl Default for IdlingIntelligenceUser {
	fn default() -> Self {
		Self::around(Vec3::ZERO, 6.5)
	}
}

impl IdlingIntelligenceUser {
	pub fn around(origin: Vec3, radius: f32) -> Self {
		Self {
			origin,
			radius: radius.max(0.5),
			arrival_radius: 1.25,
			interval: 4.0,
			enabled: true,
			// Sit out the first POI scan so meander can claim a goal first.
			next_at: 1.25,
			driving: false,
		}
	}

	pub fn with_interval(mut self, interval: f32) -> Self {
		self.interval = interval.max(0.25);
		self
	}

	pub fn with_arrival_radius(mut self, arrival_radius: f32) -> Self {
		self.arrival_radius = arrival_radius.max(0.4);
		self
	}

	pub fn is_driving(&self) -> bool {
		self.driving
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IdlingSystems {
	Write,
}

pub struct IdlingPlugin;

impl Plugin for IdlingPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			IdlingSystems::Write
				.after(PoiSystems::Drive)
				.before(TetherSystems::Write)
				.before(MovementIntelligenceSystems::Replan),
		)
		.add_systems(Update, write_idle_objectives.in_set(IdlingSystems::Write));
	}
}

pub fn write_idle_objectives(
	time: Res<Time>,
	mut users: Query<
		(Entity, &Transform, &mut IdlingIntelligenceUser, &mut MovementIntelligence),
		Without<PoiGoal>,
	>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	for (entity, transform, mut idle, mut movement) in &mut users {
		if !idle.enabled {
			idle.driving = false;
			continue;
		}
		let at = transform.translation;
		if idle.driving {
			if let MovementObjective::Reach(location) = movement.objective {
				if !location.contains_xz(at) {
					continue;
				}
			}
			idle.driving = false;
		}
		if now < idle.next_at {
			continue;
		}
		let next = idle_objective(&idle, entity, now, at.y);
		idle.next_at = now + idle.interval * (0.7 + 0.6 * unit(hash(entity, now) ^ 1));
		idle.driving = true;
		if !should_replan(movement.objective, next) {
			continue;
		}
		movement.objective = next;
		commands.entity(entity).insert(ReplanMovement);
	}
}

fn idle_objective(
	idle: &IdlingIntelligenceUser,
	entity: Entity,
	now: f32,
	y: f32,
) -> MovementObjective {
	let seed = hash(entity, now);
	let angle = unit(seed) * TAU;
	let dist = idle.radius * unit(seed ^ 1).sqrt();
	let point =
		Vec3::new(idle.origin.x + angle.cos() * dist, y, idle.origin.z + angle.sin() * dist);
	MovementObjective::Reach(MovementLocation::new(point, idle.arrival_radius))
}

fn should_replan(current: MovementObjective, next: MovementObjective) -> bool {
	if std::mem::discriminant(&current) != std::mem::discriminant(&next) {
		return true;
	}
	let a = current.location().point;
	let b = next.location().point;
	Vec2::new(a.x, a.z).distance(Vec2::new(b.x, b.z)) >= REFRESH_DISTANCE
		|| (a.y - b.y).abs() >= REFRESH_DISTANCE
		|| (current.location().radius - next.location().radius).abs() > 0.05
}

fn hash(entity: Entity, now: f32) -> u64 {
	splitmix64(entity.to_bits() ^ now.to_bits() as u64)
}

fn unit(seed: u64) -> f32 {
	(splitmix64(seed) >> 40) as f32 / 16_777_216.0
}

fn splitmix64(mut value: u64) -> u64 {
	value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
	value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use poi_intelligence::{PoiId, PoiKind};

	fn spawn_idle(world: &mut World, user: IdlingIntelligenceUser) -> Entity {
		world
			.spawn((
				Transform::from_xyz(user.origin.x, user.origin.y, user.origin.z),
				MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(
					user.origin,
					0.4,
				))),
				user,
			))
			.id()
	}

	#[test]
	fn samples_stay_inside_the_idle_disk() -> anyhow::Result<()> {
		let idle = IdlingIntelligenceUser::around(Vec3::new(10.0, 1.0, -4.0), 6.0);
		for index in 0..32_u64 {
			let entity = Entity::from_bits(index + 1);
			let objective = idle_objective(&idle, entity, index as f32 * 0.37, 1.0);
			let point = objective.location().point;
			let dist =
				Vec2::new(point.x, point.z).distance(Vec2::new(idle.origin.x, idle.origin.z));
			assert!(dist <= idle.radius + 1e-3, "{dist}");
			assert!((point.y - 1.0).abs() < 1e-4);
		}
		Ok(())
	}

	#[test]
	fn writes_reach_when_due() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let mut user = IdlingIntelligenceUser::around(Vec3::ZERO, 6.0);
		user.next_at = 0.0;
		let npc = spawn_idle(&mut world, user);
		world.run_system_once(write_idle_objectives).unwrap();
		let movement = world.get::<MovementIntelligence>(npc).expect("movement");
		let point = movement.objective.location().point;
		let dist = point.xz().length();
		assert!(matches!(movement.objective, MovementObjective::Reach(_)));
		assert!(dist > 0.05, "{dist}");
		assert!(dist <= 6.0 + 1e-3, "{dist}");
		assert!(world.get::<ReplanMovement>(npc).is_some());
		assert!(world.get::<IdlingIntelligenceUser>(npc).is_some_and(|user| user.is_driving()));
		Ok(())
	}

	#[test]
	fn disabled_users_do_not_write() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let mut user = IdlingIntelligenceUser::around(Vec3::ZERO, 6.0);
		user.next_at = 0.0;
		user.enabled = false;
		let npc = spawn_idle(&mut world, user);
		world.run_system_once(write_idle_objectives).unwrap();
		let movement = world.get::<MovementIntelligence>(npc).expect("movement");
		assert_eq!(movement.objective.location().point, Vec3::ZERO);
		assert!(world.get::<ReplanMovement>(npc).is_none());
		Ok(())
	}

	#[test]
	fn active_poi_goal_blocks_idle() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let mut user = IdlingIntelligenceUser::around(Vec3::ZERO, 6.0);
		user.next_at = 0.0;
		let npc = spawn_idle(&mut world, user);
		world.entity_mut(npc).insert(PoiGoal::new(
			1,
			PoiId(1),
			None,
			PoiKind::new("test/place"),
			Vec3::X * 8.0,
			1.0,
			0.0,
		));
		world.run_system_once(write_idle_objectives).unwrap();
		let movement = world.get::<MovementIntelligence>(npc).expect("movement");
		assert_eq!(movement.objective.location().point, Vec3::ZERO);
		Ok(())
	}
}
