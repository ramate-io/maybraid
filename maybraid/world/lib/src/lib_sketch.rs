//! SKETCH — not in the build. World assembly once every layer is filled.
//!
//! `tests/layer_wiring.rs` already type-checks this stack. The last issue swaps
//! `WorldPlugin::build`'s terrain / vegetation / urbanization / mobs block for it
//! and deletes the playground plugins from the world's dependency list.

type Ground = Urbanization<OnTerrain<Durham>>;

impl Plugin for WorldPlugin {
	fn build(&self, app: &mut App) {
		// ... unchanged prelude (motor traction, resources, FurnitureShadersPlugin,
		//     WorldMaterialRefPlugin, input, controller, player presentation, camera,
		//     combat HUD, ragdoll) ...

		app.add_plugins((
			BaseTerrainGenerationPlugin::<Durham>::new(DurhamTerrainConfig::playable_world()),
			UrbanizationGenerationPlugin::<OnTerrain<Durham>>::default(),
			VegetationGenerationPlugin,
			MobGenerationPlugin::<Ground>::default(),
			TerrainPresentationPlugin::<Ground>::default(),
			UrbanizationPresentationPlugin::<Ground>::default(),
			VegetationPresentationPlugin::<Ground>::default(),
			MobPresentationPlugin::<Ground>::default(),
		))
		// Assembler-owned generate budgets (world-effective 16 on every
		// channel). Keep these even where today's helpers also insert 16:
		// once #886 / #887 / #888 init instead of insert, these lines are
		// the only source. Insert after the layer plugins (or before —
		// init_resource will not overwrite).
		.insert_resource(LodGenerateBudget::<ForestLodChan>::new(16))
		.insert_resource(LodGenerateBudget::<UrbanizationLodChan>::new(16))
		.insert_resource(LodGenerateBudget::<MobLodChan>::new(16))
		.insert_resource(LodGenerateBudget::<BumpOutLodChan>::new(16))
		.insert_resource(LodGenerateBudget::<MediumBumpOutLodChan>::new(16))
		// Layer configs that used to ride on playground configs:
		.insert_resource(UrbanizationLayerConfig::world_defaults())   // from DevelopmentsPlaygroundConfig::world_defaults()
		.insert_resource(VegetationLayerConfig::world_defaults());    // from VegetationPlaygroundConfig::world_defaults()

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
//   richmond_developments_on_terrain_playground::UrbanSetting
//
// Terrain/vegetation/urbanization items move with their layers. Character /
// player / diag items are not layer concerns; leave them for a separate cleanup
// unless they block removing the playground plugins from the world build.
//
// ── Done when ───────────────────────────────────────────────────────────────
//
// - `rg 'VegetationOnTerrainPlugin|DevelopmentsOnTerrainPlugin|TerrainPlugin::<Durham>' maybraid/world` is empty.
// - `WorldMobSurface` / `WorldPlayerSurface` are gone (TerrainView<Ground>).
// - `tests/layer_wiring.rs` is replaced by a test that builds WorldPlugin's
//   layer stack in a headless App and runs `finish` (requirements satisfied).
