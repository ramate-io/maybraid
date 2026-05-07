//! [`ChunkTracker`] reactor pattern for footprint signals from [`crate::cascade_production`].
//!
//! Matches [RFC-154 §3.3 `ChunkTracker`](https://github.com/ramate-io/maybraid/blob/main/rfc/rfc-000-000-154-generalized-lod/README.md#33-chunktracker)
//! / checklist §4.3 ([#160](https://github.com/ramate-io/maybraid/issues/160)).
//!
//! Signal entities carry [`CascadeChunk`], [`RequirementSignal`], flow payload data, and
//! [`CascadeProductionSignalMarker`]. Trackers run when [`RequirementSignal`] is inserted or updated.

use std::marker::PhantomData;

use bevy::prelude::*;
use lod_cascade::Chunk;

use crate::cascade_production::{
	produce_cascade, CascadeChunk, CascadeProductionSignalMarker, CascadeProductionSource,
	RequirementSignal,
};

/// Reacts to chunk requirement signals emitted by [`crate::cascade_production::CascadeProduction`].
///
/// Type parameter **`T`** is the implementing tracker type (typically a ZST); **`S`** identifies the
/// [`CascadeProductionSource`] flow whose signals this tracker listens to.
///
/// Keep [`ChunkTracker::react`] minimal: no extra query data, no return value (RFC-154 §3.3.5 design notes).
pub trait ChunkTracker<S>: Send + Sync + 'static
where
	S: CascadeProductionSource,
{
	fn react(commands: &mut Commands, chunk: Chunk, signal: RequirementSignal);
}

/// Runs [`ChunkTracker::react`] for entities whose [`RequirementSignal`] changed.
///
/// Filter matches [`CascadeProductionSignalMarker`]`<S>` so disjoint flows stay isolated.
pub fn track_chunks<T, S>(
	mut commands: Commands,
	signals: Query<
		(&CascadeChunk, &RequirementSignal),
		(With<CascadeProductionSignalMarker<S>>, Changed<RequirementSignal>),
	>,
) where
	S: CascadeProductionSource,
	T: ChunkTracker<S>,
{
	for (chunk, signal) in &signals {
		T::react(&mut commands, chunk.0, *signal);
	}
}

/// Registers [`track_chunks`] on [`Update`], ordered **after** [`produce_cascade`]`<S>` so new signals
/// from the same tick are visible.
pub struct ChunkTrackerPlugin<T, S>(PhantomData<(T, S)>);

impl<T, S> Default for ChunkTrackerPlugin<T, S> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T, S> Clone for ChunkTrackerPlugin<T, S> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T, S> Copy for ChunkTrackerPlugin<T, S> {}

impl<T, S> Plugin for ChunkTrackerPlugin<T, S>
where
	S: CascadeProductionSource,
	T: ChunkTracker<S>,
{
	fn build(&self, app: &mut App) {
		app.add_systems(Update, track_chunks::<T, S>.after(produce_cascade::<S>));
	}
}
