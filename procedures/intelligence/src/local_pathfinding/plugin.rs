//! Bevy plugin and system for per-entity local pathfinding.
//!
//! Add [`LocalPathfinding`](super::LocalPathfinding)`<F, S>` and a [`Transform`] to an entity, then
//! insert [`FindPath`] with a goal. On the first frame `FindPath` is present, the system runs
//! pathfinding using **that** `LocalPathfinding` instance, inserts [`LocalPathPlan`] if a best
//! partial path exists, and removes `FindPath`.

use std::marker::PhantomData;

use bevy::prelude::*;

use super::{LocalPath, LocalPathFindingFanout, LocalPathfinding, LocalPathfindingSurface};

/// Request a path from this entity’s [`Transform::translation`] to [`Self::to_position`].
///
/// Requires the same entity to have [`LocalPathfinding`]`<F, S>` for the `F`, `S` your app
/// registered via [`LocalPathfindingPlugin`].
#[derive(Component, Debug, Clone)]
pub struct FindPath {
	pub to_position: Vec3,
}

/// Lowest-cost partial path produced for a processed [`FindPath`] request.
#[derive(Component, Debug, Clone)]
pub struct LocalPathPlan {
	pub path: LocalPath,
	pub cost: f32,
}

/// Handles [`FindPath`] using the entity’s [`LocalPathfinding`] configuration (same pattern as
/// `render_item`’s `Added<_>` query: run once per new `FindPath`, then remove it).
pub fn respond_to_find_path_requests<F, S>(
	mut commands: Commands,
	query: Query<(Entity, &FindPath, &LocalPathfinding<F, S>, &Transform), Added<FindPath>>,
) where
	F: LocalPathFindingFanout + Clone + Send + Sync + 'static,
	S: LocalPathfindingSurface + Clone + Send + Sync + 'static,
{
	for (entity, find_path, pathfinder, transform) in &query {
		let paths = pathfinder.find_partial_paths(transform.translation, find_path.to_position);
		let best = paths.into_iter().min_by(|(_, a), (_, b)| a.total_cmp(b));
		if let Some((path, cost)) = best {
			commands.entity(entity).insert(LocalPathPlan { path, cost });
		}
		commands.entity(entity).remove::<FindPath>();
	}
}

/// Registers [`respond_to_find_path_requests`] for a concrete `F` / `S` pair.
///
/// Add one plugin per `(F, S)` you use as [`LocalPathfinding`] components, e.g.
/// `LocalPathfindingPlugin::<MyFanout, MySurface>::default()`.
pub struct LocalPathfindingPlugin<F, S> {
	_phantom: PhantomData<(F, S)>,
}

impl<F, S> Default for LocalPathfindingPlugin<F, S> {
	fn default() -> Self {
		Self { _phantom: PhantomData }
	}
}

impl<F, S> Plugin for LocalPathfindingPlugin<F, S>
where
	F: LocalPathFindingFanout + Clone + Send + Sync + 'static,
	S: LocalPathfindingSurface + Clone + Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		app.add_systems(Update, respond_to_find_path_requests::<F, S>);
	}
}
