//! SKETCH — not in the build. `UrbanizationGenerationPlugin<M>::build`.
//!
//! Source: `richmond/developments-on-terrain-playground` (`lib.rs` build,
//! `urbanization_stream.rs`). Everything that writes urbanization, development,
//! built-development, or padded-cell storage moves here. Host spawning and
//! padded-cell presentation move to presentation layers.

impl<M> Plugin for UrbanizationGenerationPlugin<M>
where
	M: TerrainModel,
	Urbanization<M>: TerrainModel,
{
	fn build(&self, app: &mut App) {
		app.add_plugins(RichmondDevelopmentModelsPlugin) // DevelopmentEntryStore etc.
			.insert_resource(DevelopmentConfig {
				likelihood,
				use_urbanization,
				..from_world_seed(42)
			})
			.init_resource::<UrbanizationStreamingEnabled>()
			.init_resource::<UrbanizationLayerConfig>(); // replaces the fields of
		                                        // PlaygroundConfig this layer reads:
		                                        // `urbanization: Option<UrbanizationStreamSpec>`,
		                                        // `focus_urbanization`, `focus_development`.

		// register_urbanization_lod (urbanization_stream.rs:91), minus the
		// presenter state (-> UrbanizationPresentationPlugin):
		//   UrbanizationIndex, Urbanization{Generate,Present}Bullseye,
		//   LodGenerateBudget::<UrbanizationLodChan>::new(16)   // see budget_sketch.rs
		//   LodGenerateRegionPlugin / LodGeneratePlugin<SelectedUrbanization, UrbanizationIndex, ..>
		//   LodPresentRegionPlugin<UrbanizationPresentBullseye, ..>
		//     The present-keep region is what generate_urbanization_developments
		//     reads as its build region. It is a generation input despite the name.

		// Systems, with today's orderings verbatim:
		//   sync_urbanization_pin .before(stream_urbanization).before(LodGenerateSystems::Produce)
		//     (ungated: mobs read UrbanizationIndex before terrain streaming starts)
		//   (
		//     stream_urbanization.before(LodGenerateSystems::Produce),
		//     (generate_urbanization_developments.after(LodGenerateSystems::Drain),
		//      generate_urbanization_padded_terrain)
		//        .chain().run_if(urbanization_streaming_enabled),
		//   ).chain().run_if(terrain_streaming_enabled)
		//    .before(LodPresentSystems::Produce).before(TerrainColliderSystems::QueueMeshes)
		//
		// Today presentation systems (present_urbanization_hosts,
		// present_urbanization_padded_terrain, sync_raw_terrain_replacements)
		// sit inside that same chain. Splitting them out must keep them after
		// generation: expose `UrbanizationGenerationSystems` (a SystemSet) and
		// have presentation plugins order `.after(UrbanizationGenerationSystems)`.
		//
		// FurnitureStreamSystems::Generate.after(generate_urbanization_developments)
		//   -> furniture belongs to urbanization presentation; order it after
		//      UrbanizationGenerationSystems there.
	}

	fn finish(&self, app: &mut App) {
		M::require_generation(app);
	}
}

// ── Reading the inner model ─────────────────────────────────────────────────
//
// Pad heights sample `M`, never `Urbanization<M>` (pads must not feed back).
// Today that goes through `DevelopmentIndex` -> `TerrainEntryStore` directly.
// Strict preservation: leave DevelopmentIndex's internals alone; the type-level
// statement is the `M: TerrainModel` bound plus `finish` checking M. Swapping
// DevelopmentIndex's height reads to `TerrainView<M>` is a follow-up.
//
// `generate_urbanization_padded_terrain` iterates
// `terrain_store().terrain_ids_overlapping(region)` = `M::cell_ids_overlapping`
// and composes via `PadComposable::compose_pads` (already in pads.rs). Keep the
// store's own GeneratingSpatialIndex<TerrainWithPads> path; do not reimplement.
//
// ── Wart kept on purpose (strict preservation) ──────────────────────────────
//
// `DevelopmentForestPresenter::handle` calls
// `GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate` at present time.
// That is generation inside vegetation presentation. Keep the behavior, but move
// it behind an explicit hook owned by this crate, e.g.
//
//     pub fn prepare_development_cells(read: &mut DevelopmentIndex, bounds: Aabb3d, lod_ref: &LodRef)
//
// so vegetation presentation calls a named urbanization-generation entry point
// instead of reaching into Richmond. Removing it is the #720 follow-up.
//
// ── Also moving out of the playground crate ─────────────────────────────────
//
// UrbanizationStreamSpec, parse_urbanization_kind, DEFAULT_URBANIZATION_*,
// stream_radii_m, UrbanizationStreamLod (minus presenter), UrbanSetting (->
// urbanization presentation; mobs read it). The playground keeps camera,
// commands, UI, and becomes an assembler over layer plugins.
