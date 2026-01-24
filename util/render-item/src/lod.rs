use crate::{render_items, DispatchRenderItem, RenderItem};
use bevy::prelude::*;
use chunk::cascade::{Cascade, CascadeChunk, ResolutionMap};
use std::marker::PhantomData;

#[derive(Component, Debug)]
pub struct Lod;

#[derive(Component, Debug)]
pub struct LodChild;

#[derive(Component, Debug)]
pub struct Record {
	pub transform: Transform,
}

pub struct LodPlugin<R: ResolutionMap + Send + Sync + 'static, I: RenderItem + Send + 'static> {
	__marker: PhantomData<(R, I)>,
}

impl<R: ResolutionMap + Send + Sync + 'static, I: RenderItem + Send + Sync + 'static> Default
	for LodPlugin<R, I>
{
	fn default() -> Self {
		Self { __marker: PhantomData }
	}
}

impl<R: ResolutionMap + Send + Sync + 'static, I: RenderItem + Send + Sync + 'static>
	LodPlugin<R, I>
{
	/// Dispatches intent to record for the given entity and position.
	pub fn compute_lod_chunks(
		mut commands: Commands,
		parent_query: Query<
			(
				Entity,
				&DispatchRenderItem<I>,
				&Cascade<R>,
				&Transform,
				Option<&Children>,
				Option<&Record>,
			),
			(With<Lod>, Changed<Transform>),
		>,
		children_query: Query<&CascadeChunk, With<LodChild>>,
	) {
		for (entity, render_item, cascade, transform, children, record) in parent_query.iter() {
			log::info!("Computing lod chunks for entity: {}", entity);
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
						log::info!("chunk: {:?}", chunk);
						commands.entity(entity).with_children(|parent| {
							parent.spawn((LodChild, chunk, render_item.clone(), transform.clone()));
						});
					}

					// Despawn the children that are not in any of the chunks.
					if let Some(children) = children {
						for child in children.iter() {
							if let Ok(child_chunk) = children_query.get(child) {
								if !all_chunks.contains(child_chunk) {
									commands.entity(child).despawn();
								}
							}
						}
					}
				} else {
					log::error!("Failed to get new chunks");
				}
			} else {
				if let Ok(new_chunks) = cascade.chunks(transform.translation) {
					log::info!("Spawning new chunks for entity: {}", entity);
					// spawn the new chunks as children of the parent
					for chunk in new_chunks.all() {
						log::info!("Spawning new chunk {:?}", chunk);
						commands.entity(entity).with_children(|parent| {
							parent.spawn((LodChild, chunk, render_item.clone(), transform.clone()));
						});
					}
				} else {
					log::error!("Failed to get new chunks");
				}
			}

			commands.entity(entity).insert(Record { transform: transform.clone() });
		}
	}
}

impl<R: ResolutionMap + Send + Sync + 'static, I: RenderItem + Send + Sync + 'static> Plugin
	for LodPlugin<R, I>
{
	fn build(&self, app: &mut App) {
		app.add_systems(Update, Self::compute_lod_chunks);
		app.add_systems(Update, render_items::<I>);
	}
}
