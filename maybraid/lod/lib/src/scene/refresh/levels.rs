//! Level production: region messages + spatial index → [`LodSceneRefreshLevel`].

mod produce;

pub use produce::{produce_lod_refresh_levels, LodSceneRefreshLevel, LodSceneRefreshLevelsPlugin};
