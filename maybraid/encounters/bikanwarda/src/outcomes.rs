//! Kill → Enraged, free → Ward. One occurrence, one drop.

use bevy::prelude::*;
use encounters::{EncounterOutcome, EncounterResolved, EncounterSystems};
use lubikans::{LubikanAccountId, LubikanCatalog, LubikanLedger};

use crate::unlocks::BikanwardaUnlockConfig;

/// Awarded unlock for one resolved occurrence. `first_grant` is false on repeats.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncounterUnlockAwarded {
	pub recipient: Entity,
	pub account: LubikanAccountId,
	pub first_grant: bool,
	pub outcome: EncounterOutcome,
}

/// Placeholder departure. Freed glows and transcends; killed uses ordinary death.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BikanwardaDeparted {
	pub instance: Option<Entity>,
	pub kind: BikanwardaDeparture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BikanwardaDeparture {
	Killed,
	Transcend,
}

pub(crate) fn configure(app: &mut App) {
	app.add_message::<EncounterUnlockAwarded>()
		.add_message::<BikanwardaDeparted>()
		.add_systems(
			PostUpdate,
			(award_unlocks, announce_departure).chain().after(EncounterSystems::Resolve),
		);
}

fn award_unlocks(
	mut resolved: MessageReader<EncounterResolved>,
	mut ledgers: Query<&mut LubikanLedger>,
	config: Res<BikanwardaUnlockConfig>,
	mut catalog: ResMut<LubikanCatalog>,
	mut awarded: MessageWriter<EncounterUnlockAwarded>,
) {
	crate::unlocks::register_unlocks(&mut catalog, &config);
	for event in resolved.read() {
		let definition = match event.outcome {
			EncounterOutcome::Killed => config.enraged(),
			EncounterOutcome::Freed => config.ward(),
		};
		let Some(recipient) = event.source else {
			continue;
		};
		let Ok(mut ledger) = ledgers.get_mut(recipient) else {
			continue;
		};
		let already = ledger.contains(LubikanAccountId::unique(definition.id));
		let account = ledger.ensure_owned(definition);
		awarded.write(EncounterUnlockAwarded {
			recipient,
			account,
			first_grant: !already,
			outcome: event.outcome,
		});
	}
}

fn announce_departure(
	mut resolved: MessageReader<EncounterResolved>,
	mut departed: MessageWriter<BikanwardaDeparted>,
) {
	for event in resolved.read() {
		let kind = match event.outcome {
			EncounterOutcome::Killed => BikanwardaDeparture::Killed,
			EncounterOutcome::Freed => BikanwardaDeparture::Transcend,
		};
		departed.write(BikanwardaDeparted { instance: event.instance, kind });
	}
}

#[cfg(test)]
mod tests {
	use crate::identity::{ENRAGED_DEFINITION_ID, WARD_DEFINITION_ID};

	#[test]
	fn kill_maps_to_enraged_and_free_to_ward() {
		assert_eq!(ENRAGED_DEFINITION_ID.raw(), 1);
		assert_eq!(WARD_DEFINITION_ID.raw(), 2);
	}
}
