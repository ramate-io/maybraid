//! Combat target list shared by firearm combat and firearm movement.

use bevy::prelude::*;

/// Someone a firearm combatant may shoot at or stand relative to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatTarget {
	pub entity: Entity,
}

/// Who to shoot. Written by perception; fielded by [`crate::FirearmIntelligence`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FirearmObjective(pub Vec<CombatTarget>);

impl FirearmObjective {
	pub fn from_target(entity: Entity) -> Self {
		Self(vec![CombatTarget { entity }])
	}
}

/// Who to stand relative to. Written by perception; fielded by
/// [`crate::FirearmMovementIntelligence`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FirearmMovementObjective(pub Vec<CombatTarget>);

impl FirearmMovementObjective {
	pub fn from_target(entity: Entity) -> Self {
		Self(vec![CombatTarget { entity }])
	}
}

/// Pick a target from `candidates` given current `from` and optional sticky `engaged`.
///
/// `focus` 0 always takes the nearest. `focus` 1 keeps `engaged` until it leaves
/// the list. In between, switch only when the nearest is closer by more than
/// `focus` × 8 m.
pub fn pick_target(
	from: Vec3,
	candidates: &[CombatTarget],
	positions: impl Fn(Entity) -> Option<Vec3>,
	engaged: Option<Entity>,
	focus: f32,
) -> Option<Entity> {
	let mut nearest: Option<(Entity, f32)> = None;
	let mut engaged_dist = None;
	for target in candidates {
		let Some(point) = positions(target.entity) else {
			continue;
		};
		let dist = Vec2::new(from.x, from.z).distance(Vec2::new(point.x, point.z));
		if engaged == Some(target.entity) {
			engaged_dist = Some(dist);
		}
		let take = nearest.is_none_or(|(_, best)| dist < best);
		if take {
			nearest = Some((target.entity, dist));
		}
	}
	let (nearest_entity, nearest_dist) = nearest?;
	let Some(engaged_entity) = engaged else {
		return Some(nearest_entity);
	};
	let Some(engaged_dist) = engaged_dist else {
		return Some(nearest_entity);
	};
	let stick = focus.clamp(0.0, 1.0) * 8.0;
	if nearest_dist + stick < engaged_dist {
		Some(nearest_entity)
	} else {
		Some(engaged_entity)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn pos(map: &[(Entity, Vec3)]) -> impl Fn(Entity) -> Option<Vec3> + '_ {
		|entity| map.iter().find(|(id, _)| *id == entity).map(|(_, p)| *p)
	}

	#[test]
	fn pick_target_takes_nearest_when_unfocused() -> anyhow::Result<()> {
		let a = Entity::from_bits(1);
		let b = Entity::from_bits(2);
		let map = [(a, Vec3::X * 4.0), (b, Vec3::X * 2.0)];
		let picked = pick_target(
			Vec3::ZERO,
			&[CombatTarget { entity: a }, CombatTarget { entity: b }],
			pos(&map),
			Some(a),
			0.0,
		);
		assert_eq!(picked, Some(b));
		Ok(())
	}

	#[test]
	fn pick_target_keeps_engaged_when_focused() -> anyhow::Result<()> {
		let a = Entity::from_bits(1);
		let b = Entity::from_bits(2);
		let map = [(a, Vec3::X * 4.0), (b, Vec3::X * 2.0)];
		let picked = pick_target(
			Vec3::ZERO,
			&[CombatTarget { entity: a }, CombatTarget { entity: b }],
			pos(&map),
			Some(a),
			1.0,
		);
		assert_eq!(picked, Some(a));
		Ok(())
	}
}
