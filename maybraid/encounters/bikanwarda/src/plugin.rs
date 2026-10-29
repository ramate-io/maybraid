//! Bikanwarda outcomes on top of the generic progress owner.

use bevy::prelude::*;
use encounters::{EncounterLifetimePolicy, EncountersPlugin};
use lubikans::{LubikanCatalog, LubikanInterfacePlugin, LubikansPlugin};

use crate::outcomes;
use crate::unlocks::{register_unlocks, BikanwardaUnlockConfig};

/// Registers Ward/Enraged, awards from [`encounters::EncounterResolved`], and
/// installs [`EncountersPlugin`] with the Bikanwarda lifetime policy.
///
/// Does not spawn world hosts, depend on combat-hud, or own Lubikan session
/// ledgers. [`LubikansPlugin`] remains the ledger owner.
pub struct BikanwardaEncounterPlugin;

impl Plugin for BikanwardaEncounterPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<EncountersPlugin>() {
			app.add_plugins(EncountersPlugin);
		}
		if !app.is_plugin_added::<LubikanInterfacePlugin>() {
			app.add_plugins(LubikanInterfacePlugin);
		}
		if !app.is_plugin_added::<LubikansPlugin>() {
			app.add_plugins(LubikansPlugin);
		}
		app.insert_resource(EncounterLifetimePolicy::bikanwarda());
		app.init_resource::<BikanwardaUnlockConfig>();
		if app.world().get_resource::<LubikanCatalog>().is_none() {
			app.init_resource::<LubikanCatalog>();
		}
		{
			let config = *app.world().resource::<BikanwardaUnlockConfig>();
			register_unlocks(&mut app.world_mut().resource_mut::<LubikanCatalog>(), &config);
		}
		outcomes::configure(app);
	}
}
