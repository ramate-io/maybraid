//! [`ChunkEntityPosition`] and [`track_chunk_entities`] for re-parenting managed entities under
//! production chunk entities ([RFC-154 §3.4](https://github.com/ramate-io/maybraid/blob/main/rfc/rfc-000-000-154-generalized-lod/README.md#34-chunkentitytracker),
//! milestone [#161](https://github.com/ramate-io/maybraid/issues/161)).
//!
//! Expected hierarchy: **`CascadeProduction<S>` → `CascadeChunk` chunk → managed entity with `P`**.

use std::marker::PhantomData;

use bevy::prelude::*;
use lod_cascade::Chunk;

use crate::cascade_production::{
	CascadeChunk, CascadePosition, CascadeProduction, CascadeProductionSource, TrackBounds,
};

/// Spatial bounds component for entities parented under a [`CascadeChunk`] child of [`CascadeProduction`].
///
/// Default [`ChunkEntityPosition::select_chunk`] delegates to [`select_best_overlapping_chunk`].
pub trait ChunkEntityPosition<S>: Component
where
	S: CascadeProductionSource,
{
	fn previous(&self) -> Option<TrackBounds>;

	fn current(&self) -> TrackBounds;

	fn select_chunk(
		&self,
		current_parent_chunk: Chunk,
		production: &CascadeProduction<S>,
		position: &CascadePosition<S::PositionData>,
	) -> Option<Entity>
	where
		S: CascadeProductionSource,
	{
		select_best_overlapping_chunk::<S>(
			current_parent_chunk,
			production,
			position,
			self.previous(),
			self.current(),
		)
	}
}

/// Level-preserving chunk pick: among candidates from [`LodCascade::all_possible_new_chunks`], keep
/// footprints whose characteristic size matches the current parent when the cascade has rings
/// ([RFC-154 §3.4.4](https://github.com/ramate-io/maybraid/blob/main/rfc/rfc-000-000-154-generalized-lod/README.md#344-default-chunk-selection)).
///
/// Chooses the table entity with maximum **overlap volume** between the candidate [`Chunk`] and
/// `current` bounds (this crate’s `Chunk` API exposes [`Chunk::overlap_volume`] rather than area).
pub fn select_best_overlapping_chunk<S>(
	current_parent_chunk: Chunk,
	production: &CascadeProduction<S>,
	_position: &CascadePosition<S::PositionData>,
	previous: Option<TrackBounds>,
	current: TrackBounds,
) -> Option<Entity>
where
	S: CascadeProductionSource,
{
	let cascade = production.cascade;
	let candidates = cascade.all_possible_new_chunks(previous, current);

	let parent_size = current_parent_chunk.max_extent_component();

	candidates
		.into_iter()
		.filter(|candidate| {
			cascade.ring_count() == 0 || candidate.max_extent_component() == parent_size
		})
		.filter_map(|chunk| {
			let entity = production.table.table.get(&chunk).copied()?;
			let overlap = chunk.overlap_volume(&current);
			Some((entity, overlap))
		})
		.max_by(|(_, a), (_, b)| a.total_cmp(b))
		.map(|(entity, _)| entity)
}

/// Re-parents or despawns managed entities when their [`ChunkEntityPosition`] changes.
///
/// Join: managed entity → [`ChildOf`] chunk → [`ChildOf`] production.
pub fn track_chunk_entities<P, S>(
	mut commands: Commands,
	managed: Query<(Entity, &P, &ChildOf), Changed<P>>,
	chunks: Query<(&CascadeChunk, &ChildOf)>,
	productions: Query<(&CascadeProduction<S>, &CascadePosition<S::PositionData>)>,
) where
	S: CascadeProductionSource,
	P: ChunkEntityPosition<S>,
{
	for (entity, chunk_entity_position, child_of_chunk) in &managed {
		let current_parent_chunk_entity = child_of_chunk.parent();

		let Ok((current_parent_cascade_chunk, child_of_production)) =
			chunks.get(current_parent_chunk_entity)
		else {
			commands.entity(entity).despawn();
			continue;
		};

		let production_entity = child_of_production.parent();
		let Ok((production, cascade_position)) = productions.get(production_entity) else {
			commands.entity(entity).despawn();
			continue;
		};

		let current_parent_chunk = current_parent_cascade_chunk.0;

		match chunk_entity_position.select_chunk(current_parent_chunk, production, cascade_position)
		{
			Some(new_chunk_entity) => {
				if new_chunk_entity != current_parent_chunk_entity {
					commands.entity(new_chunk_entity).add_child(entity);
				}
			}
			None => {
				commands.entity(entity).despawn();
			}
		}
	}
}

/// Registers [`track_chunk_entities`] on [`Update`].
pub struct ChunkEntityTrackerPlugin<P, S> {
	marker: PhantomData<(P, S)>,
}

impl<P, S> Default for ChunkEntityTrackerPlugin<P, S> {
	fn default() -> Self {
		Self { marker: PhantomData }
	}
}

impl<P, S> Clone for ChunkEntityTrackerPlugin<P, S> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<P, S> Copy for ChunkEntityTrackerPlugin<P, S> {}

impl<P, S> Plugin for ChunkEntityTrackerPlugin<P, S>
where
	S: CascadeProductionSource,
	P: ChunkEntityPosition<S>,
{
	fn build(&self, app: &mut App) {
		app.add_systems(Update, track_chunk_entities::<P, S>);
	}
}
