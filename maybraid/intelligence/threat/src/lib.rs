//! Retained local threat discovery gated by directional affiliation memory.
//!
//! Threat knowledge is semantic candidate memory, not proof of visibility.
//! [`export_threat_spotting_hints`] feeds candidates into spotting, which still
//! owns sightline probes and visual contact memory.

mod affiliation;
mod discovery;
mod id;
mod knowledge;
mod plugin;
mod registry;
mod source;
mod subject;

pub use affiliation::{AffiliationStrength, Affiliations};
pub use discovery::{
	discover_threats, export_threat_spotting_hints, ingest_threat_observations,
	sync_threat_registry,
};
pub use id::{ThreatGroupId, ThreatId};
pub use knowledge::{
	KnownThreat, ThreatDiscoveryPolicy, ThreatIntelligenceUser, ThreatKnowledge, ThreatObservation,
};
pub use plugin::{ThreatIntelligencePlugin, ThreatSystems};
pub use registry::{ThreatRecord, ThreatRegistry};
pub use source::ThreatSource;
pub use subject::ThreatSubject;

#[cfg(test)]
mod tests;
