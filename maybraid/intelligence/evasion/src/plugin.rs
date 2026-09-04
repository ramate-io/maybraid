use bevy::prelude::*;

use crate::spotting::sync_spotted_assailants;
use crate::EvasionIntelligenceUser;

/// Ordering: observe contacts, then rank, then hide | flee actuators.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EvasionSystems {
	Ingest,
	Rank,
}

pub struct EvasionPlugin;

impl Plugin for EvasionPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(Update, (EvasionSystems::Ingest, EvasionSystems::Rank).chain())
			.add_systems(Update, sync_spotted_assailants.in_set(EvasionSystems::Ingest))
			.add_systems(Update, rank_assailants.in_set(EvasionSystems::Rank));
	}
}

pub fn rank_assailants(
	time: Res<Time>,
	mut users: Query<(&Transform, &mut EvasionIntelligenceUser)>,
) {
	let now = time.elapsed_secs();
	for (transform, mut evasion) in &mut users {
		if evasion.needs_rebalance(now) {
			evasion.rebalance(now, transform.translation);
		}
	}
}
