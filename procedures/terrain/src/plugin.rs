use crate::TerrainSdf;
use bevy::prelude::*;
use chunk::{
	cascade::{Cascade, CascadeChunk, ResolutionMap},
	system::{LodChild, LodPlugin},
};
use render_item::DispatchRenderItem;

#[derive(Component)]
pub struct Terrain;

#[derive(Debug, Clone)]
pub struct TerrainPlugin<R: ResolutionMap + Send + Sync + 'static> {
	pub seed: u32,
	pub cascade: Cascade<R>,
}

impl<R: ResolutionMap + Send + Sync + 'static> TerrainPlugin<R> {
	pub fn new(seed: u32, cascade: Cascade<R>) -> Self {
		Self { seed, cascade }
	}

	pub fn compute_lod_chunks(
		mut commands: Commands,
		parent_query: Query<
			(Entity, &DispatchRenderItem<TerrainSdf>, &Cascade<R>, &Children, &Transform),
			(With<Terrain>, Added<DispatchRenderItem<TerrainSdf>>),
		>,
		children_query: Query<Entity, (With<LodChild>, With<CascadeChunk>)>,
	) {
		// for each parent
		for (_entity, dispatch, _cascade, children, transform) in parent_query.iter() {
			// for each child
			for child in children.iter() {
				// if the child has the CascadeChunk and LodChild components
				if let Ok(child_entity) = children_query.get(child) {
					// First insert the render item, the cascade chunk should already be inserted.
					commands
						.entity(child_entity)
						.insert(DispatchRenderItem::new(dispatch.item().clone()));

					// Then insert the transform as it is needed for the render item system to trigger
					// For now, the transform will largely be irrelevant.
					commands.entity(child_entity).insert(transform.clone());
				}
			}
		}
	}
}

impl<R: ResolutionMap + Send + Sync + 'static> Plugin for TerrainPlugin<R> {
	fn build(&self, app: &mut App) {
		app.add_plugins(LodPlugin::<R>::default());
	}
}
