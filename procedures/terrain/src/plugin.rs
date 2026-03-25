use crate::render::TerrainRenderItem;
use bevy::prelude::*;
use chunk::cascade::ResolutionMap;
use render_item::lod::LodPlugin;
use std::{hash::Hash, marker::PhantomData};

#[derive(Component)]
pub struct Terrain;

#[derive(Debug, Clone)]
pub struct TerrainPlugin<R: ResolutionMap + Send + Sync + 'static, M: Material> {
	__marker: PhantomData<(R, M)>,
}

impl<R: ResolutionMap + Send + Sync + 'static, M: Material> Default for TerrainPlugin<R, M> {
	fn default() -> Self {
		Self { __marker: PhantomData }
	}
}

impl<R: ResolutionMap + Send + Sync + 'static, M: Material> Plugin for TerrainPlugin<R, M>
where
	M::Data: PartialEq + Eq + Hash + Clone,
{
	fn build(&self, app: &mut App) {
		app.add_plugins(LodPlugin::<R, TerrainRenderItem<M>>::default());
	}
}
