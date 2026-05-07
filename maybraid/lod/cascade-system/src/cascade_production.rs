//! [`CascadeProduction`] / [`produce_cascade`]: geometry deltas (**[`lod_cascade::Cascade::new_chunks`]** /
//! **[`lod_cascade::Cascade::expired_chunks`]**) plus a context-aware [`RequirementBuilder`].

use std::collections::HashMap;
use std::marker::PhantomData;

use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod_cascade::{Cascade, Chunk};

/// Axis-aligned bounds used for cascade motion (`AaBb` in the RFC).
pub type CascadeBounds = Aabb3d;

/// [`Chunk`](lod_cascade::Chunk) footprint stored on entities (production chunks and transient signals).
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CascadeChunk(pub Chunk);

impl std::ops::Deref for CascadeChunk {
	type Target = Chunk;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// Maps footprint keys to spawned chunk entities owned under a producer.
#[derive(Clone, Default, Debug)]
pub struct CascadeTable {
	pub table: HashMap<Chunk, Entity>,
}

/// Producer component: geometry kernel plus chunk entity table.
#[derive(Component)]
pub struct CascadeProduction<S: CascadeProductionSource> {
	pub cascade: Cascade,
	pub table: CascadeTable,
	marker: PhantomData<S>,
}

impl<S: CascadeProductionSource> CascadeProduction<S> {
	pub fn new(cascade: Cascade) -> Self {
		Self { cascade, table: CascadeTable::default(), marker: PhantomData }
	}
}

/// Snapshot of focal bounds used to drive cascade deltas.
#[derive(Component, Clone)]
pub struct CascadePosition<D: Component + Clone + Send + Sync + 'static> {
	pub previous: Option<CascadeBounds>,
	pub current: CascadeBounds,
	pub data: D,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RequirementSignal {
	Visible,
	Hidden,
	Remove,
}

/// Policy for footprint transitions. Only **`signal_for_new`** and **`signal_for_expired`** run each tick;
/// chunks that remain inside both snapshots are left unchanged.
pub trait RequirementBuilder: Component + Clone + Send + Sync + 'static + Default {
	fn signal_for_new<D: Component + Clone + Send + Sync + 'static>(
		&self,
		_cascade: &Cascade,
		_position: &CascadePosition<D>,
		_chunk: Chunk,
	) -> RequirementSignal {
		RequirementSignal::Visible
	}

	fn signal_for_expired<D: Component + Clone + Send + Sync + 'static>(
		&self,
		_cascade: &Cascade,
		_position: &CascadePosition<D>,
		_chunk: Chunk,
	) -> RequirementSignal {
		RequirementSignal::Remove
	}
}

