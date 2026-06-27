//! Trunkless radial shoot construction ([#225](https://github.com/ramate-io/maybraid/issues/225)).

mod assembly;
mod canopy;
mod config;
pub mod preset;
pub mod render_item_plugin;
mod stick;

pub use assembly::HighBushShoots;
pub use config::{HighBushFoliageStyle, HighBushShootsShape};
pub use preset::{
	apply_common_high_bush_preset, COMMON_HIGH_BUSH_RADIAL_STRENGTH, COMMON_HIGH_BUSH_SHOOT_COUNT,
	COMMON_HIGH_BUSH_VERTICAL_BIAS,
};
