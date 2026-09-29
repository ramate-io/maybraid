//! SKETCH — not in the build. What terrain presentation still does not do.
//!
//! Raw Durham is already live. [`TerrainPresentationPlugin`](crate::TerrainPresentationPlugin)
//! calls [`TerrainPresentation::install_presentation`](terrain_layer_model::TerrainPresentation)
//! and `Durham` owns that hook: `TerrainPresentEnabled`, the three
//! `TerrainStreamPresenterState`s, and `present_cells`. The world adds
//! `TerrainPresentationPlugin::<OnTerrain<Durham>>` with the gate off;
//! Training turns it on. This file is only the work that remains.

// ── #886 — padded (urbanized) presentation ──────────────────────────────────
//
// `impl TerrainPresentation for Urbanization<M>` moves
// `present_urbanization_padded_terrain` + `PaddedTerrainPresenter`
// (`richmond/developments-on-terrain-playground/src/urbanization_stream.rs`,
// `richmond/development-models/src/presentation.rs`).
//
// Leave `sync_raw_terrain_replacements` in that playground. The world does
// spawn raw Durham roots while Training has `TerrainPresentEnabled` on, and
// the sync hands those cells back when urbanization turns off.

// ── Follow-up — one presenter over `TerrainCell` ────────────────────────────
//
// Unifying the raw and padded presenters behind `TerrainCell` is not this
// split. That port needs, on `TerrainCell` / `TerrainModel`:
//   - a LOD draw test (`draws_at` / scene level) so the keep filter stays generic
//   - a presented-root marker the collider systems already read
//   - a cell-set revision for the presenter's tick key
// Do not special-case Durham or Richmond in this crate to get there.
