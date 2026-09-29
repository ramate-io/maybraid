//! SKETCH — not in the build. `MobPresentationPlugin<G: UrbanModel>`.
//!
//! Source: `world/lib/src/mobs.rs` (presenter half of WorldMobsPlugin).

impl<G: UrbanModel> Plugin for MobPresentationPlugin<G> {
	fn build(&self, app: &mut App) {
		// MobLodRefreshMode::Indexed
		// WorldMobPresenterState
		// LodPresentPlugin<WorldMobCell, WorldMobIndex, MobPresenter<G>, MobLodChan, With<LodViewer>>
		// LodPresentCullPlugin<WorldMobCell, WorldMobIndex, MobPresenter<G>, MobLodChan>
		// LodSceneRefreshRegionPlugin<MobHighLodRegion, With<LodViewer>, MobHighLodChan>
		// GimmeLodSceneRefreshPlugin<MobScene, MobHighLodChan, With<LodViewer>>
		// fit_mob_hosts_to_surface::<G>.in_set(MobSceneSystems::Surface)
		// pulse_world_mob_high_lod  on_timer(250 ms) in LodRefreshSystems::ProduceRegions
		// update_lod_host_levels::<MobScene, (), With<LodViewer>> on_timer(1 s) in UpdateLevels
	}

	fn finish(&self, app: &mut App) {
		app.require_layer::<MobGenerationPlugin<G>, Self>();
	}
}

// ── The surface, once ───────────────────────────────────────────────────────
//
// WorldMobSurface (mobs.rs:428) is the composed-height formula:
//
//     raw = terrain.composed_height_at(layout, x, z).unwrap_or(base.height_at(x, z))
//     pads(probe ±0.5 m).modify_elevation(raw, x, z)
//
// That is exactly `TerrainView<Urbanization<OnTerrain<Durham>>>::height_or_fallback`.
// Replace the struct with `TerrainView<G>` in MobPresenter::handle and
// fit_mob_hosts_to_surface. Same formula is copied in:
//   world/lib/src/player_lifecycle.rs:88-106  (WorldPlayerSurface)
//   world/lib/src/training_plaza.rs           (plaza surface)
// Those are world-crate consumers; switch them to TerrainView<G> in the
// "composed surface" issue, which can land before the layer moves.
//
// Parity test: on a world with one stored Durham cell and one pad complex,
// old WorldMobSurface::surface_height == TerrainView::height_or_fallback at a
// grid of points (inside pad, on skirt, outside, and in an ungenerated cell).
