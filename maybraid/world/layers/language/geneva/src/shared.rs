//! Geneva on the shared HCSG runtime ([`lod::hcsg::shared`]): large language
//! tiles generate on the worker from the [`LanguageWorldSeed`] root within
//! channel `C`'s window, and the frame names what the world under it has
//! published near the viewer into the [`LanguageOverlay`].
//!
//! A session starts by advancing the epoch, then [`GenevaRoots::reset`].

use std::marker::PhantomData;

use bevy::ecs::system::{SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::DVec3;
use bevy::prelude::*;
use chico::ForestGround;
use lod::gen::{Id, OriginalId};
use lod::hcsg::shared::{
	self, Busy, GenerationContext, GenerationPlugin, HcsgBounds, HcsgRegions, HcsgStorage,
	HcsgSystems,
};
use lod::hcsg::universal_bounds;
use lod::LodViewer;
use richmond::DiscoverablePlaceIndex;

use crate::index::{
	large_tiles_overlapping, LanguageIndex, LanguageSourceDeps, LanguageWorldSeed, SourceClass,
};
use crate::present::{present_language_overlay, LanguageOverlay};
use crate::sources::{window_tiles, NameSources, NamingRegion, NAME_WINDOW_QUANT_M};
use crate::tiles::{large_tile_aabb, large_tile_index, large_tile_origin, LargeTile, LARGE_TILE};

/// Language tiles stay generated within this XZ radius of the viewer.
const LANGUAGE_GENERATE_RADIUS: f32 = 40_000.0;
/// Nearby entity naming. Language tiles keep the broad generate window.
pub(crate) const LANGUAGE_NAME_RADIUS: f32 = 400.0;
const LANGUAGE_NAME_HYSTERESIS: f32 = 80.0;
/// Modest per-frame assignment budget. Overlay rebuild is presentation's job.
const ASSIGN_BUDGET: usize = 32;

impl shared::GenerationScheme for LargeTile {
	fn original_ids_for(_cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		large_tiles_overlapping(region).map(|(ix, iz)| OriginalId(LargeTile::id(ix, iz))).collect()
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let (ix, iz) = LargeTile::index_of(id)?;
		let seed = cx.get::<LanguageWorldSeed>(Id::Universal)?;
		Some((LargeTile::generate(seed.0, ix, iz), large_tile_aabb(ix, iz)))
	}
}

/// The large tiles within [`LANGUAGE_GENERATE_RADIUS`] of the [`LodViewer`],
/// as one box on the tile grid: it changes only when the window's edge
/// crosses a tile line.
pub struct LanguageNeighborhood;

impl LanguageNeighborhood {
	fn around(viewer: Vec3) -> Aabb3d {
		let r = LANGUAGE_GENERATE_RADIUS;
		let (min_x, min_z) = large_tile_origin(
			large_tile_index(viewer.x - r),
			large_tile_index(viewer.z - r),
		);
		let (max_x, max_z) = large_tile_origin(
			large_tile_index(viewer.x + r) + 1,
			large_tile_index(viewer.z + r) + 1,
		);
		Aabb3d::from_min_max(Vec3::new(min_x, -1.0, min_z), Vec3::new(max_x, 1.0, max_z))
	}
}

impl HcsgBounds for LanguageNeighborhood {
	type Param = Query<'static, 'static, &'static Transform, With<LodViewer>>;

	fn regions(viewers: &SystemParamItem<Self::Param>) -> Vec<Aabb3d> {
		viewers.iter().next().map(|viewer| Self::around(viewer.translation)).into_iter().collect()
	}

	fn focus(viewers: &SystemParamItem<Self::Param>) -> Option<Vec3> {
		viewers.iter().next().map(|viewer| viewer.translation)
	}
}

/// Every value Geneva owns in the shared storage: its tiles and its root.
pub struct GenevaNodes;

const TILE_SCALE: DVec3 = DVec3::new(LARGE_TILE as f64, 2.0, LARGE_TILE as f64);

impl GenevaNodes {
	pub fn configure(storage: &HcsgStorage) {
		storage.configure::<LargeTile>(TILE_SCALE);
	}

	/// Within a restart, after the epoch has advanced.
	pub fn clear(storage: &HcsgStorage) {
		storage.clear::<LargeTile>();
		storage.clear::<LanguageWorldSeed>();
	}
}

/// Geneva's root resource: the language world seed.
#[derive(SystemParam)]
pub struct GenevaRoots<'w> {
	seed: Res<'w, LanguageWorldSeed>,
}

