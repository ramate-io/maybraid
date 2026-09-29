//! SKETCH — not in the build. `VegetationPresentationPlugin<G: TerrainModel>`.
//!
//! Replaces three presenter pairs with one generic presenter per channel:
//!
//!   chico  DurhamForestPresenter            / DurhamCanopyBumpOutPresenter / DurhamMedium…
//!   richmond DevelopmentForestPresenter     / DevelopmentCanopyBumpOutPresenter / DevelopmentMedium…
//!
//! The Richmond variants exist only to blend pads into Durham height. With
//! `G = Urbanization<OnTerrain<Durham>>`, `G::snapshot` already does that
//! (UrbanSnapshot = Durham snapshot + merged pads), so the pad special case disappears.

/// Grove presenter over any ground model. Port of DevelopmentForestPresenter.
#[derive(SystemParam)]
pub struct GroundForestPresenter<'w, 's, G: TerrainModel> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, ForestPresenterState>,
	ground: TerrainView<'w, 's, G>,
	// Strict preservation: DevelopmentForestPresenter generates DevelopmentCells for the
	// grove bounds before sampling. Keep that via an explicit prepare hook, not by
	// naming Richmond here. Options (pick one, keep it tiny):
	//   a) `TerrainModel::Prepare: SystemParam` + `fn prepare(p, bounds, lod_ref)`,
	//      no-op for Durham, calls urbanization's prepare_development_cells.
	//   b) a `GroundPrepare<G>` trait in terrain-layer-model with a blanket no-op
	//      and an impl in urbanization-layer-model.
	// Delete it in the #720 follow-up.
}

impl<G: TerrainModel> RegionPresenter<ChicoGrove, ForestIndex>
	for GroundForestPresenter<'_, '_, G>
{
	fn handle(&mut self, id: Id, version: Version, grove: &ChicoGrove, lod_ref: &LodRef) {
		// prepare(grove.aabb(), lod_ref)   // see above
		let world = GroundGroveSample::new(self.ground.snapshot(grove.aabb()));
		self.state
			.present_with_world(&mut self.commands, id, version, grove, lod_ref, world);
	}
	// presented_version / hide / is_hidden / presented_ids / remove_stale / cull:
	// delegate to state exactly as today.
}

/// `GroveWorldSample` over any `HeightField`.
/// Today's `DurhamGroveSample<OwnedDurhamTerrain>` also provides steepness;
/// derive steepness from the height field by finite differences at the same
/// spacing DurhamGroveSample uses, or add `fn steepness_at` to HeightField.
/// Must produce identical samples on Durham (add a parity test).
pub struct GroundGroveSample<S: HeightField>(S);

impl<G: TerrainModel> Plugin for VegetationPresentationPlugin<G> {
	fn build(&self, app: &mut App) {
		// Presenter halves of register_forest_lod / register_bump_out_lod:
		//   ForestPresenterState, LodPresentPlugin<ChicoGrove, ForestIndex, GroundForestPresenter<G>, ForestLodChan, ..>
		//   LodPresentCullPlugin<..same..>
		//   bump-out presenter states + LodPresent/Cull plugins with GroundBumpOutPresenter<G>
		// VegetationHostPlugin { register_camera: false }, VegetationOnTerrainMaterialRefPlugin,
		// register_vegetation_view, ChicoBumpOutPlugin (confirm split vs generation).
	}

	fn finish(&self, app: &mut App) {
		G::require_generation(app);
		app.require_layer::<VegetationGenerationPlugin, Self>();
	}
}

// ── Bump-outs ───────────────────────────────────────────────────────────────
//
// Bump-out presenters clone terrain fine/medium cell mesh handles
// (`fine_terrain_for`, `medium_terrain_for`, `terrain_chunk_ref`). Generic form:
// `ground.cell_ids_overlapping(bounds)` + `ground.cell(id)` + TerrainCell's
// `mesh_builder()` / `chunk_pose()` / `bounds()`. For Urbanization that is the
// padded cell, which is what Development* bump-out presenters use today. Verify.
//
// ── Invariants ──────────────────────────────────────────────────────────────
//
// - `pad_modulation_sets_exact_terrace_and_preserves_base_outside` survives as a
//   test on UrbanSnapshot (urbanization-layer-model).
// - Grove heights identical on Durham-only and Urbanized grounds before/after.
