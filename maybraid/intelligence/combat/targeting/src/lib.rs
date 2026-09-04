//! Shared combat contact memory and target ranking.
//!
//! Perception adapters write [`CombatContact`] snapshots and semantic
//! [`TargetSource`] memberships. Policy adapters write [`TargetFactors`] and
//! [`TimedInfluence`] values. [`CombatTargetingPlugin`] performs time-aware
//! ranking without owning line of sight, faction policy, or weapon behavior.

mod algebra;
mod contact;
mod plugin;
mod source;
mod targeting;

pub use algebra::{TargetAlgebra, TargetFactor, TargetFactors};
pub use contact::CombatContact;
pub use plugin::{rank_combat_targets, CombatTargetingPlugin, CombatTargetingSystems};
pub use source::TargetSource;
pub use targeting::{ActiveTarget, CombatTargeting, RankedTarget, TimedInfluence};
