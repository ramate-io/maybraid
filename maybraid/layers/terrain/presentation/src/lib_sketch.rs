//! SKETCH — not in the build. What terrain presentation still does not do.
//!
//! Raw Durham is already live. [`TerrainPresentationPlugin`](crate::TerrainPresentationPlugin)
//! is keyed on a presenter type. [`DurhamCellPresenter`](durham_terrain_models::DurhamCellPresenter)
//! owns `TerrainPresentEnabled`, the three `TerrainStreamPresenterState`s, and
//! `present_cells`. The world adds `TerrainPresentationPlugin::<DurhamCellPresenter>`
//! with the gate off; Training turns it on. This file is only the work that remains.

// ── #886 — padded (urbanized) presentation ──────────────────────────────────
//
// `PaddedCellPresenter<M>` lives in `urbanization-layer-presentation` and
// installs `present_urbanization_padded_terrain` + `PaddedTerrainPresenter`
// (`richmond/developments-on-terrain-playground/src/urbanization_stream.rs`,
// `richmond/development-models/src/presentation.rs`).
// The world adds
// `TerrainPresentationPlugin::<PaddedCellPresenter<OnTerrain<Durham>>>`.
//
// Leave `sync_raw_terrain_replacements` in that playground. The world does
// spawn raw Durham roots while Training has `TerrainPresentEnabled` on, and
// the sync hands those cells back when urbanization turns off.

// ── Follow-up — one presenter over `TerrainCell` data ───────────────────────
//
// A later generic `CellPresenter<M>` lives in this crate. A presentation-side
// cell trait carries the material then. That port also needs:
//   - a LOD draw test (`draws_at` / scene level) so the keep filter stays generic
//   - a presented-root marker the collider systems already read
//   - a cell-set revision for the presenter's tick key
// Do not put that trait on `terrain-layer-model`.
