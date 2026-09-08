use bevy::prelude::*;
use intelligence_lod::IntelligencePriority;
use spotting_intelligence::SpottingSystems;

use crate::{
	discover_threats, export_threat_spotting_hints, ingest_threat_observations,
	sync_threat_registry, ThreatObservation, ThreatRegistry,
};

/// How many due recipients may run a local threat scan on one Discover tick.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThreatDiscoverLimits {
	pub max_scans_per_tick: usize,
}

impl Default for ThreatDiscoverLimits {
	fn default() -> Self {
		Self { max_scans_per_tick: 8 }
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThreatSystems {
	/// Application-owned actor affiliation setup runs here.
	Prepare,
	Index,
	Ingest,
	Discover,
	Export,
}

pub struct ThreatIntelligencePlugin;

impl Plugin for ThreatIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<ThreatRegistry>()
			.init_resource::<IntelligencePriority>()
			.init_resource::<ThreatDiscoverLimits>()
			.add_message::<ThreatObservation>()
			.configure_sets(
				Update,
				(
					ThreatSystems::Prepare,
					ThreatSystems::Index,
					ThreatSystems::Ingest,
					ThreatSystems::Discover,
					ThreatSystems::Export,
				)
					.chain(),
			)
			.configure_sets(Update, ThreatSystems::Export.before(SpottingSystems::Observe))
			.add_systems(Update, sync_threat_registry.in_set(ThreatSystems::Index))
			.add_systems(Update, ingest_threat_observations.in_set(ThreatSystems::Ingest))
			.add_systems(Update, discover_threats.in_set(ThreatSystems::Discover))
			.add_systems(Update, export_threat_spotting_hints.in_set(ThreatSystems::Export));
	}
}
