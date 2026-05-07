//! Generic **render dispatch**: anything can implement [`dispatch::RenderItem`], and
//! [`dispatch::DispatchRenderItem`] starts a handling chain whose responses are spawned as
//! **children** of the dispatching entity.
//!
//! Query wiring follows the same idea as cascade production’s
//! `CascadeProductionSource` in the `lod-cascade-system` crate:
//! you define [`dispatch::RenderDispatchSource`] with custom [`bevy::ecs::query::QueryData`] instead
//! of hard-coding `CascadeChunk` + `Transform`. LOD / chunk normalization live outside this crate.
//!
//! ## Modules
//!
//! - [`dispatch`] — [`DispatchRenderItem`], [`RenderItem`], [`RenderDispatchSource`], systems.
//! - [`mesh_sdf`] — [`SdfRenderContext`], [`SdfMeshPayload`], optional cuboid helper with `Assets<Mesh>`.
//! - [`wrappers`] — placeholder disk/cache wrapper types.
//! - [`plugin`] — [`RenderDispatchPlugin`].

pub mod dispatch;
pub mod mesh_sdf;
pub mod plugin;
pub mod wrappers;

pub use dispatch::{
	process_render_dispatches, process_render_dispatches_simple, DispatchRenderItem,
	RenderDispatchSource, RenderItem,
};
pub use mesh_sdf::{spawn_sdf_placeholder_cuboid_child, SdfMeshPayload, SdfRenderContext};
pub use plugin::RenderDispatchPlugin;

#[cfg(test)]
mod tests;
