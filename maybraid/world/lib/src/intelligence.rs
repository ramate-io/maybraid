//! Application-owned budgets and cadence for world NPC intelligence.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use evasion_intelligence::{EvasionPlugin, EvasionSystems};
use firearm_intelligence::{FirearmIntelligencePlugin, FirearmIntelligenceSystems};
use firearm_user::FirearmUserPlugin;
use firearms::{FirearmWeaponSystems, FirearmWeaponsPlugin};
use fleeing_intelligence::{FleeingPlugin, FleeingSystems};
use hiding_intelligence::{HidingPlugin, HidingSystems};
use meandering_intelligence::MeanderingIntelligencePlugin;
use movement_intelligence::{
	CandidateBudget, MovementIntelligenceLimits, MovementIntelligencePlugin,
};
use movement_intelligence_avian::AvianMovementSurface;
use movement_realization::MovementRealizationPlugin;
use poi_intelligence::PoiSystems;
use routing_intelligence::RoutingPlugin;
use spotting_intelligence::SpottingSystems;
use threat_intelligence::ThreatIntelligencePlugin;
use threat_intelligence_damage::ThreatIntelligenceDamagePlugin;
use threat_management_intelligence::ThreatManagementPlugin;

pub struct WorldIntelligencePlugin;

impl Plugin for WorldIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(MovementIntelligenceLimits {
			max_budget: CandidateBudget { max_candidates: 8, max_steps: 3, horizon: 28.0 },
		});
		if !app.is_plugin_added::<FirearmWeaponsPlugin>() {
			app.add_plugins(FirearmWeaponsPlugin);
		}
		if !app.is_plugin_added::<FirearmUserPlugin>() {
			app.add_plugins(FirearmUserPlugin);
		}
		if !app.is_plugin_added::<MovementIntelligencePlugin<AvianMovementSurface<'_, '_>>>() {
			app.add_plugins(MovementIntelligencePlugin::<AvianMovementSurface<'_, '_>>::default());
		}
		if !app.is_plugin_added::<FirearmIntelligencePlugin>() {
			app.add_plugins(FirearmIntelligencePlugin);
		}
		if !app.is_plugin_added::<ThreatIntelligencePlugin>() {
			app.add_plugins(ThreatIntelligencePlugin);
		}
		if !app.is_plugin_added::<ThreatIntelligenceDamagePlugin>() {
			app.add_plugins(ThreatIntelligenceDamagePlugin);
		}
		if !app.is_plugin_added::<ThreatManagementPlugin>() {
			app.add_plugins(ThreatManagementPlugin);
		}
		if !app.is_plugin_added::<EvasionPlugin>() {
			app.add_plugins(EvasionPlugin);
		}
		if !app.is_plugin_added::<FleeingPlugin>() {
			app.add_plugins(FleeingPlugin);
		}
		if !app.is_plugin_added::<HidingPlugin>() {
			app.add_plugins(HidingPlugin);
		}
		if !app.is_plugin_added::<MeanderingIntelligencePlugin>() {
			app.add_plugins(MeanderingIntelligencePlugin);
		}
		if !app.is_plugin_added::<RoutingPlugin>() {
			app.add_plugins(RoutingPlugin);
		}
		if !app.is_plugin_added::<MovementRealizationPlugin>() {
			app.add_plugins(MovementRealizationPlugin);
		}
		app.configure_sets(
			Update,
			SpottingSystems::Observe.run_if(on_timer(Duration::from_millis(125))),
		)
		.configure_sets(
			Update,
			FirearmIntelligenceSystems::Spotting.run_if(on_timer(Duration::from_millis(125))),
		)
		.configure_sets(
			Update,
			FirearmIntelligenceSystems::Movement.run_if(on_timer(Duration::from_millis(125))),
		)
		.configure_sets(
			Update,
			(EvasionSystems::Ingest, EvasionSystems::Rank)
				.chain()
				.after(SpottingSystems::Observe)
				.run_if(on_timer(Duration::from_millis(125))),
		)
		.configure_sets(
			Update,
			(FleeingSystems::Write, HidingSystems::Write)
				.chain()
				.after(EvasionSystems::Rank)
				.run_if(on_timer(Duration::from_millis(125))),
		)
		.configure_sets(Update, PoiSystems::Select.run_if(on_timer(Duration::from_millis(200))))
		.configure_sets(
			PostUpdate,
			FirearmIntelligenceSystems::ValidateAim.run_if(on_timer(Duration::from_millis(33))),
		)
		.configure_sets(
			PostUpdate,
			FirearmIntelligenceSystems::Fire.run_if(on_timer(Duration::from_millis(33))),
		)
		.configure_sets(
			PostUpdate,
			FirearmWeaponSystems::Fire.run_if(on_timer(Duration::from_millis(33))),
		);
	}
}
