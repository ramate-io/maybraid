//! Assailant knowledge and exclusive hide | flee routing.
//!
//! Perception adapters write [`AssailantContact`] snapshots and semantic
//! [`AssailantSource`] memberships. [`EvasionPlugin`] ranks them and emits
//! [`EvasionSignal`]. Fleeing and hiding consume that signal; this crate does
//! not write movement objectives.

mod algebra;
mod contact;
mod plugin;
mod signal;
mod source;
mod spotting;
mod user;

pub use algebra::{AssailantAlgebra, AssailantFactor, AssailantFactors};
pub use contact::AssailantContact;
pub use plugin::{rank_assailants, EvasionPlugin, EvasionSystems};
pub use signal::{EvasionActuator, EvasionSignal};
pub use source::AssailantSource;
pub use spotting::sync_spotted_assailants;
pub use user::{
	ActiveAssailant, EvasionIntelligenceUser, EvasionSettings, RankedAssailant, TimedInfluence,
};