impl GenevaRoots<'_> {
	/// Clears Geneva's stores and seeds the world seed. Within a restart,
	/// after the epoch has advanced.
	///
	/// [`LanguageIndex`] and [`LanguageOverlay`] belong to the root they were
	/// filled under, so they start over once naming reads the new one.
	pub fn reset(&self, storage: &HcsgStorage) {
		GenevaNodes::clear(storage);
		storage.seed(*self.seed, universal_bounds());
	}
}

/// Channel `C`'s latest regions as naming last read them. Empty idles naming.
#[derive(Resource, Default)]
struct LanguageWindow {
	boxes: Vec<Aabb3d>,
	focus: Option<Vec3>,
}

impl LanguageWindow {
	fn bounds(&self) -> Option<Aabb3d> {
		self.boxes.iter().copied().reduce(|a, b| {
			Aabb3d::from_min_max(Vec3::from(a.min.min(b.min)), Vec3::from(a.max.max(b.max)))
		})
	}
}

fn track_window<C: Send + Sync + 'static>(
	mut regions: MessageReader<HcsgRegions<C>>,
	mut window: ResMut<LanguageWindow>,
) {
	if let Some(latest) = regions.read().last() {
		window.boxes = latest.boxes.clone();
		window.focus = latest.focus;
	}
}

fn collect_radius() -> f32 {
	LANGUAGE_NAME_RADIUS + NAME_WINDOW_QUANT_M * 0.5
}

fn retain_radius() -> f32 {
	collect_radius() + LANGUAGE_NAME_HYSTERESIS
}

/// Names within [`LANGUAGE_NAME_RADIUS`] of the viewer, over the tiles the
/// worker has published in the window. Rescans a source only when its
/// revision moves or the quantized naming origin does.
fn name_nearby<W: ForestGround>(
	window: Res<LanguageWindow>,
	viewers: Query<&Transform, With<LodViewer>>,
	storage: Res<HcsgStorage>,
	places: Option<Res<DiscoverablePlaceIndex>>,
	mut index: ResMut<LanguageIndex>,
) {
	let Some(tile_region) = window.bounds() else {
		index.end_session();
		return;
	};
	let root = match storage.try_entry::<LanguageWorldSeed>(Id::Universal) {
		Ok(Some(root)) => root,
		Ok(None) => {
			index.end_session();
			return;
		}
		Err(Busy) => return,
	};
	let Some(origin) = viewers.iter().next().map(|viewer| viewer.translation).or(window.focus)
	else {
		return;
	};
	index.begin_session(root.version);
	let seed = root.value.0;
	let sources = NameSources::<W>::new(&storage, places.as_deref());
	let Ok(revisions) = sources.revisions() else {
		return;
	};
	let naming = NamingRegion::around(origin.xz(), collect_radius(), retain_radius());
	let deps = LanguageSourceDeps::from_windows(revisions, seed, tile_region, naming.origin);
	if index.source_deps() != Some(deps) {
		let tiles = window_tiles(&window.boxes);
		if rescan(&mut index, &sources, &tiles, naming, deps).is_err() {
			return;
		}
		index.note_source_deps(deps);
	}
	index.assign_budgeted(seed, ASSIGN_BUDGET);
}

/// Admits newly published tiles and queues the sources whose inputs moved
/// since the index's last deps. Nothing is queued unless every read succeeds.
fn rescan<W: ForestGround>(
	index: &mut LanguageIndex,
	sources: &NameSources<W>,
	tiles: &[(i32, i32)],
	naming: NamingRegion,
	deps: LanguageSourceDeps,
) -> Result<(), Busy> {
	let prev = index.source_deps();
	let tiles_moved = !prev.is_some_and(|prev| prev.tiles_match(deps));
	if tiles_moved {
		let admitted = sources.tiles(tiles, index)?;
		index.sync_tiles(deps.seed, tiles, admitted);
	}
	let window_moved = tiles_moved || !prev.is_some_and(|prev| prev.naming_window_match(deps));
	let moved = |revision: fn(&LanguageSourceDeps) -> u64| {
		window_moved || prev.as_ref().map(revision) != Some(revision(&deps))
	};
	let groves = moved(|deps| deps.revisions.forest).then(|| sources.groves(naming, index));
	let geography = moved(|deps| deps.revisions.terrain).then(|| sources.geography(naming, index));
	let urban = moved(|deps| deps.revisions.urban).then(|| sources.urban(naming, index));
	let places = moved(|deps| deps.revisions.places).then(|| sources.places(naming, index));
	let (groves, geography, urban) =
		(groves.transpose()?, geography.transpose()?, urban.transpose()?);
	for (snapshot, class) in [
		(groves, SourceClass::Vegetation),
		(geography, SourceClass::Geography),
		(urban, SourceClass::Urban),
	] {
		if let Some(snapshot) = snapshot {
			index.queue_feature_snapshot(snapshot, class);
		}
	}
	if let Some(places) = places {
		index.queue_place_snapshot(places);
	}
	Ok(())
}

