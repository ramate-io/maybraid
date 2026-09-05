//! Entity-free High wish for a roster slot. The live body is not a `ChildOf` the host.

use std::sync::Arc;

use bevy::prelude::*;

/// Recipe handle stamped on a High stub. Clone is an Arc bump; no [`Entity`].
///
/// `T` is the plant scene recipe (typically `CharacterSceneRecipe`). This is not
/// a [`crate::Mob`] host and not a `LodScene`.
#[derive(Component, Clone, Debug)]
pub struct RosterRef<T: Send + Sync + 'static> {
	pub recipe: Arc<T>,
	pub slot: u16,
	pub offset: Vec3,
}

impl<T: Default + Send + Sync + 'static> Default for RosterRef<T> {
	fn default() -> Self {
		Self { recipe: Arc::new(T::default()), slot: 0, offset: Vec3::ZERO }
	}
}

impl<T: Send + Sync + 'static> RosterRef<T> {
	pub fn new(recipe: Arc<T>, slot: u16, offset: Vec3) -> Self {
		Self { recipe, slot, offset }
	}
}

impl<T: Send + Sync + 'static> PartialEq for RosterRef<T>
where
	T: PartialEq,
{
	fn eq(&self, other: &Self) -> bool {
		self.slot == other.slot && self.offset == other.offset && self.recipe == other.recipe
	}
}

/// Live body spawned from a [`RosterRef`] stub. Stored on the stub, not in the recipe.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RosterBinding {
	pub body: Entity,
	pub host: Entity,
	pub slot: u16,
}
