use bevy::prelude::*;
use firearm_user::FirearmUser;
use player::{Npc, Player};
use projectiles::ProjectileContact;

pub(crate) const MAX_HEALTH: f32 = 100.0;
pub(crate) const PROJECTILE_DAMAGE: f32 = 25.0;
pub(crate) const RESPAWN_SECS: f32 = 2.0;

#[derive(Resource, Default)]
pub(crate) struct CombatRespawn {
	pub player_at: Option<f32>,
	pub npc_at: Vec<f32>,
}

impl CombatRespawn {
	pub fn clear(&mut self) {
		self.player_at = None;
		self.npc_at.clear();
	}
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct Health {
	pub current: f32,
	pub max: f32,
}

impl Default for Health {
	fn default() -> Self {
		Self { current: MAX_HEALTH, max: MAX_HEALTH }
	}
}

impl Health {
	pub fn apply_damage(&mut self, damage: f32) {
		self.current = (self.current - damage.max(0.0)).max(0.0);
	}

	pub fn is_dead(self) -> bool {
		self.current <= 0.0
	}

	pub fn fraction(self) -> f32 {
		if self.max <= 0.0 {
			0.0
		} else {
			(self.current / self.max).clamp(0.0, 1.0)
		}
	}
}

/// Applied hit. `origin` is the attacker position, or the contact point if unknown.
#[derive(Message, Clone, Copy, Debug)]
pub(crate) struct DamageTaken {
	pub target: Entity,
	pub origin: Vec3,
}

pub(crate) fn apply_projectile_damage(
	mut contacts: MessageReader<ProjectileContact>,
	mut health: Query<&mut Health>,
	transforms: Query<&GlobalTransform>,
	mut hits: MessageWriter<DamageTaken>,
) {
	for contact in contacts.read() {
		if contact.source == Some(contact.target) {
			continue;
		}
		let Ok(mut target) = health.get_mut(contact.target) else {
			continue;
		};
		if target.is_dead() {
			continue;
		}
		target.apply_damage(PROJECTILE_DAMAGE);
		let origin = contact
			.source
			.and_then(|source| transforms.get(source).ok())
			.map(GlobalTransform::translation)
			.unwrap_or(contact.point);
		hits.write(DamageTaken { target: contact.target, origin });
	}
}

type DeadCombatants<'w, 's> =
	Query<'w, 's, (Entity, &'static Health, Option<&'static FirearmUser>, Has<Player>, Has<Npc>)>;

pub(crate) fn despawn_dead(
	time: Res<Time>,
	mut respawn: ResMut<CombatRespawn>,
	mut engagement: ResMut<crate::engagement::NpcEngagement>,
	mut commands: Commands,
	combatants: DeadCombatants,
) {
	let now = time.elapsed_secs();
	for (entity, health, user, is_player, is_npc) in &combatants {
		if !health.is_dead() {
			continue;
		}
		if is_player {
			respawn.player_at = Some(now + RESPAWN_SECS);
			engagement.reset();
		}
		if is_npc {
			respawn.npc_at.push(now + RESPAWN_SECS);
		}
		if let Some(user) = user {
			commands.entity(user.held).try_despawn();
		}
		commands.entity(entity).try_despawn();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn damage_clamps_at_zero() {
		let mut health = Health::default();
		health.apply_damage(25.0);
		assert_eq!(health.current, 75.0);
		health.apply_damage(100.0);
		assert_eq!(health.current, 0.0);
		assert!(health.is_dead());
	}

	#[test]
	fn fraction_tracks_remaining() {
		let mut health = Health::default();
		assert_eq!(health.fraction(), 1.0);
		health.apply_damage(50.0);
		assert!((health.fraction() - 0.5).abs() < 1e-5);
		health.apply_damage(50.0);
		assert_eq!(health.fraction(), 0.0);
	}
}
