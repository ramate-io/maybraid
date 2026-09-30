//! [`MobGenerationPlugin`]: which mob groups exist where, over urbanized ground `G`.

mod config;
mod generation;
mod index;
mod stream;

pub use config::MobLayerConfig;
pub use generation::{MobGenerationPlugin, MobGenerationSystems};
pub use index::{MobCell, MobIndex};
pub use stream::{MobLodChan, MobStreamSuspended};

#[cfg(test)]
mod tests;