/// Geneva over ground `W`: language tiles within channel `C`'s regions,
/// names near the [`LodViewer`] for what `W`, urbanization, Durham and
/// Richmond's [`DiscoverablePlaceIndex`] have published, and the
/// [`LanguageOverlay`] they present to.
///
/// `W` is the ground groves grow on, as for Chico's presentation: the
/// Discovery world passes `Urbanization<Richmond<OnTerrain<Durham>>>`. An
/// empty region set on `C` idles naming and empties the overlay. The world
/// seed comes from [`LanguageWorldSeed`], [`LanguageConfig::world_defaults`]
/// unless inserted first.
///
/// [`LanguageConfig::world_defaults`]: crate::LanguageConfig::world_defaults
pub struct GenevaPlugin<C, W>(PhantomData<fn() -> (C, W)>);

impl<C, W> Default for GenevaPlugin<C, W> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static, W: ForestGround> Plugin for GenevaPlugin<C, W> {
	fn build(&self, app: &mut App) {
		let storage = app.world_mut().get_resource_or_init::<HcsgStorage>().clone();
		GenevaNodes::configure(&storage);
		app.init_resource::<LanguageWorldSeed>()
			.init_resource::<LanguageIndex>()
			.init_resource::<LanguageOverlay>()
			.init_resource::<LanguageWindow>()
			.add_plugins(GenerationPlugin::<C, LargeTile>::default())
			.add_systems(
				Update,
				(track_window::<C>, name_nearby::<W>, present_language_overlay)
					.chain()
					.after(HcsgSystems),
			);
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use bevy::scene::ScenePlugin;
	use bevy::state::app::StatesPlugin;
	use chico::{ChicoPresentationPlugin, ChicoRoots, ForestSelection, GroveNeighborhood};
	use durham::{
		fine_patch_cell_layout, Durham, DurhamRoots, TerrainConfig, TerrainMeshLodBand,
		TerrainPresentationAssets, TerrainStampConfigs, WaterPresentationAssets, WatershedConfigs,
	};
	use lod::hcsg::shared::{Gated, HcsgBoundsPlugin, HcsgDemand, HcsgGate};
	use lod::lod_ref::LodNodePose;
	use richmond::{AuthoredDevelopments, DevelopmentConfig, DevelopmentSites, Richmond, RichmondRoots};
	use terrain_layer_model::OnTerrain;
	use urbanization_cells::UrbanizationSelection;
	use urbanization_layer_model::Urbanization;

	use super::*;
	use crate::NameKey;

	type Ground = OnTerrain<Durham>;
	type Urban = Urbanization<Richmond<Ground>>;

	const IDLE: Duration = Duration::from_secs(300);

	#[derive(Resource)]
	struct Restart(bool);

	/// Whether the test's language channel is open.
	#[derive(Resource)]
	struct Open(bool);

	struct Opened;

	impl HcsgGate for Opened {
		type Param = Res<'static, Open>;

		fn open(open: &SystemParamItem<Self::Param>) -> bool {
			open.0
		}
	}

	type Window = Gated<Opened, LanguageNeighborhood>;

	fn restart_language(
		geneva: GenevaRoots,
		storage: Res<HcsgStorage>,
		demand: Res<HcsgDemand>,
		mut pending: ResMut<Restart>,
	) {
		if std::mem::take(&mut pending.0) {
			demand.advance_epoch();
			geneva.reset(&storage);
		}
	}

	fn spawn_viewer(app: &mut App, at: Vec3) {
		let at = Transform::from_translation(at);
		app.world_mut().spawn((LodViewer, at, LodNodePose { previous: at, current: at }));
	}

	/// Geneva alone over an empty world, viewed from `at`.
	fn language_app(at: Vec3) -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin))
			.add_plugins((AssetPlugin::default(), ScenePlugin))
			.insert_resource(Open(true))
			.insert_resource(Restart(true))
			.add_plugins(HcsgBoundsPlugin::<Window>::default())
			.add_plugins(GenevaPlugin::<Window, Urban>::default())
			.add_systems(Update, restart_language.before(HcsgSystems));
		app.finish();
		app.cleanup();
		spawn_viewer(&mut app, at);
		app
	}

	fn settle(app: &mut App) -> anyhow::Result<()> {
		for _ in 0..2 {
			app.update();
			let demand = app.world().resource::<HcsgDemand>().clone();
			anyhow::ensure!(demand.wait_idle(IDLE), "worker did not go idle");
		}
		for _ in 0..8 {
			app.update();
		}
		Ok(())
	}

	fn region_names(app: &App) -> Vec<(i32, i32, String)> {
		let mut names: Vec<_> = app
			.world()
			.resource::<LanguageOverlay>()
			.names
			.iter()
			.filter_map(|name| match name.key {
				NameKey::Region { ix, iz } => Some((ix, iz, name.surface.clone())),
				_ => None,
			})
			.collect();
		names.sort();
		names
	}

	/// The tiles within the generate radius of the origin.
	fn origin_window() -> Vec<(i32, i32)> {
		(-2..=1).flat_map(|ix| (-2..=1).map(move |iz| (ix, iz))).collect()
	}

	#[test]
	fn the_window_generates_tiles_and_names_their_regions() -> anyhow::Result<()> {
		let mut app = language_app(Vec3::ZERO);
		settle(&mut app)?;

		let storage = app.world().resource::<HcsgStorage>().clone();
		for (ix, iz) in origin_window() {
			let tile = storage
				.get::<LargeTile>(LargeTile::id(ix, iz))
				.ok_or_else(|| anyhow::anyhow!("tile ({ix}, {iz}) was not generated"))?;
			assert_eq!(*tile, LargeTile::generate(LanguageWorldSeed::default().0, ix, iz));
		}
		assert!(storage.get::<LargeTile>(LargeTile::id(3, 0)).is_none(), "beyond the window");

		let overlay = app.world().resource::<LanguageOverlay>();
		assert_eq!(overlay.large_tiles.len(), origin_window().len());
		let named: Vec<_> = region_names(&app).into_iter().map(|(ix, iz, _)| (ix, iz)).collect();
		assert_eq!(named, origin_window(), "every tile in the window names its region");
		assert!(region_names(&app).iter().all(|(_, _, surface)| !surface.is_empty()));
		Ok(())
	}

	#[test]
	fn a_closed_channel_idles_naming_and_empties_the_overlay() -> anyhow::Result<()> {
		let mut app = language_app(Vec3::ZERO);
		settle(&mut app)?;
		assert!(!region_names(&app).is_empty());

		app.world_mut().resource_mut::<Open>().0 = false;
		settle(&mut app)?;
		assert_eq!(*app.world().resource::<LanguageOverlay>(), LanguageOverlay {
			epoch: app.world().resource::<LanguageIndex>().epoch,
			..LanguageOverlay::default()
		});
		assert!(app.world().resource::<LanguageIndex>().session().is_none());
		let epoch = app.world().resource::<LanguageIndex>().epoch;
		app.update();
		assert_eq!(app.world().resource::<LanguageIndex>().epoch, epoch, "idle stays idle");

		app.world_mut().resource_mut::<Open>().0 = true;
		settle(&mut app)?;
		assert_eq!(region_names(&app).len(), origin_window().len());
		Ok(())
	}

	#[test]
	fn a_restart_with_a_new_seed_renames_every_region() -> anyhow::Result<()> {
		let mut app = language_app(Vec3::ZERO);
		settle(&mut app)?;
		let before = region_names(&app);

		app.insert_resource(LanguageWorldSeed(99)).insert_resource(Restart(true));
		settle(&mut app)?;
		let after = region_names(&app);

		assert_eq!(after.len(), before.len());
		assert_ne!(after, before, "the new seed composes new regions");
		let storage = app.world().resource::<HcsgStorage>().clone();
		let tile = storage
			.get::<LargeTile>(LargeTile::id(0, 0))
			.ok_or_else(|| anyhow::anyhow!("tile"))?;
		assert_eq!(*tile, LargeTile::generate(99, 0, 0));
		Ok(())
	}

	#[test]
	fn crossing_a_tile_line_moves_the_window() -> anyhow::Result<()> {
		let mut app = language_app(Vec3::new(9_000.0, 0.0, 0.0));
		settle(&mut app)?;
		let named: Vec<_> = region_names(&app).into_iter().map(|(ix, iz, _)| (ix, iz)).collect();
		assert_eq!(named, origin_window());

		let mut viewers = app.world_mut().query_filtered::<&mut Transform, With<LodViewer>>();
		*viewers.single_mut(app.world_mut())? = Transform::from_xyz(16_000.0, 0.0, 0.0);
		settle(&mut app)?;
		let named: Vec<_> = region_names(&app).into_iter().map(|(ix, iz, _)| (ix, iz)).collect();
		let moved: Vec<_> = (-1..=2).flat_map(|ix| (-2..=1).map(move |iz| (ix, iz))).collect();
		assert_eq!(named, moved, "regions follow the window; tiles behind it stay admitted");
		assert_eq!(app.world().resource::<LanguageOverlay>().large_tiles.len(), 20);
		Ok(())
	}

	fn restart_world(
		durham: DurhamRoots,
		richmond: RichmondRoots,
		chico: ChicoRoots,
		geneva: GenevaRoots,
		storage: Res<HcsgStorage>,
		demand: Res<HcsgDemand>,
		mut pending: ResMut<Restart>,
	) {
		if std::mem::take(&mut pending.0) {
			demand.advance_epoch();
			durham.reset(&storage);
			richmond.reset::<Ground>(&storage);
			chico.reset::<Urban>(&storage);
			geneva.reset(&storage);
		}
	}

	/// Groves on Chico's 2×2 fine patch at the origin, named by Geneva.
	fn forested_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin))
			.add_plugins((AssetPlugin::default(), ScenePlugin))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.init_asset::<bevy::world_serialization::WorldAsset>()
			.insert_resource(fine_patch_cell_layout(1, IVec2::new(-1, -1)))
			.insert_resource(TerrainStampConfigs::from_world_seed(1))
			.insert_resource(WatershedConfigs::default().with_seed(1))
			.insert_resource(TerrainPresentationAssets {
				config: TerrainConfig::new(1),
				material: Handle::default(),
				lod_bands: vec![TerrainMeshLodBand { max_radius_cells: 1, res_2: 2 }],
				outer_add_walls: false,
				fine_grid_max_radius: Some(1),
				macro_seam_half_extents: Vec::new(),
				macro_cell_min_size: None,
				macro_res_2: None,
			})
			.insert_resource(WaterPresentationAssets { material: Handle::default() })
			.insert_resource(DevelopmentConfig {
				sites: DevelopmentSites::Authored,
				..DevelopmentConfig::default()
			})
			.init_resource::<AuthoredDevelopments>()
			.init_resource::<UrbanizationSelection>()
			.insert_resource(ForestSelection {
				layering: Some(chico::LayeringKind::LushJungle),
				..ForestSelection::default()
			})
			.insert_resource(Open(true))
			.insert_resource(Restart(true))
			.add_plugins((
				HcsgBoundsPlugin::<GroveNeighborhood>::default(),
				HcsgBoundsPlugin::<Window>::default(),
			))
			.add_plugins(ChicoPresentationPlugin::<GroveNeighborhood, Urban>::default())
			.add_plugins(GenevaPlugin::<Window, Urban>::default())
			.add_systems(Update, restart_world.before(HcsgSystems));
		app.finish();
		app.cleanup();
		spawn_viewer(&mut app, Vec3::ZERO);
		app
	}

	#[test]
	fn nearby_groves_are_named_in_their_tiles_languages() -> anyhow::Result<()> {
		let mut app = forested_app();
		settle(&mut app)?;
		for _ in 0..64 {
			app.update();
		}

		let overlay = app.world().resource::<LanguageOverlay>().clone();
		let groves: Vec<_> = overlay
			.names
			.iter()
			.filter(|name| matches!(name.key, NameKey::Forest(_) | NameKey::Grove(_)))
			.collect();
		assert!(!groves.is_empty(), "the patch's forests and groves are named");
		let window = NamingRegion::around(Vec2::ZERO, collect_radius(), retain_radius());
		for name in &groves {
			assert!(!name.surface.is_empty());
			let extent = Aabb3d::from_min_max(
				Vec3::new(name.extent.min.x, -1.0, name.extent.min.y),
				Vec3::new(name.extent.max.x, 1.0, name.extent.max.y),
			);
			assert!(window.retains_bounds(extent), "{:?} named outside the window", name.key);
		}

		let storage = app.world().resource::<HcsgStorage>().clone();
		let grown = storage.overlapping::<chico::GrownGrove<Urban>>(window.query_aabb());
		let named_groves = groves.iter().filter(|name| matches!(name.key, NameKey::Grove(_))).count();
		assert!(named_groves > 0 && named_groves <= grown.len(), "only grown groves are named");
		Ok(())
	}
}