/// Typed production flow: query wiring + bounds accessors.
///
/// Implement `QueryFilter` as `()` when no filter is needed.
pub trait CascadeProductionSource: Send + Sync + 'static {
	type PositionData: Component + Clone + Send + Sync + 'static;
	type Builder: RequirementBuilder + Default;
	type QueryData: QueryData;
	type QueryFilter: QueryFilter + Send + Sync + 'static;

	fn entity(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> Entity;

	fn current_position(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> CascadeBounds;

	fn position_data(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> Self::PositionData;
}

/// ZST marker on transient signal entities so garbage collection can target them.
#[derive(Component)]
pub struct CascadeProductionSignalMarker<S>(PhantomData<S>);

impl<S> Default for CascadeProductionSignalMarker<S> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

/// Runs **`before`** [`produce_cascade`].
pub fn garbage_collect_requirement_signals<S: CascadeProductionSource>(
	mut commands: Commands,
	signals: Query<
		Entity,
		(With<CascadeChunk>, With<RequirementSignal>, With<CascadeProductionSignalMarker<S>>),
	>,
) {
	for entity in &signals {
		commands.entity(entity).despawn();
	}
}

pub fn produce_cascade<S: CascadeProductionSource>(
	mut commands: Commands,
	mut query: Query<
		(
			S::QueryData,
			&mut CascadeProduction<S>,
			Option<&CascadePosition<S::PositionData>>,
			Option<&S::Builder>,
		),
		S::QueryFilter,
	>,
) {
	for (item, mut production, old_position, builder) in query.iter_mut() {
		let entity = S::entity(&item);

		let position = update_cascade_position::<S>(&mut commands, entity, &item, old_position);

		let builder = resolve_requirement_builder::<S>(&mut commands, entity, builder);

		update_cascade_chunks::<S>(
			&mut commands,
			entity,
			&item,
			&mut production,
			&position,
			&builder,
		);
	}
}

fn update_cascade_position<S: CascadeProductionSource>(
	commands: &mut Commands,
	entity: Entity,
	item: &<S::QueryData as QueryData>::Item<'_, '_>,
	old_position: Option<&CascadePosition<S::PositionData>>,
) -> CascadePosition<S::PositionData> {
	let current = S::current_position(item);

	let position = CascadePosition {
		previous: old_position.map(|old| old.current),
		current,
		data: S::position_data(item),
	};

	commands.entity(entity).insert(position.clone());
	position
}

fn resolve_requirement_builder<S: CascadeProductionSource>(
	commands: &mut Commands,
	entity: Entity,
	builder: Option<&S::Builder>,
) -> S::Builder {
	match builder {
		Some(builder) => builder.clone(),
		None => {
			let builder = S::Builder::default();
			commands.entity(entity).insert(builder.clone());
			builder
		}
	}
}

fn update_cascade_chunks<S: CascadeProductionSource>(
	commands: &mut Commands,
	producer: Entity,
	item: &<S::QueryData as QueryData>::Item<'_, '_>,
	production: &mut CascadeProduction<S>,
	position: &CascadePosition<S::PositionData>,
	builder: &S::Builder,
) {
	let cascade = production.cascade;

	let expired_chunks = cascade.expired_chunks(position.previous, position.current);
	apply_expired_chunks::<S>(commands, item, production, position, builder, &expired_chunks);

	let new_chunks = cascade.new_chunks(position.previous, position.current);
	apply_new_chunks::<S>(commands, producer, item, production, position, builder, &new_chunks);
}

fn apply_expired_chunks<S: CascadeProductionSource>(
	commands: &mut Commands,
	item: &<S::QueryData as QueryData>::Item<'_, '_>,
	production: &mut CascadeProduction<S>,
	position: &CascadePosition<S::PositionData>,
	builder: &S::Builder,
	expired_chunks: &[Chunk],
) {
	for &chunk in expired_chunks {
		let Some(chunk_entity) = production.table.table.get(&chunk).copied() else {
			continue;
		};

		let signal = builder.signal_for_expired(&production.cascade, position, chunk);

		match signal {
			RequirementSignal::Visible => {
				commands.entity(chunk_entity).insert(Visibility::Visible);
			}
			RequirementSignal::Hidden => {
				commands.entity(chunk_entity).insert(Visibility::Hidden);
				spawn_requirement_signal::<S>(commands, item, chunk, signal);
			}
			RequirementSignal::Remove => {
				production.table.table.remove(&chunk);
				commands.entity(chunk_entity).despawn();
				spawn_requirement_signal::<S>(commands, item, chunk, signal);
			}
		}
	}
}

fn apply_new_chunks<S: CascadeProductionSource>(
	commands: &mut Commands,
	producer: Entity,
	item: &<S::QueryData as QueryData>::Item<'_, '_>,
	production: &mut CascadeProduction<S>,
	position: &CascadePosition<S::PositionData>,
	builder: &S::Builder,
	new_chunks: &[Chunk],
) {
	for &chunk in new_chunks {
		let signal = builder.signal_for_new(&production.cascade, position, chunk);

		match signal {
			RequirementSignal::Visible => {
				let chunk_entity = *production.table.table.entry(chunk).or_insert_with(|| {
					let e = commands.spawn((CascadeChunk(chunk), Visibility::Visible)).id();
					commands.entity(producer).add_child(e);
					e
				});

				commands.entity(chunk_entity).insert(Visibility::Visible);
			}
			RequirementSignal::Hidden => {
				let chunk_entity = *production.table.table.entry(chunk).or_insert_with(|| {
					let e = commands.spawn((CascadeChunk(chunk), Visibility::Hidden)).id();
					commands.entity(producer).add_child(e);
					e
				});

				commands.entity(chunk_entity).insert(Visibility::Hidden);

				spawn_requirement_signal::<S>(commands, item, chunk, signal);
			}
			RequirementSignal::Remove => {
				spawn_requirement_signal::<S>(commands, item, chunk, signal);
			}
		}
	}
}

fn spawn_requirement_signal<S: CascadeProductionSource>(
	commands: &mut Commands,
	item: &<S::QueryData as QueryData>::Item<'_, '_>,
	chunk: Chunk,
	signal: RequirementSignal,
) {
	let payload = S::position_data(item);
	commands.spawn((
		CascadeChunk(chunk),
		signal,
		payload,
		CascadeProductionSignalMarker::<S>::default(),
	));
}

pub struct CascadeProductionPlugin<S>(PhantomData<S>);

impl<S> Default for CascadeProductionPlugin<S> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<S: CascadeProductionSource> Plugin for CascadeProductionPlugin<S> {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(garbage_collect_requirement_signals::<S>, produce_cascade::<S>).chain(),
		);
	}
}

mod standard_marker;
mod standard_requirement;

pub use standard_marker::{StandardBounds, StandardFlow, StandardMarker};
pub use standard_requirement::StandardRequirement;

#[cfg(test)]
mod tests;
