//! [`CascadeProduction`] / [`produce_cascade`] from [RFC-154 §3.2](https://github.com/ramate-io/maybraid/blob/main/rfc/rfc-000-000-154-generalized-lod/README.md#32-cascadeproduction)
//! ([issue #159](https://github.com/ramate-io/maybraid/issues/159)).

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

/// Producer component: geometry kernel plus chunk entity table ([RFC §3.2.1](https://github.com/ramate-io/maybraid/blob/main/rfc/rfc-000-000-154-generalized-lod/README.md#321-core-components)).
#[derive(Component)]
pub struct CascadeProduction<S: CascadeProductionSource> {
	pub cascade: Cascade,
	pub table: CascadeTable,
	marker: PhantomData<S>,
}

impl<S: CascadeProductionSource> CascadeProduction<S> {
	pub fn new(cascade: Cascade) -> Self {
		Self {
			cascade,
			table: CascadeTable::default(),
			marker: PhantomData,
		}
	}
}

/// Snapshot of focal bounds used to drive [`Cascade::new_chunks`].
#[derive(Component, Clone)]
pub struct CascadePosition<D: Component + Clone + Send + Sync + 'static> {
	pub previous: Option<CascadeBounds>,
	pub current: CascadeBounds,
	pub data: D,
}

/// Endemic chunk outcome carried as its own component on signal entities ([RFC §3.2.2](https://github.com/ramate-io/maybraid/blob/main/rfc/rfc-000-000-154-generalized-lod/README.md#322-requirement-signal)).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RequirementSignal {
	Visible,
	Hidden,
	Remove,
}

/// Requirement placed on chunk entities; exposes the desired [`RequirementSignal`].
pub trait CascadeRequirement: Component + Clone + Send + Sync + 'static {
	fn signal(&self) -> RequirementSignal;
}

/// Builds per-footprint requirements from the current [`CascadePosition`].
pub trait RequirementBuilder<R>: Component + Clone + Send + Sync + 'static
where
	R: CascadeRequirement,
{
	fn build<D>(&self, position: &CascadePosition<D>, chunk: Chunk) -> R
	where
		D: Component + Clone + Send + Sync + 'static;
}

/// Typed production flow: query wiring + bounds accessors ([RFC §3.2.4](https://github.com/ramate-io/maybraid/blob/main/rfc/rfc-000-000-154-generalized-lod/README.md#324-source-trait)).
///
/// Implement `QueryFilter` as `()` when no filter is needed.
pub trait CascadeProductionSource: Send + Sync + 'static {
	type PositionData: Component + Clone + Send + Sync + 'static;
	type Requirement: CascadeRequirement;
	type Builder: RequirementBuilder<Self::Requirement> + Default;
	type QueryData: QueryData;
	type QueryFilter: QueryFilter + Send + Sync + 'static;

	fn entity(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> Entity;

	fn current_position(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> CascadeBounds;

	fn position_data(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> Self::PositionData;
}

/// ZST marker on transient `(chunk, signal, …)` entities so garbage collection can target them ([RFC §3.2.5](https://github.com/ramate-io/maybraid/blob/main/rfc/rfc-000-000-154-generalized-lod/README.md#325-signal-entities)).
#[derive(Component)]
pub struct CascadeProductionSignalMarker<S>(PhantomData<S>);

impl<S> Default for CascadeProductionSignalMarker<S> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

/// Runs **`before`** [`produce_cascade`] ([RFC §3.2.13](https://github.com/ramate-io/maybraid/blob/main/rfc/rfc-000-000-154-generalized-lod/README.md#3213-garbage-collect-requirement-signals)).
pub fn garbage_collect_requirement_signals<S: CascadeProductionSource>(
	mut commands: Commands,
	signals: Query<
		Entity,
		(
			With<CascadeChunk>,
			With<RequirementSignal>,
			With<CascadeProductionSignalMarker<S>>,
		),
	>,
) {
	for entity in &signals {
		commands.entity(entity).despawn();
	}
}

/// Full production tick ([RFC §3.2.6](https://github.com/ramate-io/maybraid/blob/main/rfc/rfc-000-000-154-generalized-lod/README.md#326-system)).
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
	let new_chunks = production
		.cascade
		.new_chunks(position.previous, position.current);

	apply_requirements_to_new_chunks::<S>(
		commands,
		producer,
		item,
		production,
		position,
		builder,
		&new_chunks,
	);

	apply_requirements_to_existing_chunks::<S>(
		commands,
		item,
		production,
		position,
		builder,
	);
}

fn apply_requirements_to_new_chunks<S: CascadeProductionSource>(
	commands: &mut Commands,
	producer: Entity,
	item: &<S::QueryData as QueryData>::Item<'_, '_>,
	production: &mut CascadeProduction<S>,
	position: &CascadePosition<S::PositionData>,
	builder: &S::Builder,
	new_chunks: &[Chunk],
) {
	for &chunk in new_chunks {
		let requirement = builder.build(position, chunk);
		let signal = requirement.signal();

		match signal {
			RequirementSignal::Visible => {
				let chunk_entity = *production
					.table
					.table
					.entry(chunk)
					.or_insert_with(|| {
						let e = commands
							.spawn((CascadeChunk(chunk), Visibility::Visible))
							.id();
						commands.entity(producer).add_child(e);
						e
					});

				commands
					.entity(chunk_entity)
					.insert((requirement, Visibility::Visible));
			}
			RequirementSignal::Hidden => {
				let chunk_entity = *production
					.table
					.table
					.entry(chunk)
					.or_insert_with(|| {
						let e = commands
							.spawn((CascadeChunk(chunk), Visibility::Hidden))
							.id();
						commands.entity(producer).add_child(e);
						e
					});

				commands
					.entity(chunk_entity)
					.insert((requirement, Visibility::Hidden));

				spawn_requirement_signal::<S>(commands, item, chunk, signal);
			}
			RequirementSignal::Remove => {
				spawn_requirement_signal::<S>(commands, item, chunk, signal);
			}
		}
	}
}

fn apply_requirements_to_existing_chunks<S: CascadeProductionSource>(
	commands: &mut Commands,
	item: &<S::QueryData as QueryData>::Item<'_, '_>,
	production: &mut CascadeProduction<S>,
	position: &CascadePosition<S::PositionData>,
	builder: &S::Builder,
) {
	let existing: Vec<(Chunk, Entity)> = production
		.table
		.table
		.iter()
		.map(|(&chunk, &entity)| (chunk, entity))
		.collect();

	for (chunk, entity) in existing {
		let requirement = builder.build(position, chunk);
		let signal = requirement.signal();

		match signal {
			RequirementSignal::Visible => {
				commands
					.entity(entity)
					.insert((requirement, Visibility::Visible));
			}
			RequirementSignal::Hidden => {
				commands
					.entity(entity)
					.insert((requirement, Visibility::Hidden));

				spawn_requirement_signal::<S>(commands, item, chunk, signal);
			}
			RequirementSignal::Remove => {
				production.table.table.remove(&chunk);
				commands.entity(entity).despawn();
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

/// Registers [`garbage_collect_requirement_signals`] → [`produce_cascade`] on [`Update`] ([RFC §3.2.14](https://github.com/ramate-io/maybraid/blob/main/rfc/rfc-000-000-154-generalized-lod/README.md#3214-plugin)).
#[derive(Default)]
pub struct CascadeProductionPlugin<S>(PhantomData<S>);

impl<S: CascadeProductionSource> Plugin for CascadeProductionPlugin<S> {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(garbage_collect_requirement_signals::<S>, produce_cascade::<S>).chain(),
		);
	}
}
