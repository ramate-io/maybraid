//! SKETCH — not in the build. What terrain presentation still does not do.
//!
//! Raw Durham is already live. [`TerrainPresentationPlugin`](crate::TerrainPresentationPlugin)
//! is `TerrainPresentationPlugin<M, P>`: model first, then how it is drawn.
//! [`DurhamCells`](durham_terrain_models::DurhamCells) implements
//! `TerrainPresenter<OnTerrain<Durham>>` and owns `TerrainPresentEnabled`, the
//! three `TerrainStreamPresenterState`s, and `present_cells`. The world adds
//! `TerrainPresentationPlugin::<OnTerrain<Durham>, DurhamCells>` with the gate
//! off; Training turns it on. Padded urbanized presentation is live as
//! `TerrainPresentationPlugin::<Urbanization<OnTerrain<Durham>>, PaddedCells>`.
//! This file is only the work that remains.

// ── Follow-up — one presenter over `TerrainCell` data ───────────────────────
//
// A later generic presenter is `impl<M: TerrainModel> TerrainPresenter<M> for
// CellPresenter`. A presentation-side cell trait carries the material then.
// That port also needs:
//   - a LOD draw test (`draws_at` / scene level) so the keep filter stays generic
//   - a presented-root marker the collider systems already read
//   - a cell-set revision for the presenter's tick key
// Do not put that trait on `terrain-layer-model`.
