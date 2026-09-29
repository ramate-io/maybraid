//! SKETCH — not in the build. Durham split into generation vs raw presentation.
//!
//! `layer.rs` already implements `TerrainModel` / `TerrainGeneration` for
//! `Durham` by delegating to `TerrainPlugin::<Durham>` with `present = false`.
//! That matches the world today. This sketch is the remaining split so that
//! `TerrainPlugin` disappears and raw present is only reachable through
//! `TerrainPresentationPlugin<OnTerrain<Durham>>`.

// ── host.rs today (TerrainPlugin<Durham>::build) ────────────────────────────
//
//   generation (stays in `Durham::install_generation`):
//     VisualGeometryCorePlugin, DurhamTerrainModelsPlugin, DurhamTerrainShaderPlugin
//     install_enforced_mesh_cache::<TerrainMeshBuilder, DurhamTerrainShader>
//     share_terrain_chunk_refs::<TerrainMeshBuilder>
//     install_enforced_mesh_cache::<ComposedWater, RefractionWater>
//     MeshFulfillBudget<TerrainMeshBuilder>
//     TerrainConfig, WorldBaseTerrain, TerrainCoverage, TerrainCellLayout,
//     TerrainFillParams, TerrainPresentationDirty, TerrainPresentPending,
//     TerrainStreamingEnabled, TerrainLayoutPinned
//     generate_cells in TerrainFillSystems::Generate
//
//     Shader + mesh-cache plugins are shared by every model whose cell material
//     is `DurhamTerrainShader` (raw and padded). They belong to generation
//     because both presentations need them and generation is always present.
//
//   raw presentation (moves behind `TerrainPresentationPlugin<OnTerrain<Durham>>`):
//     TerrainPresentEnabled, TerrainStreamPresenterState<Near|Far|Background>,
//     setup_presentation_assets (Startup), present_cells
//
// `TerrainPresentEnabled` goes away: "present raw Durham" becomes "the app
// added `TerrainPresentationPlugin<OnTerrain<Durham>>`". The world never adds it.

impl TerrainGeneration for Durham {
	type Config = DurhamTerrainConfig;

	fn install_generation(app: &mut App, config: &DurhamTerrainConfig) {
		// Body of TerrainPlugin<Durham>::build minus the raw-present block.
		// `setup_presentation_assets` also inserts `WaterPresentationAssets` and
		// `TerrainPresentationAssets`; check whether padded present / water read
		// them in the world before deciding which side owns that Startup system.
		// If the padded presenter needs them, keep it in generation.
		todo!()
	}
}

// ── How TerrainPresentationPlugin<M> reaches Durham-specific presenters ─────
//
// See maybraid/layers/terrain/presentation/src/lib_sketch.rs. Short form: the
// generic plugin drives a presenter through `TerrainCell` only. If raw Durham
// banding (Near/Far/Background presenter states, scale filtering) cannot be
// expressed through `TerrainCell`, extend `TerrainCell` rather than special-case
// Durham in the presentation crate.

// ── Callers to migrate (no wrapper left behind) ─────────────────────────────
//
//   world/lib/src/lib.rs:163          TerrainPlugin::<Durham>::playable_world()
//     -> BaseTerrainGenerationPlugin::<Durham>::new(DurhamTerrainConfig::playable_world())
//   chico/vegetation-on-terrain-playground (own_terrain branch)
//     -> BaseTerrainGenerationPlugin::<Durham> + TerrainPresentationPlugin::<OnTerrain<Durham>>
//   durham playgrounds / training (`fine_patch`) likewise.
//   `rg 'TerrainPlugin::<Durham>'` must be empty at the end.
//
// ── Invariants ──────────────────────────────────────────────────────────────
//
// - host.rs tests keep passing (move them next to whatever they test).
// - `playable_world_disables_raw_presentation` becomes "world stack has no raw
//   presentation plugin", asserted in maybraid-world.
