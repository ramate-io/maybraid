//! Registers Bikanwarda combat and optional occupancy rule extension.

use bevy::prelude::*;
use lubikans::{LubikanCatalog, LubikanInterfacePlugin, LubikanSelfConfig, LubikansPlugin};
use occupancy_rewards::RewardRuleSet;

use crate::combat;
use crate::identity::{enraged_definition, ward_definition};
use crate::occupancy;

/// Bikanwarda charge rules ([#1136](https://github.com/ramate-io/maybraid/issues/1136)).
pub struct BikanwardaLubikansPlugin {
	/// When true, push [`occupancy::forest_banyan_rule`] onto [`RewardRuleSet`] if present.
	pub extend_occupancy: bool,
}

impl Default for BikanwardaLubikansPlugin {
	fn default() -> Self {
		Self { extend_occupancy: true }
	}
}

impl Plugin for BikanwardaLubikansPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LubikanInterfacePlugin>() {
			app.add_plugins(LubikanInterfacePlugin);
		}
		if !app.is_plugin_added::<LubikansPlugin>() {
			app.add_plugins(LubikansPlugin);
		}
		register_catalog(app);
		combat::configure(app);
		if self.extend_occupancy {
			app.add_systems(Startup, extend_occupancy_rules_on_startup);
		}
	}
}

fn register_catalog(app: &mut App) {
	let ward = ward_definition();
	let enraged = enraged_definition();
	if let Some(mut catalog) = app.world_mut().get_resource_mut::<LubikanCatalog>() {
		catalog.register(ward);
		catalog.register(enraged);
	} else {
		let config = app
			.world()
			.get_resource::<LubikanSelfConfig>()
			.copied()
			.unwrap_or_default();
		let mut catalog = LubikanCatalog::with_self(&config);
		catalog.register(ward);
		catalog.register(enraged);
		app.insert_resource(catalog);
	}
}

fn extend_occupancy_rules_on_startup(rules: Option<ResMut<RewardRuleSet>>) {
	if let Some(mut rules) = rules {
		occupancy::extend_occupancy_rules(&mut *rules);
	}
}

impl BikanwardaLubikansPlugin {
	/// Merge the forest Banyan rule into an existing set (world / tests).
	pub fn extend_occupancy_rules(rules: &mut RewardRuleSet) {
		occupancy::extend_occupancy_rules(rules);
	}
}
