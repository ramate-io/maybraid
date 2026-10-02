//! Urbanization as a terrain model: `Urbanization<M>` is model `M` with
//! development pads composed in, plus the urban artifacts built on it.
//!
//! Storage stays in Richmond's [`DevelopmentEntryStore`](richmond_development_models::DevelopmentEntryStore)
//! and [`UrbanizationIndex`](richmond_urbanization::UrbanizationIndex); this
//! crate only declares the contract over them. [`UrbanizationGenerationPlugin`]
//! lives here because it is what brings `Urbanization<M>` into existence.

mod config;
mod generation;
mod model;
mod pads;
mod setting;
mod stream;
mod urban;

pub use config::{
	DevelopmentFocus, UrbanizationLayerConfig, UrbanizationSharedConfig, PLAYGROUND_LIKELIHOOD,
};
pub use generation::{
	UrbanizationGenerationCore, UrbanizationGenerationPlugin, UrbanizationGenerationSystems,
	UrbanizationLayerRegion, UrbanizationModeConfig, UrbanizationScheme,
	UrbanizationStoreSystems,
};
pub use model::{UrbanRead, UrbanSnapshot, Urbanization};
pub use pads::PadComposable;
pub use setting::UrbanSetting;
pub use stream::{
	clear_urbanization_stream,
	generate_urbanization_developments,
	generate_urbanization_padded_terrain,
	install_urbanization_stream,
	parse_urbanization_kind,
	prepare_development_cells,
	stream_radii_m,
	stream_urbanization,
	sync_urbanization_pin,
	urbanization_host_region,
	urbanization_visual_region,
	write_urbanization_host_region,
	UrbanizationStreamKey,
	UrbanizationStreamLod,
	UrbanizationStreamSpec,
	DEFAULT_URBANIZATION_NOISE,
	DEFAULT_URBANIZATION_STREAM_RADIUS,
};
pub use urban::UrbanModel;

#[cfg(test)]
mod tests;
