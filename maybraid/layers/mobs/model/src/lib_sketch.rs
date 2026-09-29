//! SKETCH — not in the build. `MobGenerationPlugin<G: UrbanModel>`.
//!
//! Source: `world/lib/src/mobs.rs`. Split `WorldMobsPlugin` into generation
//! (this) and presentation (mobs/presentation). The world crate keeps nothing
//! mob-specific except Training's suspend flag wiring.

impl<G: UrbanModel> Plugin for MobGenerationPlugin<G> {
	fn build(&self, app: &mut App) {
		// MobGroupsPlugin (drop the is_plugin_added guard)
		// WorldMobIndex -> MobIndex, MobGenerateBullseye, MobPresentBullseye
		// LodGenerateBudget::<MobLodChan>::new(16)          // see budget_sketch.rs
		// LodGenerateRegionPlugin<MobGenerateBullseye, With<LodViewer>, MobLodChan>
		// LodGeneratePlugin<WorldMobCell, WorldMobIndex, MobLodChan, With<LodViewer>>
		// LodPresentRegionPlugin<MobPresentBullseye, With<LodViewer>, MobLodChan>
		//
		// (sync_world_mob_models, sync_world_mob_plant_hosts::<G>).chain()
		//     .after(LodGenerateSystems::Produce).before(LodGenerateSystems::Drain)
		// stream_world_mobs.before(LodGenerateSystems::Produce)
	}

	fn finish(&self, app: &mut App) {
		G::require_generation(app);
		app.require_layer::<VegetationGenerationPlugin, Self>();
	}
}

// ── sync_world_mob_plant_hosts through G ────────────────────────────────────
//
// Today (mobs.rs:563):
//   ResMut<UrbanizationIndex>  -> ensure_selected(extent, noise) over the generate keep
//   urbanization.filled_leaves_overlapping(region)   -> G::urbanization_leaves
//   developments.filled_cells_overlapping(region)    -> G::development_cells
//   Query<(&UrbanSetting, &GlobalTransform)>          (presentation output)
//   Query<(&DiscoverablePlace, &GlobalTransform)>     (presentation output)
//
// Two layering violations to PRESERVE and NAME, not fix:
//   1. `ensure_selected` is a write into urbanization generation from mob
//      generation. Give it a named entry point in urbanization-layer-model
//      (e.g. `UrbanizationSelect` system param with `ensure_selected(region)`),
//      and use that here instead of ResMut<UrbanizationIndex>.
//   2. UrbanSetting / DiscoverablePlace are spawned by presentation. Keep the
//      queries; add a comment + a follow-up issue.
//
// ── Training coupling ───────────────────────────────────────────────────────
//
// stream_world_mobs reads `Option<Res<TrainingGrounds>>` (world crate type).
// A layer crate cannot name it. Introduce `MobStreamSuspended(pub bool)` in
// this crate; the world sets it from TrainingGrounds each frame (or Training
// sets it directly). Same teardown behavior as today.
//
// ── Surface ─────────────────────────────────────────────────────────────────
//
// Generation does not read height. `WorldMobSurface` is presentation (see
// mobs/presentation sketch).
