//! SKETCH — not in the build. `UrbanizationPresentationPlugin<G: UrbanModel>`.
//!
//! Building hosts, furniture, and building walk colliders for built developments.
//! Padded terrain cells are NOT here: they are `TerrainPresentationPlugin<G>`.

impl<G: UrbanModel> Plugin for UrbanizationPresentationPlugin<G> {
	fn build(&self, app: &mut App) {
		// From DevelopmentsOnTerrainPlugin::build (is_plugin_added guards go away;
		// the assembler adds each once):
		//   FurnitureShadersPlugin, FurnitureAssembliesPlugin, FurnitureStreamPlugin
		//   BuildingWalkColliderPlugin      (world/lib/src/lib.rs:190 adds it today)
		//   UrbanizationPresenterState      (urbanization_stream.rs:116)
		//   HostsDirty
		//
		// Systems:
		//   present_urbanization_hosts
		//     .after(UrbanizationGenerationSystems)
		//     .run_if(urbanization_streaming_enabled).run_if(terrain_streaming_enabled)
		//     .before(LodPresentSystems::Produce)
		//   FurnitureStreamSystems::Generate.after(present_urbanization_hosts)
		//     (today: .after(generate_urbanization_developments))
		//
		// present_urbanization_hosts reads through G instead of DevelopmentIndex:
		//   for leaf in G::urbanization_leaves(read, keep)       // non-empty leaves
		//     cell  = G::development_cells(...) matching leaf id
		//     built = G::built(...) matching leaf id  (+ version)
		//   If "by id" lookups are needed, add `development_cell(read, id)` /
		//   `built_at(read, id) -> Option<(&BuiltDevelopment, Version)>` to UrbanModel
		//   rather than falling back to Richmond stores.
		//
		// UrbanSetting components are spawned here. Mob generation reads them today
		// (world/lib/src/mobs.rs::sync_world_mob_plant_hosts). That is generation
		// reading presentation. Preserve for now; see mobs/model sketch.
	}

	fn finish(&self, app: &mut App) {
		G::require_generation(app);
	}
}
