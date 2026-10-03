//! Shared generation-mode state and layer-stack wiring.
//!
//! Layer crates take [`GenerationMode`] as a plugin parameter and never name a
//! concrete mode. [`Generate`] and [`Present`] are keyed by a [`Layer`] wrapper.

mod generate;
mod generation_mode;
mod layer;
mod present;
mod present_gate;
mod require;
mod subscription;

pub use generate::Generate;
pub use generation_mode::{
	in_generation_mode, ActiveGenerationMode, GenerationMode, GenerationModePlugin,
	GenerationModeSystems, GenerationReadiness,
};
pub use layer::{
	register_layer_label, Layer, LayerGenerationCore, LayerModeConfig, LayerPresentation,
	LayerPresentationCore, LayerSystems, Scheme,
};
pub use present::Present;
pub use present_gate::{
	install_lod_present_gate, sync_lod_present_gate, LodPresentGatePlugin, LodPresentGateSync,
};
pub use require::RequireLayer;
pub use subscription::{mode_subscribed, subscribe_mode, ModeSubscribers, ModeSubscription};

#[cfg(test)]
mod layer_tests;
