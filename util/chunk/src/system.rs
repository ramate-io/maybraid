use crate::cascade::{Cascade, CascadeChunk, ResolutionMap};
use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Component, Debug)]
pub struct Lod;

#[derive(Component, Debug)]
pub struct LodChild;

#[derive(Component, Debug)]
pub struct Record {
	pub transform: Transform,
}

pub struct LodPlugin<R: ResolutionMap + Send + Sync + 'static> {
	__marker: PhantomData<R>,
}

impl<R: ResolutionMap + Send + Sync + 'static> Default for LodPlugin<R> {
	fn default() -> Self {
		Self { __marker: PhantomData }
	}
}

impl<R: ResolutionMap + Send + Sync + 'static> LodPlugin<R> {
	/// Dispatches intent to record for the given entity and position.
	pub fn compute_lod_chunks(
		mut commands: Commands,
		parent_query: Query<
			(Entity, &Lod, &Cascade<R>, &Transform, &Children, Option<&Record>),
			Changed<Transform>,
		>,
		children_query: Query<&CascadeChunk, With<LodChild>>,
	) {
		for (entity, _lod, cascade, transform, children, record) in parent_query.iter() {
			// Handle the new chunks and cull the old chunks
			if let Some(record) = record {
				// Short circuit if the chunks are the same.
				if !cascade.needs_new_chunks(record.transform.translation, transform.translation) {
					continue;
				}

				if let Ok((new_chunks, all_chunks)) =
					cascade.new_chunks(record.transform.translation, transform.translation)
				{
					// Spawn the chunks that didn't appear before.
					for chunk in new_chunks.all() {
						commands.entity(entity).with_children(|parent| {
							parent.spawn((chunk, LodChild));
						});
					}

					// Despawn the children that are not in any of the chunks.
					for child in children.iter() {
						if let Ok(child_chunk) = children_query.get(child) {
							if !all_chunks.contains(child_chunk) {
								commands.entity(child).despawn();
							}
						}
					}
				} else {
					log::error!("Failed to get new chunks");
				}
			} else {
				if let Ok(new_chunks) = cascade.chunks(transform.translation) {
					// spawn the new chunks as children of the parent
					for chunk in new_chunks.all() {
						commands.entity(entity).with_children(|parent| {
							parent.spawn((chunk, LodChild));
						});
					}
				} else {
					log::error!("Failed to get new chunks");
				}
			}
		}
	}
}

impl<R: ResolutionMap + Send + Sync + 'static> Plugin for LodPlugin<R> {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, Self::compute_lod_chunks);
	}
}
