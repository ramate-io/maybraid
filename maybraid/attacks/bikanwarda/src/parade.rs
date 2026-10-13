//! Sequential Parade blasts. The parent attack owns the line telegraph; each
//! explosion is a concurrent [`crate::PARADE_BLAST`] on [`attacks::HazardOrigin::Emission`].

use attacks::{AttackInstance, AttackStage, AttackSystems, BeginAttack};
use bevy::prelude::*;

use crate::{BikanwardaAttackParams, PARADE, PARADE_BLAST};

/// How far the parent Parade has walked its authored blast list.
#[derive(Component, Clone, Copy, Debug)]
pub struct ParadeClock {
	pub next_index: u8,
}

pub fn parade_plugin(app: &mut App) {
	app.add_systems(
		Update,
		(attach_parade_clocks, emit_parade_blasts).chain().after(AttackSystems::Advance),
	);
}

fn attach_parade_clocks(
	instances: Query<(Entity, &AttackInstance), Without<ParadeClock>>,
	mut commands: Commands,
) {
	for (entity, instance) in &instances {
		if instance.definition == PARADE && instance.stage == AttackStage::Execute {
			commands.entity(entity).insert(ParadeClock { next_index: 0 });
		}
	}
}

fn emit_parade_blasts(
	params: Res<BikanwardaAttackParams>,
	mut clocks: Query<(&AttackInstance, &mut ParadeClock)>,
	mut begins: MessageWriter<BeginAttack>,
) {
	for (instance, mut clock) in &mut clocks {
		if instance.definition != PARADE || instance.stage != AttackStage::Execute {
			continue;
		}
		let parade = params.parade;
		while clock.next_index < parade.blast_count {
			let due = f32::from(clock.next_index) * parade.interval.max(0.0);
			if instance.stage_age + 1e-4 < due {
				break;
			}
			let point = parade.blast_point(
				instance.snapshot.origin,
				instance.snapshot.direction,
				clock.next_index,
			);
			let mut begin = BeginAttack::new(instance.owner, PARADE_BLAST)
				.with_allies(instance.allies.clone())
				.with_emission(point);
			if let Some(target) = instance.live_target {
				begin = begin.with_target(target);
			}
			begins.write(begin);
			clock.next_index = clock.next_index.saturating_add(1);
		}
	}
}
