//! SKETCH — not in the build. `VegetationGenerationPlugin`.
//!
//! Vegetation generation reads no terrain: grove, forest, and bump-out selection
//! are noise + extent only. That is why the plugin is untyped.
//!
//! Sources:
//!   chico/forests/src/stream.rs::register_forest_lod   (generate half)
//!   chico/vegetation-on-terrain-playground/src/bump_out.rs::register_bump_out_lod (generate half)
//!   chico/vegetation-on-terrain-playground/src/lib.rs   (stream_* systems)

impl Plugin for VegetationGenerationPlugin {
	fn build(&self, app: &mut App) {
		// Forest:
		//   ForestIndex, ForestGenerateBullseye, ForestPresentBullseye,
		//   LodGenerateBudget::<ForestLodChan>: init or take from
		//     VegetationLayerConfig — do not insert. Today's
		//     register_forest_lod still inserts 16; WorldPlugin inserts 16
		//     again so the world value is explicit. Last-insert-wins is now
		//     per channel. When this layer owns registration, drop the
		//     insert so the assembler can set the budget before or after.
		//   LodGenerateRegionPlugin<ForestGenerateBullseye, With<LodViewer>, ForestLodChan>
		//   LodGeneratePlugin<ChicoGrove, ForestIndex, ForestLodChan, With<LodViewer>>
		//   LodPresentRegionPlugin<ForestPresentBullseye, With<LodViewer>, ForestLodChan>
		//     (present regions are keep bookkeeping; the presenter plugins are in presentation)
		//
		// Bump-outs: same split for CanopyBumpOut / MediumCanopyBumpOut
		//   (BumpOutLodChan / MediumBumpOutLodChan). Same rule: init or
		//   take from layer config, do not insert. Today's
		//   register_bump_out_lod inserts 16 on both (they inherited
		//   forest's global 16). World also inserts 16 on both.
		//
		// Streams (today in VegetationOnTerrainPlugin, non-commands branch):
		//   stream_durham_forest.before(LodGenerateSystems::Produce).before(LodPresentSystems::Produce)
		//   stream_canopy_bump_outs.after(stream_durham_forest) (same befores)
		//   both .run_if(terrain_streaming_enabled)
		//   Rename `stream_durham_forest` -> `stream_forest`; it never read Durham.
		//
		// ChicoBumpOutPlugin, VegetationHostPlugin, register_vegetation_view:
		//   check each. Host / material / view registration belongs to presentation.
		//
		// `configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain))`
		// is global; whichever plugin needs it adds it (idempotent).
	}
}

// `spawn_groves` (tiled groves, fine-patch playgrounds) reads Durham at spawn.
// It is presentation; it goes with VegetationPresentationPlugin or stays in the
// playground if the world never runs it (check: world passes commands: false,
// and the non-commands branch still adds spawn_groves).
