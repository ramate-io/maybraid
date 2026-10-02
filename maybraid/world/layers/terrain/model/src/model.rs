//! [`TerrainModel`] and the per-cell / owned-sample traits it names.

use bevy::app::App;
use bevy::ecs::system::{ReadOnlySystemParam, SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::transform::components::Transform;
use lod::lod_ref::LodRef;

/// Owned height field for work off the main thread (grove grow).
pub trait HeightField {
	/// Stored-only height at `xz`. `None` when the covering cell is not generated.
	fn height_at(&self, xz: Vec2) -> Option<f32>;

	/// Analytic height when [`Self::height_at`] is `None` (Durham: base noise).
	///
	/// Snapshots carry their own copy so grove grow can sample off the main thread.
	/// Urbanized snapshots apply pads to this fallback the same way they apply
	/// pads to a stored height.
	fn fallback_height_at(&self, xz: Vec2) -> f32;
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

	/// Sample-grid resolution exponent (`2.pow(res_2)` samples across the cell).
	///
	/// Canopy bump-outs copy this into the same cascade-chunk builder the fill
	/// uses, then into the overlay `TerrainChunkRef`.
	fn res_2(&self) -> u8;
}

/// A generated world surface, named by a zero-sized marker type.
///
/// Every accessor is GET-only: it reads what generation already stored and never
/// admits missing cells. Callers that can tolerate a coarse answer opt into
/// [`Self::fallback_height_at`] explicitly (see
/// [`TerrainView::height_or_fallback`](crate::TerrainView::height_or_fallback)).
pub trait TerrainModel: Send + Sync + 'static {
	/// Model that owns the terrain contract resources.
	///
	/// Wrappers name their inner model's [`Self::Base`] so readers share one
	/// `TerrainStreaming` / `TerrainExtent` / collider set.
	type Base: Send + Sync + 'static;

	/// Per-cell artifact this model's generation stores.
	type Cell: TerrainCell;

	/// Read-only borrow of the resources that hold this model.
	type Read: ReadOnlySystemParam + 'static;

	/// Region-scoped owned copy of this model's height field.
	type Snapshot: HeightField + Clone + Send + Sync + 'static;

	/// Present-time prepare before a region is sampled.
	///
	/// #720 wart: grove present generates development cells for the grove bounds
	/// before sampling. Durham and [`OnTerrain`](crate::OnTerrain) do nothing;
	/// urbanization calls `prepare_development_cells`. The presenter runs this
	/// in a [`ParamSet`](bevy::ecs::system::ParamSet) ahead of [`TerrainView`](crate::TerrainView)
	/// because the prepare borrows development storage mutably. Delete the hook
	/// in <https://github.com/ramate-io/maybraid/issues/720>.
	type Prepare: SystemParam + 'static;

	/// See [`Self::Prepare`].
	fn prepare(
		prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	);

	/// Stored-only height at `xz`.
	fn height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> Option<f32>;

	/// Analytic height used before the covering cell is stored (Durham: base noise).
	fn fallback_height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> f32;

	/// Cell a canopy bump-out should clone for `bounds`.
	///
	/// `target_size` is the raw-cell width. Among stored cells overlapping
	/// `bounds`, the greatest XZ overlap whose width is within 25% of
	/// `target_size` wins (the old `fine_terrain_for` / `medium_terrain_for`
	/// rule).
	///
	/// A model with a replacement cell (urbanization's padded terrain) prefers
	/// it first. `overlay_size_tolerance` gates that preference only:
	/// - `None` accepts any replacement size (urbanized fine bump-outs);
	/// - `Some(tol)` accepts it only when `|width - target_size| < tol`
	///   (urbanized medium uses `1e-2`).
	///
	/// A rejected replacement falls through to the raw cell. Models with no
	/// replacement (Durham) ignore `overlay_size_tolerance`.
	///
	/// The returned cell's [`TerrainCell::mesh_builder`] and [`TerrainCell::res_2`]
	/// build the overlay chunk ref: `cascade_chunk_for_cell(bounds, res_2)` into
	/// `TerrainChunkRef::new(mesh_builder, chunk, res_2)`.
	fn overlay_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		bounds: Aabb3d,
		target_size: f32,
		overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = <Self::Cell as TerrainCell>::Mesh>>;

	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Self::Snapshot;

	/// Panic naming any generation plugin this model needs that `app` lacks.
	///
	/// Wrappers recurse into their inner model, so one call checks the whole stack.
	fn require_generation(app: &App);
}
