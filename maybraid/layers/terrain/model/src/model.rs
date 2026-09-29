//! [`TerrainModel`] and the per-cell / owned-sample traits it names.

use bevy::app::App;
use bevy::ecs::system::{ReadOnlySystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::transform::components::Transform;
use lod::gen::Id;

/// Owned height field for work off the main thread (grove grow).
pub trait HeightField {
	/// Stored-only height at `xz`. `None` when the covering cell is not generated.
	fn height_at(&self, xz: Vec2) -> Option<f32>;
}

/// One generated surface cell that terrain presentation and overlays can mesh.
pub trait TerrainCell: Send + Sync + 'static {
	/// Mesh identity shared by the fill and overlays such as canopy bump-outs.
	type Mesh: Clone + Send + Sync + 'static;

	fn bounds(&self) -> Aabb3d;

	fn mesh_builder(&self) -> Self::Mesh;

	/// World pose for [`Self::mesh_builder`] vertices.
	fn chunk_pose(&self) -> Transform;

	/// Whether this cell's fill carries the walk collider.
	fn seeds_collision(&self) -> bool;
}

/// A generated world surface, named by a zero-sized marker type.
///
/// Every accessor is GET-only: it reads what generation already stored and never
/// admits missing cells. Callers that can tolerate a coarse answer opt into
/// [`Self::fallback_height_at`] explicitly (see
/// [`TerrainView::height_or_fallback`](crate::TerrainView::height_or_fallback)).
pub trait TerrainModel: Send + Sync + 'static {
	/// Per-cell artifact this model's generation stores.
	type Cell: TerrainCell;

	/// Read-only borrow of the resources that hold this model.
	type Read: ReadOnlySystemParam + 'static;

	/// Region-scoped owned copy of this model's height field.
	type Snapshot: HeightField + Clone + Send + Sync + 'static;

	/// Stored-only height at `xz`.
	fn height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> Option<f32>;

	/// Analytic height used before the covering cell is stored (Durham: base noise).
	fn fallback_height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> f32;

	/// Ids of stored cells whose bounds intersect `region`.
	fn cell_ids_overlapping(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Vec<Id>;

	fn cell<'a>(read: &'a SystemParamItem<'_, '_, Self::Read>, id: Id) -> Option<&'a Self::Cell>;

	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Self::Snapshot;

	/// Panic naming any generation plugin this model needs that `app` lacks.
	///
	/// Wrappers recurse into their inner model, so one call checks the whole stack.
	fn require_generation(app: &App);
}
