//! SKETCH — not in the build. World assembly once every layer is filled.
//!
//! `tests/layer_wiring.rs` already type-checks this stack. The last issue swaps
//! `WorldPlugin::build`'s terrain / vegetation / urbanization / mobs block for it
//! and deletes the playground plugins from the world's dependency list.

impl Plugin for WorldPlugin {
	fn build(&self, app: &mut App) {
		// ... unchanged prelude (motor traction, resources, FurnitureShadersPlugin,
		//     WorldMaterialRefPlugin, input, controller, player presentation, camera,
		//     combat HUD, ragdoll) ...

		app.add_plugins((
			BaseTerrainGenerationPlugin::<Durham>::new(DurhamTerrainConfig::playable_world()),
			UrbanizationGenerationPlugin::<OnTerrain<Durham>>::new(
				UrbanizationLayerConfig::world_defaults(),
			),
			VegetationGenerationPlugin::new(VegetationLayerConfig::world_defaults()),
			MobGenerationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
			TerrainPresentationPlugin::<Urbanization<OnTerrain<Durham>>, PaddedCells>::default(),
			UrbanizationPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
			VegetationPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
			MobPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
			// Training only: raw Durham, gated off in the open world.
			TerrainPresentationPlugin::<OnTerrain<Durham>, DurhamCells>::default(),
		))
		// Mob generate budget stays here until #888. Forest and bump-out budgets
		// come from `VegetationLayerConfig` inside `VegetationGenerationPlugin`.
		.insert_resource(LodGenerateBudget::<MobLodChan>::new(16));

		// Training: set MobStreamSuspended from TrainingGrounds (mobs/model sketch).

		// ... unchanged remainder (intelligence, skill map, POI, lifecycle, stash,
		//     position, training, sky, chrome, systems) ...
	}
}

// ── Things the world still takes from playground crates today ───────────────
//
//   chico_vegetation_on_terrain_playground::{PlayerPhysicsEnabled, PlayerSpawnXz,
//     CharacterCameraFollowEnabled, CharacterLocomotion, CharacterSpecies,
//     PadMovementEnabled, PlayerControlSystems, PlaygroundDiag, PlaygroundMode,
//     PlaygroundTimingPlugin, RequestSetCharacter, VegetationPlayerMotor, WorldBaseTerrain}
//   urbanization_layer_presentation::UrbanSetting (moved in #886)
//
// Terrain/vegetation/urbanization items move with their layers. Character /
// player / diag items are not layer concerns; leave them for a separate cleanup
// unless they block removing the playground plugins from the world build.
//
// ── Done when ───────────────────────────────────────────────────────────────
//
// - playground vegetation / development plugins and `TerrainPlugin::<Durham>` are gone from the world.
// - `WorldMobSurface` / `WorldPlayerSurface` are gone
//   (`TerrainView<Urbanization<OnTerrain<Durham>>>`; the pad-free plaza probe
//   is `TerrainView<OnTerrain<Durham>>`).
// - `tests/layer_wiring.rs` is replaced by a test that builds WorldPlugin's
//   layer stack in a headless App and runs `finish` (requirements satisfied).
