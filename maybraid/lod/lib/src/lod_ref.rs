use bevy::prelude::*;
use lod_cascade::Aabb3d;
use std::marker::PhantomData;

/// A component type to mark fine LOD.
/// This enables archetype filtering to ignore a lot of entities.
#[derive(Debug, Component)]
pub struct FineLod;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodRequest {
	/// Details with cells of the immediate neighborhood.
	Fine,
	/// Details outside the immediate neighborhood but that are still required for gameplay.
	Coarse,
	/// Details that are meant to be prepared for future use.
	Warm,
}

#[derive(Debug, Clone)]
pub struct LodRef<'a, T> {
	/// The entity that triggered the LOD change.
	///
	/// For the most part, the developer will not use this.
	/// However, for more complex use cases without well-defined systems, this can be used to lookup the original entity
	/// and perform bespoke logic.
	pub entity: Entity,
	/// The previous transform of the entity that triggered the LOD change.
	pub previous_transform: &'a Transform,
	/// The transform of the entity that triggered the LOD change.
	pub current_transform: &'a Transform,
	/// The bounds of the entity that triggered the LOD change.
	pub bounds: &'a Aabb3d,
	marker: PhantomData<T>,
}
