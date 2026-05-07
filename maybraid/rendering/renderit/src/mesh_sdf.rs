//! SDF-oriented **context** and payload for the mesh pathway.
//!
//! A full `Assets<Mesh>` pipeline belongs in a system with `ResMut<Assets<Mesh>>` (see
//! [`spawn_sdf_placeholder_cuboid_child`]). [`SdfMeshPayload`] implements [`RenderItem`] by
//! spawning a small **placeholder** hierarchy under the dispatch entity so dispatch ordering and
//! parenting stay testable without pulling mesh resources into the trait.

use bevy::math::bounding::{Aabb3d, BoundingVolume};
use bevy::prelude::*;
use sdf::Sdf;

use crate::dispatch::RenderItem;

/// Context for [`SdfMeshPayload`]: axis-aligned bounds, sampling resolution, optional omission hull.
pub trait SdfRenderContext: Send + Sync {
	fn render_bounds_aabb(&self) -> Aabb3d;

	/// Intended voxel / sample spacing along the longest axis (used once real mesher lands).
	fn mesh_resolution(&self) -> f32;

	/// Optional region carved out of sampling (difference against the primary SDF); reserved for meshing.
	fn omission_aabb(&self) -> Option<Aabb3d> {
		None
	}
}

/// Bundles an [`Sdf`] with a [`MeshMaterial3d`] for mesh output.
#[derive(Clone)]
pub struct SdfMeshPayload<S, M>
where
	M: Material,
{
	pub sdf: S,
	pub material: MeshMaterial3d<M>,
}

impl<S, M> SdfMeshPayload<S, M>
where
	M: Material,
{
	pub fn new(sdf: S, material: MeshMaterial3d<M>) -> Self {
		Self { sdf, material }
	}
}

impl<S, M, Ctx> RenderItem<Ctx> for SdfMeshPayload<S, M>
where
	S: Sdf + Clone + Send + Sync + 'static,
	M: Material + Clone + Send + Sync + 'static,
	Ctx: SdfRenderContext + Send + Sync + 'static,
{
	fn spawn_render_items(&self, commands: &mut Commands, dispatch_entity: Entity, ctx: &Ctx) {
		let bb = ctx.render_bounds_aabb();
		let center: Vec3 = bb.center().into();
		let _ =
			(self.sdf.distance(center), ctx.mesh_resolution(), ctx.omission_aabb(), &self.material);

		commands.entity(dispatch_entity).with_children(|parent| {
			parent.spawn((
				Name::new("renderit:sdf_mesh_placeholder"),
				Transform::from_translation(center),
				Visibility::default(),
			));
		});
	}
}

/// Optional helper when you **do** have `Assets<Mesh>`: spawns a cuboid sized to `bounds` as a child.
pub fn spawn_sdf_placeholder_cuboid_child<M: Material + Clone>(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	dispatch_entity: Entity,
	bounds: Aabb3d,
	material: MeshMaterial3d<M>,
) {
	let he = bounds.half_size() * 2.0;
	let mesh = Mesh::from(Cuboid::new(he.x.max(1e-4), he.y.max(1e-4), he.z.max(1e-4)));
	let handle = meshes.add(mesh);
	let center: Vec3 = bounds.center().into();
	commands.entity(dispatch_entity).with_children(|parent| {
		parent.spawn((Mesh3d(handle), material, Transform::from_translation(center)));
	});
}
