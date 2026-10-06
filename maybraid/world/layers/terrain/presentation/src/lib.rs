//! Terrain presentation helpers. Install goes through [`layer_stack::Present`].

/// Installs cell presentation for a terrain model. Models own the hook;
/// this marker remains for Durham's raw-cell type.
pub trait TerrainPresenter<M: terrain_layer_model::TerrainModel>: Send + Sync + 'static {
	fn install(app: &mut bevy::app::App);
}

#[cfg(test)]
mod tests;
