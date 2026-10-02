//! Shared generation-mode state and layer-stack wiring.
//!
//! Layer crates take [`GenerationMode`] as a plugin parameter and never name a
//! concrete mode.

mod generation_mode;
mod require;
mod subscription;

pub use generation_mode::{
	in_generation_mode, ActiveGenerationMode, GenerationMode, GenerationModePlugin,
	GenerationModeSystems,
};
pub use require::RequireLayer;
pub use subscription::{mode_subscribed, subscribe_mode, ModeSubscribers, ModeSubscription};
