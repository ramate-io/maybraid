use bevy::prelude::*;

use crate::{DamageApplied, Died, HeadshotBand, Health, Hit};

pub fn apply_hits(
	mut hits: MessageReader<Hit>,
	mut health: Query<&mut Health>,
	bands: Query<(&GlobalTransform, &HeadshotBand)>,
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
		let amount = bands
			.get(hit.target)
			.map(|(transform, band)| band.scale(transform, hit.point, hit.amount))
			.unwrap_or(hit.amount);
		target.apply_damage(amount);
		applied.write(DamageApplied {
			target: hit.target,
			source: hit.source,
			amount,
			remaining: target.current,
			point: hit.point,
		});
		if target.is_dead() {
			died.write(Died { entity: hit.target, source: hit.source });
		}
	}
}
