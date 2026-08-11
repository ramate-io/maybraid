//! [`MaterialLib`]: resolve [`MaterialRef`] across one or more concrete material types.

use bevy::prelude::{Commands, Entity};

use crate::material_ref::MaterialRef;

/// Capability implemented by a [`bevy::ecs::system::SystemParam`] item that can
/// turn a [`MaterialRef`] into an inserted material component (any `M`).
///
/// Parallel to [`lod::LodSceneRegionIndex`]: the trait is the API; the implementor
/// is a `#[derive(SystemParam)]` struct that borrows only the `Assets` / caches it needs.
///
/// ```ignore
/// #[derive(SystemParam)]
/// pub struct ChicoMaterialLib<'w> {
///     leaf: ResMut<'w, Assets<ChicoLeafMaterial>>,
///     cache: ResMut<'w, MaterialRefCache<ChicoLeafMaterial>>,
/// }
///
/// impl MaterialLib for ChicoMaterialLib<'_> {
///     fn fulfill(&mut self, entity: Entity, r: &MaterialRef, commands: &mut Commands) {
///         // insert MeshMaterial3d::<ChicoLeafMaterial>(…)
///     }
/// }
/// ```
pub trait MaterialLib {
	/// Build or reuse a material for `material_ref` and insert it on `entity`.
	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands);
}
