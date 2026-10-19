//! Shared Lubikan interface.
//!
//! Other lanes consume these types. Self is a normal definition; a missing
//! equipped selection resolves to Self. Charge and activation live in
//! gameplay — HUD construction must not create them.
//!
//! This crate does not depend on Bikanwarda.

mod activation;
mod award;
mod catalog;
mod change;
mod charge;
mod heal;
mod identity;
mod ledger;
mod lifetime;
mod plugin;
mod selection;
mod summon;

#[cfg(any(test, feature = "test-support"))]
pub mod fixtures;

use bevy::prelude::*;

pub use activation::{
	LubikanActivation, LubikanActivationDenied, LubikanActivationFinished, LubikanActivationGrant,
	LubikanActivationOutcome, LubikanActivationPhase, LubikanActivationRefused,
	LubikanActivationReport, LubikanActivationRequest, LubikanActivationSession,
	LubikanActivationSessionId, LubikanReadiness,
};
pub use award::LubikanCombatAward;
pub use catalog::{
	LubikanCatalog, LubikanSelfConfig, DEFAULT_SELF_ACTIVATION_COST, DEFAULT_SELF_CAPACITY,
	DEFAULT_SELF_MAX_STOCK,
};
pub use change::{LubikanChargeChanged, LubikanChargeReason, LubikanChargeRequest};
pub use charge::{LubikanCharge, LubikanChargeMutation};
pub use heal::{
	LubikanHealingEnabled, LubikanSelfHeal, LubikanSelfHealChannel, LubikanSelfHealConfig,
	DEFAULT_SELF_HEAL_CAP_DAMAGED, DEFAULT_SELF_HEAL_CAP_HEALTHY, DEFAULT_SELF_HEAL_DURATION_SECS,
	DEFAULT_SELF_HEAL_RATE_FRAC,
};
pub use identity::{LubikanAccountId, LubikanDefinition, LubikanDefinitionId};
pub use ledger::{LubikanAccount, LubikanAccountBinding, LubikanLedger, LubikanOwner};
pub use lifetime::{
	LubikanLifetimePolicy, LubikanOwnerLifetime, LubikanOwnerLifetimeKind, LubikanPersistence,
};
pub use plugin::{LubikanSessionStore, LubikansPlugin};
pub use selection::LubikanSelection;
pub use summon::{
	fixture_summon_definition, install_fixture_summon, LubikanSummon, LubikanSummonCatalog,
	LubikanSummonConfig, LubikanSummonDeparture, LubikanSummonDepartureReason,
	LubikanSummonEnabled, LubikanSummonInstanceId, LubikanSummonInvalidPlacement,
	LubikanSummonPresentation, LubikanSummonRecipe, LubikanSummons, FIXTURE_SUMMON_DEFINITION_ID,
	PLACEHOLDER_SUMMON_ACTIVATION_COST, PLACEHOLDER_SUMMON_CAPACITY,
	PLACEHOLDER_SUMMON_FORWARD_OFFSET, PLACEHOLDER_SUMMON_HEALTH, PLACEHOLDER_SUMMON_LIFETIME_SECS,
	PLACEHOLDER_SUMMON_MAX_CONCURRENT,
};

/// Schedule sets other lanes order against. The interface plugin does not
/// run systems in these sets; gameplay plugins do.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LubikanSystems {
	/// Attach, snapshot, and restore owner ledgers.
	Owner,
	/// Sole owner of [`LubikanActivationRequest`]s: spend once and emit a
	/// [`LubikanActivationSession`], or emit [`LubikanActivationRefused`].
	Activate,
	/// Self healing consumer: pick up Self sessions, regenerate, and report.
	Heal,
	/// Summon consumer: pick up non-Self sessions, spawn, and report.
	Summon,
	/// Apply [`LubikanChargeRequest`]s and emit [`LubikanChargeChanged`].
	Award,
}

/// Registers the shared messages. Does not spawn owners or insert Self into
/// a world — that is gameplay, and stays independent of HUD visibility.
pub struct LubikanInterfacePlugin;

impl Plugin for LubikanInterfacePlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<LubikanChargeChanged>()
			.add_message::<LubikanChargeRequest>()
			.add_message::<LubikanActivationRequest>()
			.add_message::<LubikanActivationSession>()
			.add_message::<LubikanActivationRefused>()
			.add_message::<LubikanActivationReport>()
			.add_message::<LubikanActivationFinished>()
			.add_message::<LubikanOwnerLifetime>()
			.add_message::<LubikanSummonDeparture>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn interface_plugin_registers_shared_messages() {
		let mut app = App::new();
		app.add_plugins(LubikanInterfacePlugin);
		assert!(app.world().get_resource::<Messages<LubikanChargeChanged>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanChargeRequest>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationRequest>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationSession>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationRefused>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationReport>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanActivationFinished>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanOwnerLifetime>>().is_some());
		assert!(app.world().get_resource::<Messages<LubikanSummonDeparture>>().is_some());
	}
}
