//! Shared generation-mode state and layer-stack wiring.
//!
//! Layer crates take [`GenerationMode`] as a plugin parameter and never name a
//! concrete mode.

mod generation_mode;
mod present_gate;
mod require;
mod subscription;

pub use generation_mode::{
	in_generation_mode, ActiveGenerationMode, GenerationMode, GenerationModePlugin,
	GenerationModeSystems,
};
pub use present_gate::{
	install_lod_present_gate, sync_lod_present_gate, LodPresentGatePlugin, LodPresentGateSync,
};
pub use require::RequireLayer;
pub use subscription::{mode_subscribed, subscribe_mode, ModeSubscribers, ModeSubscription};
