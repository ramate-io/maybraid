//! Sole mutator of [`EncounterProgressStore`].

use bevy::prelude::*;
use damage::{DamageSystems, Died};

use crate::lifetime::{EncounterLifetimePolicy, EncounterTimeEnabled};
use crate::signal::{EncounterResolved, EncounterSignal};
use crate::store::{
	bind_added_instances, ingest_deaths, ingest_removed_instances, resolve_signals,
	EncounterProgressStore,
};
use crate::EncounterSystems;

/// Gameplay plugin. No HUD, no HCSG, no Bikanwarda catalog.
pub struct EncountersPlugin;

impl Plugin for EncountersPlugin {
	fn build(&self, app: &mut App) {
		if app.world().get_resource::<Messages<Died>>().is_none() {
			app.add_message::<Died>();
		}
		app.init_resource::<EncounterProgressStore>()
			.init_resource::<EncounterLifetimePolicy>()
			.init_resource::<EncounterTimeEnabled>()
			.add_message::<EncounterSignal>()
			.add_message::<EncounterResolved>()
			.configure_sets(
				PostUpdate,
				(EncounterSystems::Ingest.after(DamageSystems::Down), EncounterSystems::Resolve)
					.chain(),
			)
			.add_systems(
				PostUpdate,
				(
					(bind_added_instances, ingest_deaths, ingest_removed_instances)
						.chain()
						.in_set(EncounterSystems::Ingest),
					resolve_signals.in_set(EncounterSystems::Resolve),
				),
			);
	}
}
