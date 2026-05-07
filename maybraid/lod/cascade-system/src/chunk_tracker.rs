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
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum ChunkTrackerOrder {
	/// Register `track_chunks::<T, S>` with `.after(produce_cascade::<S>)`.
	#[default]
	AfterProduceCascade,
	/// Register with no explicit relation to `produce_cascade::<S>`.
	Unordered,
}

pub struct ChunkTrackerPlugin<T, S> {
	order: ChunkTrackerOrder,
	marker: PhantomData<(T, S)>,
}

impl<T, S> Default for ChunkTrackerPlugin<T, S> {
	fn default() -> Self {
		Self { order: ChunkTrackerOrder::default(), marker: PhantomData }
	}
}

impl<T, S> Clone for ChunkTrackerPlugin<T, S> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T, S> Copy for ChunkTrackerPlugin<T, S> {}

impl<T, S> ChunkTrackerPlugin<T, S> {
	pub fn with_order(order: ChunkTrackerOrder) -> Self {
		Self { order, marker: PhantomData }
	}
}

impl<T, S> Plugin for ChunkTrackerPlugin<T, S>
where
	S: CascadeProductionSource,
	T: ChunkTracker<S>,
{
	fn build(&self, app: &mut App) {
		match self.order {
			ChunkTrackerOrder::AfterProduceCascade => {
				app.add_systems(Update, track_chunks::<T, S>.after(produce_cascade::<S>));
			}
			ChunkTrackerOrder::Unordered => {
				app.add_systems(Update, track_chunks::<T, S>);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use anyhow::{anyhow, Result};

	use crate::cascade_production::{StandardFlow, StandardRequirement};

	use super::*;

	#[derive(Debug)]
	struct FlowAlpha;
	#[derive(Debug)]
	struct FlowBeta;

	type AlphaFlow = StandardFlow<FlowAlpha, StandardRequirement, ()>;
	type BetaFlow = StandardFlow<FlowBeta, StandardRequirement, ()>;

	#[derive(Component, Clone, Copy)]
	struct ObservedChunk(pub Chunk);
	#[derive(Component, Clone, Copy)]
	struct ObservedSignal(pub RequirementSignal);

	struct RecordingTracker;

	impl<S> ChunkTracker<S> for RecordingTracker
	where
		S: CascadeProductionSource,
	{
		fn react(commands: &mut Commands, chunk: Chunk, signal: RequirementSignal) {
			commands.spawn((ObservedChunk(chunk), ObservedSignal(signal)));
		}
	}

	fn spawned_observations(world: &mut World) -> Vec<(Chunk, RequirementSignal)> {
		world
			.query::<(&ObservedChunk, &ObservedSignal)>()
			.iter(world)
			.map(|(c, s)| (c.0, s.0))
			.collect()
	}

	fn flow_signal_entity<S: CascadeProductionSource>(
		world: &mut World,
		chunk: Chunk,
		signal: RequirementSignal,
	) -> Entity {
		world
			.spawn((CascadeChunk(chunk), signal, CascadeProductionSignalMarker::<S>::default()))
			.id()
	}

	fn first_observation(world: &mut World) -> Result<(Chunk, RequirementSignal)> {
		let all = spawned_observations(world);
		all.first()
			.copied()
			.ok_or_else(|| anyhow!("expected at least one recorded observation"))
	}

	#[test]
	fn track_chunks_reacts_only_when_requirement_signal_changes() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_systems(Update, track_chunks::<RecordingTracker, AlphaFlow>);

		let chunk = Chunk::from_min_max(Vec3::ZERO, Vec3::ONE, None);
		let signal_entity =
			flow_signal_entity::<AlphaFlow>(app.world_mut(), chunk, RequirementSignal::Visible);

		app.update();
		assert_eq!(spawned_observations(app.world_mut()).len(), 1);
		assert_eq!(first_observation(app.world_mut())?, (chunk, RequirementSignal::Visible),);

		app.update();
		assert_eq!(
			spawned_observations(app.world_mut()).len(),
			1,
			"unchanged RequirementSignal should not retrigger",
		);

		app.world_mut().entity_mut(signal_entity).insert(RequirementSignal::Hidden);
		app.update();
		assert_eq!(spawned_observations(app.world_mut()).len(), 2);

		Ok(())
	}

	#[test]
	fn track_chunks_isolated_by_flow_marker_type() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_systems(Update, track_chunks::<RecordingTracker, AlphaFlow>);

		let alpha_chunk = Chunk::from_min_max(Vec3::ZERO, Vec3::ONE, None);
		let beta_chunk =
			Chunk::from_min_max(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0), None);
		flow_signal_entity::<AlphaFlow>(app.world_mut(), alpha_chunk, RequirementSignal::Visible);
		flow_signal_entity::<BetaFlow>(app.world_mut(), beta_chunk, RequirementSignal::Visible);

		app.update();
		let seen = spawned_observations(app.world_mut());
		assert_eq!(seen.len(), 1);
		assert_eq!(seen[0], (alpha_chunk, RequirementSignal::Visible));
		Ok(())
	}

	#[test]
	fn chunk_tracker_plugin_unordered_mode_supports_headless_usage() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_plugins(ChunkTrackerPlugin::<RecordingTracker, AlphaFlow>::with_order(
			ChunkTrackerOrder::Unordered,
		));

		let chunk = Chunk::from_min_max(Vec3::ZERO, Vec3::ONE, None);
		flow_signal_entity::<AlphaFlow>(app.world_mut(), chunk, RequirementSignal::Remove);

		app.update();
		assert_eq!(spawned_observations(app.world_mut()).len(), 1);
		assert_eq!(first_observation(app.world_mut())?, (chunk, RequirementSignal::Remove),);
		Ok(())
	}
}
