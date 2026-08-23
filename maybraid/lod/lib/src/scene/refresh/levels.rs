//! Level production: region messages + spatial index → [`LodSceneRefreshLevel`].

mod produce;

pub use produce::{
	fill_lod_produce_cache, produce_lod_refresh_levels, LodProduceCache, LodSceneRefreshAabb,
	LodSceneRefreshLevel, LodSceneRefreshLevelsFillPlugin, LodSceneRefreshLevelsPlugin,
};
