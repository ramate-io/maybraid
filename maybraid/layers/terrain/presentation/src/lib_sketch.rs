//! SKETCH — not in the build. `TerrainPresentationPlugin<M>`.
//!
//! One generic presenter over `M::Cell: TerrainCell`. In the world `M` is
//! `Urbanization<OnTerrain<Durham>>`, so this replaces the padded path in
//! `richmond/developments-on-terrain-playground/src/urbanization_stream.rs`:
//! `present_urbanization_padded_terrain` + `PaddedTerrainPresenter`
//! (`richmond/development-models/src/presentation.rs`).
//! For `OnTerrain<Durham>` (fine-patch playgrounds) it replaces
//! `durham/models/src/terrain/host.rs::present_cells`.

impl<M: TerrainModel> Plugin for TerrainPresentationPlugin<M> {
	fn build(&self, app: &mut App) {
		app.init_resource::<CellPresenterState<M>>().add_systems(
			Update,
			present_terrain_cells::<M>
				.run_if(terrain_streaming_enabled)
				// Orderings copied from the padded path today:
				.before(LodPresentSystems::Produce)
				.before(TerrainColliderSystems::QueueMeshes),
		);
	}

	fn finish(&self, app: &mut App) {
		M::require_generation(app);
	}
}

/// Presented root per cell id, generic over `M`. Port of `PaddedTerrainPresenterState`.
#[derive(Resource)]
struct CellPresenterState<M> {/* HashMap<Id, PresentedEntry>, PhantomData<M> */}

fn present_terrain_cells<M: TerrainModel>(
	view: TerrainView<M>,
	region: PresentRegion, // today: pad_visual_region(layout, urban keep)
	viewer: ViewerXz,      // today: first LodViewer, else Camera3d, quantized 8 m
	mut state: ResMut<CellPresenterState<M>>,
	mut commands: Commands,
	mut last: Local<Option<TickKey>>,
) {
	// 1. Tick key: (region, cell-set revision, viewer quant). Skip if unchanged.
	//    Today's key uses DevelopmentEntryStore + TerrainEntryStore membership
	//    revisions. Generic form: add `fn revision(read) -> u64` to TerrainModel
	//    (Urbanization combines both; Durham returns its store revision).
	// 2. ids = view.cell_ids_overlapping(region)
	// 3. For each id with view.cell(id): keep if it draws at the viewer's level or
	//    seeds collision (today: `stream_banded_draws(value, level) || seeds_collision()`).
	//    -> TerrainCell needs `fn draws_at(&self, lod_ref: &LodRef) -> bool` (or
	//       `scene_lod_level`) so this stays generic. Add it to TerrainCell.
	// 4. Spawn/refresh from mesh_builder(), material(), chunk_pose(); tag the root
	//    with a presented marker; stamp collider source when seeds_collision().
	//    Today's markers: `PresentedPaddedTerrainScene(id)` / `PresentedTerrainScene(id)`.
	//    Collider cooking and TerrainSuperseded read those. Either make the marker
	//    generic (`PresentedTerrainCell<M>(Id)`) and update the collider systems,
	//    or keep a per-model marker via an associated const/type on TerrainCell.
	//    Pick the smallest change that keeps collider tests green.
	// 5. remove_stale(wanted)
}

// ── What does NOT come along ────────────────────────────────────────────────
//
// `sync_raw_terrain_replacements` hides raw Durham roots once padded replacements
// are cooked. With `present = false` in the world there are no raw roots, so it is
// a no-op there. Confirm (grep for `PresentedTerrainScene` spawns reachable from
// the world stack), then either delete it or leave it in the playground that still
// presents both models. Do not port it into the generic plugin.
//
// Water: `present_cells` in the developments playground also presents water.
// Find out what presents water in the world today before moving anything;
// water stays Durham-owned whichever way that lands.
//
// ── Invariants ──────────────────────────────────────────────────────────────
//
// - Same cells drawn, same colliders cooked, same ordering vs LodPresent/Collider sets.
// - richmond presentation tests and the durham collider tests keep passing.
