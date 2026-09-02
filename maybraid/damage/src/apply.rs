use bevy::prelude::*;

use crate::{DamageApplied, Died, Health, Hit};

pub fn apply_hits(
	mut hits: MessageReader<Hit>,
	mut health: Query<&mut Health>,
	mut applied: MessageWriter<DamageApplied>,
	mut died: MessageWriter<Died>,
) {
	for hit in hits.read() {
		if hit.source == Some(hit.target) {
			continue;
		}
		let Ok(mut target) = health.get_mut(hit.target) else {
			continue;
		};
		if target.is_dead() {
			continue;
		}
		target.apply_damage(hit.amount);
		applied.write(DamageApplied {
			target: hit.target,
			source: hit.source,
			amount: hit.amount,
			remaining: target.current,
			point: hit.point,
		});
		if target.is_dead() {
			died.write(Died { entity: hit.target, source: hit.source });
		}
	}
}
