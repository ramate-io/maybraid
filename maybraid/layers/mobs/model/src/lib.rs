//! [`MobGenerationPlugin`]: which mob groups exist where, over urbanized ground `G`.

mod config;
mod generation;
mod index;
mod stream;

pub use config::MobLayerConfig;
pub use generation::{
	MobGenerationCore, MobGenerationPlugin, MobGenerationSystems, MobScheme,
};
pub use index::{MobCell, MobCellExtent, MobIndex};
pub use stream::{announce_mob_cell, install_mob_grid_stream, MobLodChan};

#[cfg(test)]
mod tests;
