//! Present-layer [`LodScene`] for a grown forest grove tile ([#652](https://github.com/ramate-io/maybraid/issues/652)).
//!
//! `ChicoForest` stays select-only. Playground forest present registers this one
//! host type. Typed `/show orchard` keeps the concrete grove `LodScene`.

use bevy::ecs::template::template;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use bevy::shape::Aabb3d;
use lod::lod_ref::LodRef;
use lod::scene::LodScene;
use lod::{lod_host_scene_pending, LodSceneCulls, LodSceneLevel, LodSceneStatus, SceneChunk};

use crate::{ForestGroveTile, ForestLayer, LayerDropOut};

/// One presented forest grove: tile geometry plus the stacking layer.
#[derive(Clone, Component)]
pub struct ChicoGroveHost {
	pub tile: ForestGroveTile,
	pub layer: ForestLayer,
	visuals: crate::tree_field::GroveGpuVisuals,
}

impl ChicoGroveHost {
	pub fn new(tile: ForestGroveTile, layer: ForestLayer) -> Self {
		let visuals = crate::GroveGpuVisuals::for_tile(&tile, layer);
		Self { tile, layer, visuals }
	}

	pub fn drop_out(&self) -> LayerDropOut {
		LayerDropOut::for_stacked(self.layer, self.tile.is_tuft())
	}

	/// GPU field snapshot that replaces kit meshes when the experiment is on.
	pub fn gpu_visuals(&self) -> &crate::tree_field::GroveGpuVisuals {
		&self.visuals
	}

	/// GPU catalogs own plant visuals. The host still goes through LodScene
	/// with one High band covering the near-grove ring. Realizing High admits
	/// a catalog entry; leaving High retires it. Distance LOD stays in the GPU
	/// program.
	pub fn stamps_lod_scene_host(&self) -> bool {
		true
	}

	/// This tile as a nested host, pending at the level `lod_ref` (the
	/// viewer) sees it. GPU groves stamp the same LodScene shell; High
	/// fulfill streams the catalog entry.
	pub fn scene(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let host = self.clone();
		(
			lod_host_scene_pending(self.scene_lod_level(lod_ref), self.scene_bounds()),
			bsn! {
				~{Transform::IDENTITY}
				Visibility::default()
				template(move |_ctx| Ok(host.clone()))
			},
		)
	}

	fn catalog_scene(&self) -> Box<dyn Scene> {
		if let Some(field) = self.visuals.trees.clone() {
			return Box::new(bsn! { ~{field} });
		}
		if let Some(patch) = self.visuals.bushes.clone() {
			return Box::new(bsn! { ~{patch} });
		}
		if let Some(patch) = self.visuals.grass.clone() {
			return Box::new(bsn! { ~{patch} });
		}
		Box::new(Self::empty_scene())
	}

	fn level_scene(&self, level: LodSceneLevel) -> Box<dyn Scene> {
		if self.visuals.replaces_kits() && level == LodSceneLevel::High {
			self.catalog_scene()
		} else {
			Box::new(Self::empty_scene())
		}
	}

	fn empty_scene() -> impl Scene + 'static {
		bsn! {
			Visibility::Inherited
		}
	}

	fn empty_chunks() -> SceneChunk {
		SceneChunk::primitive(Self::empty_scene())
	}

	fn gpu_cpu_lod_high_only(&self) -> bool {
		self.visuals.replaces_kits()
	}

	fn tuft_high_chunks(&self, lod_ref: &LodRef, min_height_m: f32) -> Option<SceneChunk> {
		use ForestGroveTile::*;
		let chunks = match &self.tile {
			BraidGrass(g) => g.tuft_body().high_medium_chunks_dropping_shorter_than(
				lod_ref,
				LodSceneLevel::High,
				min_height_m,
			),
			CommonTufts(g) => g.tuft_body().high_medium_chunks_dropping_shorter_than(
				lod_ref,
				LodSceneLevel::High,
				min_height_m,
			),
			MonsterGrass(g) => g.tuft_body().high_medium_chunks_dropping_shorter_than(
				lod_ref,
				LodSceneLevel::High,
				min_height_m,
			),
			TallGrass(g) => g.tuft_body().high_medium_chunks_dropping_shorter_than(
				lod_ref,
				LodSceneLevel::High,
				min_height_m,
			),
			TropicalTufts(g) => g.high_chunks_dropping_shorter_than(lod_ref, min_height_m),
			WildGrass(g) => g.tuft_body().high_medium_chunks_dropping_shorter_than(
				lod_ref,
				LodSceneLevel::High,
				min_height_m,
			),
			_ => return None,
		};
		Some(chunks)
	}
}

impl LodScene for ChicoGroveHost {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.visuals
			.cpu_scene_lod_level()
			.unwrap_or_else(|| self.tile.scene_lod_level(lod_ref))
	}

	fn scene_lod_level_from_levels(&self, lod_refs: &[LodRef]) -> LodSceneLevel {
		if self.gpu_cpu_lod_high_only() {
			return LodSceneLevel::High;
		}
		lod_refs
			.iter()
			.map(|lod_ref| self.scene_lod_level(lod_ref))
			.max()
			.unwrap_or(LodSceneLevel::UltraLow)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.visuals
			.cpu_scene_lod_status()
			.unwrap_or_else(|| self.tile.scene_lod_status(lod_ref))
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		self.visuals
			.cpu_scene_lod_culls()
			.unwrap_or_else(|| self.tile.scene_lod_culls(lod_ref, current))
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.level_scene(level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		if self.visuals.replaces_kits() {
			return SceneChunk::primitive(self.level_scene(level));
		}
		let drop = self.drop_out();
		if drop.omits(level) {
			return Self::empty_chunks();
		}
		if drop.min_height_m > 0.0 && level == LodSceneLevel::High {
			if let Some(chunks) = self.tuft_high_chunks(lod_ref, drop.min_height_m) {
				return chunks;
			}
		}
		self.tile.scene_chunks_with_level(lod_ref, level)
	}

	fn scene_bounds(&self) -> Aabb3d {
		self.tile.scene_bounds()
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		self.scene(lod_ref)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::forest_world_sample;
	use crate::{ChicoGrove, ForestGroveKind, ForestGroveRecipe, GpuFieldKind};
	use bevy::asset::AssetPlugin;
	use bevy::camera::visibility::VisibilityPlugin;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::{
		App, Component, Entity, Mesh3d, MinimalPlugins, Transform, TransformPlugin, Vec3, With,
		World,
	};
	use bevy::scene::{ScenePlugin, WorldSceneExt};
	use bevy::world_serialization::{WorldAsset, WorldSerializationPlugin};
	use lod::{LodScene, LodSceneHost};
	use vegetation_bush_field::{
		collect_catalog_ops as collect_bush_catalog_ops, BushCatalogOp, BushCatalogOps,
		BushFieldSettings, BushGroveCatalog, BushPatch,
	};
	use vegetation_components::{FoliageNode, StickNode};
	use vegetation_grass_field::{
		collect_catalog_ops as collect_grass_catalog_ops, GrassCatalogOp, GrassCatalogOps,
		GrassFieldSettings, GrassGroveCatalog, GrassPatch,
	};
	use vegetation_groves::GroveExtent;
	use vegetation_tree_field::{
		collect_catalog_ops as collect_tree_catalog_ops, TreeCatalogOp, TreeCatalogOps,
		TreeFieldSettings, TreeGrove, TreeGroveCatalog,
	};

	fn lod_at(translation: Vec3) -> (Transform, Aabb3d) {
		(Transform::from_translation(translation), Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE))
	}

	fn with_lod<R>(translation: Vec3, f: impl FnOnce(&LodRef<'_>) -> R) -> R {
		with_lod_move(translation, translation, f)
	}

	fn with_lod_move<R>(previous: Vec3, current: Vec3, f: impl FnOnce(&LodRef<'_>) -> R) -> R {
		let (prev, bounds) = lod_at(previous);
		let curr = Transform::from_translation(current);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &prev,
			current_transform: &curr,
			bounds: &bounds,
		};
		f(&lod_ref)
	}

	fn grow_host(kind: ForestGroveKind, layer: ForestLayer) -> ChicoGroveHost {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let tile = ForestGroveRecipe::uniform(kind, extent).grow(&forest_world_sample());
		ChicoGroveHost::new(tile, layer)
	}

	fn assert_gpu_cpu_lod_high_only(host: &ChicoGroveHost) {
		use crate::GROVE_PRESENT_RADIUS_M;
		let near = Vec3::new(50.0, 20.0, 80.0);
		let far = Vec3::new(50.0, 20.0, GROVE_PRESENT_RADIUS_M + 500.0);
		assert!(near.length() < GROVE_PRESENT_RADIUS_M);
		assert!(far.z > GROVE_PRESENT_RADIUS_M);
		with_lod(near, |lod_ref| {
			assert_eq!(host.scene_lod_level(lod_ref), LodSceneLevel::High);
		});
		with_lod(far, |lod_ref| {
			assert_eq!(host.scene_lod_level(lod_ref), LodSceneLevel::High);
			assert_eq!(host.scene_lod_level_from_levels(&[lod_ref.clone()]), LodSceneLevel::High);
			let culls = host.scene_lod_culls(lod_ref, LodSceneLevel::High);
			assert!(!culls.should_cull(LodSceneLevel::High));
			assert!(culls.should_cull(LodSceneLevel::Medium));
			assert!(culls.should_cull(LodSceneLevel::Low));
			assert!(culls.should_cull(LodSceneLevel::UltraLow));
		});
		with_lod_move(near, far, |lod_ref| {
			assert_eq!(host.scene_lod_status(lod_ref), LodSceneStatus::Unchanged);
		});
		assert_eq!(host.scene_lod_level_from_levels(&[]), LodSceneLevel::High);
	}

	#[test]
	fn tufts_drop_out_omits_medium() {
		assert!(ForestLayer::Tufts.drop_out().omits(LodSceneLevel::Medium));
		assert!(ForestLayer::Tufts.drop_out().omits(LodSceneLevel::Low));
		assert!(ForestLayer::Tufts.drop_out().omits(LodSceneLevel::UltraLow));
		assert!(!ForestLayer::Tufts.drop_out().omits(LodSceneLevel::High));
		assert!(!ForestLayer::UpperCanopy.drop_out().omits(LodSceneLevel::Medium));
	}

	#[test]
	fn tufts_host_medium_is_empty_high_is_not() {
		let _lock = crate::GroveGpuVisuals::lock_admission();
		crate::GroveGpuVisuals::admit(false);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let grove = ChicoGrove::selected(
			extent,
			ForestLayer::Tufts,
			vec![ForestGroveRecipe::uniform(ForestGroveKind::CommonTufts, extent)],
		);
		let tile = grove.grow(&forest_world_sample()).remove(0);
		let host = ChicoGroveHost::new(tile, ForestLayer::Tufts);
		with_lod(Vec3::new(0.0, 20.0, 40.0), |lod_ref| {
			assert_eq!(
				host.scene_chunks_with_level(lod_ref, LodSceneLevel::Medium).total_primitives(),
				1
			);
			assert!(host.scene_chunks_with_level(lod_ref, LodSceneLevel::High).total_weight() > 1);
		});
	}

	#[test]
	fn gpu_host_suppresses_kit_chunks_for_trees_bushes_and_grass() {
		let _lock = crate::GroveGpuVisuals::lock_admission();
		crate::GroveGpuVisuals::admit(true);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let stacked = [
			(ForestGroveKind::TemperateLowerMassives, ForestLayer::LowerCanopy),
			(ForestGroveKind::LowBush, ForestLayer::Understory),
			(ForestGroveKind::TallGrass, ForestLayer::Tufts),
		];
		for (kind, layer) in stacked {
			let tile = ForestGroveRecipe::uniform(kind, extent).grow(&forest_world_sample());
			let host = ChicoGroveHost::new(tile, layer);
			assert!(host.gpu_visuals().replaces_kits(), "{kind:?}");
			with_lod(Vec3::new(0.0, 20.0, 40.0), |lod_ref| {
				assert_eq!(
					host.scene_chunks_with_level(lod_ref, LodSceneLevel::High).total_primitives(),
					1,
					"{kind:?} High must be one catalog primitive, not kit chunks"
				);
			});
		}
		crate::GroveGpuVisuals::admit(false);
	}

	fn stacked_gpu_kinds() -> [(ForestGroveKind, ForestLayer); 3] {
		[
			(ForestGroveKind::TemperateLowerMassives, ForestLayer::LowerCanopy),
			(ForestGroveKind::LowBush, ForestLayer::Understory),
			(ForestGroveKind::TallGrass, ForestLayer::Tufts),
		]
	}

	fn scene_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_plugins((
			TransformPlugin,
			AssetPlugin::default(),
			ScenePlugin,
			VisibilityPlugin,
			WorldSerializationPlugin,
		));
		app.init_asset::<WorldAsset>();
		app
	}

	fn entities_with<T: Component>(world: &mut World) -> Vec<Entity> {
		let mut query = world.query_filtered::<Entity, With<T>>();
		query.iter(world).collect()
	}

	fn assert_no_kit_or_plant_render_children(world: &mut World, kind: ForestGroveKind) {
		assert_eq!(
			entities_with::<StickNode>(world).len(),
			0,
			"{kind:?} High realize spawned StickNode kits"
		);
		assert_eq!(
			entities_with::<FoliageNode>(world).len(),
			0,
			"{kind:?} High realize spawned FoliageNode kits"
		);
		assert_eq!(
			entities_with::<Mesh3d>(world).len(),
			0,
			"{kind:?} High realize spawned Mesh3d kits"
		);
	}

	#[test]
	fn gpu_host_is_lod_scene_indexed_with_high_only_band() {
		let _lock = crate::GroveGpuVisuals::lock_admission();
		crate::GroveGpuVisuals::admit(true);
		for (kind, layer) in stacked_gpu_kinds() {
			let host = grow_host(kind, layer);
			assert!(host.gpu_visuals().replaces_kits(), "{kind:?}");
			assert!(host.stamps_lod_scene_host(), "{kind:?}");
			assert_gpu_cpu_lod_high_only(&host);
			with_lod(Vec3::new(0.0, 20.0, 40.0), |lod_ref| {
				let mut app = scene_app();
				app.world_mut()
					.spawn_scene(host.scene(lod_ref))
					.unwrap_or_else(|err| panic!("{kind:?} host scene: {err}"));
				let hosts = entities_with::<LodSceneHost>(app.world_mut());
				assert_eq!(hosts.len(), 1, "{kind:?} GPU host must stamp LodSceneHost");
				assert_eq!(
					app.world().get::<LodSceneLevel>(hosts[0]).copied(),
					Some(LodSceneLevel::High),
					"{kind:?} GPU host must pending High"
				);
				assert!(
					entities_with::<TreeGrove>(app.world_mut()).is_empty(),
					"{kind:?} catalog is High fulfill, not host.scene()"
				);
				assert!(entities_with::<BushPatch>(app.world_mut()).is_empty(), "{kind:?}");
				assert!(entities_with::<GrassPatch>(app.world_mut()).is_empty(), "{kind:?}");
			});
		}
		crate::GroveGpuVisuals::admit(false);
	}

	#[test]
	fn gpu_host_high_realize_and_unrealize_is_one_admit_and_retire() {
		let _lock = crate::GroveGpuVisuals::lock_admission();
		crate::GroveGpuVisuals::admit(true);
		for (kind, layer) in stacked_gpu_kinds() {
			let host = grow_host(kind, layer);
			assert!(host.gpu_visuals().replaces_kits(), "{kind:?}");
			with_lod(Vec3::new(0.0, 20.0, 40.0), |lod_ref| {
				let mut app = scene_app();
				match crate::GroveGpuVisuals::field_kind(&host.tile, host.layer) {
					GpuFieldKind::Tree => {
						app.init_resource::<TreeFieldSettings>()
							.init_resource::<TreeGroveCatalog>()
							.init_resource::<TreeCatalogOps>();
					}
					GpuFieldKind::Bush => {
						app.init_resource::<BushFieldSettings>()
							.init_resource::<BushGroveCatalog>()
							.init_resource::<BushCatalogOps>();
					}
					GpuFieldKind::Grass => {
						app.init_resource::<GrassFieldSettings>()
							.init_resource::<GrassGroveCatalog>()
							.init_resource::<GrassCatalogOps>();
					}
				}
				app.world_mut()
					.spawn_scene(host.scene_with_level(lod_ref, LodSceneLevel::High))
					.unwrap_or_else(|err| panic!("{kind:?} High realize: {err}"));
				assert_no_kit_or_plant_render_children(app.world_mut(), kind);
				match crate::GroveGpuVisuals::field_kind(&host.tile, host.layer) {
					GpuFieldKind::Tree => {
						let ids = entities_with::<TreeGrove>(app.world_mut());
						assert_eq!(ids.len(), 1, "{kind:?} High realize must spawn one TreeGrove");
						app.world_mut()
							.run_system_once(collect_tree_catalog_ops)
							.expect("tree catalog collect");
						let catalog = app.world().resource::<TreeGroveCatalog>();
						assert_eq!(catalog.live_grove_count(), 1, "{kind:?}");
						let admits = app
							.world()
							.resource::<TreeCatalogOps>()
							.ops
							.iter()
							.filter(|op| matches!(op, TreeCatalogOp::Admit { .. }))
							.count();
						assert_eq!(admits, 1, "{kind:?} High realize must emit one Admit");
						app.world_mut().entity_mut(ids[0]).despawn();
						app.world_mut()
							.run_system_once(collect_tree_catalog_ops)
							.expect("tree catalog retire");
						assert_eq!(
							app.world().resource::<TreeGroveCatalog>().live_grove_count(),
							0
						);
						let retires = app
							.world()
							.resource::<TreeCatalogOps>()
							.ops
							.iter()
							.filter(|op| matches!(op, TreeCatalogOp::Retire { .. }))
							.count();
						assert_eq!(retires, 1, "{kind:?} High unrealize must emit one Retire");
					}
					GpuFieldKind::Bush => {
						let ids = entities_with::<BushPatch>(app.world_mut());
						assert_eq!(ids.len(), 1, "{kind:?} High realize must spawn one BushPatch");
						app.world_mut()
							.run_system_once(collect_bush_catalog_ops)
							.expect("bush catalog collect");
						assert_eq!(
							app.world().resource::<BushGroveCatalog>().live_grove_count(),
							1
						);
						let admits = app
							.world()
							.resource::<BushCatalogOps>()
							.ops
							.iter()
							.filter(|op| matches!(op, BushCatalogOp::Admit { .. }))
							.count();
						assert_eq!(admits, 1, "{kind:?} High realize must emit one Admit");
						app.world_mut().entity_mut(ids[0]).despawn();
						app.world_mut()
							.run_system_once(collect_bush_catalog_ops)
							.expect("bush catalog retire");
						assert_eq!(
							app.world().resource::<BushGroveCatalog>().live_grove_count(),
							0
						);
						let retires = app
							.world()
							.resource::<BushCatalogOps>()
							.ops
							.iter()
							.filter(|op| matches!(op, BushCatalogOp::Retire { .. }))
							.count();
						assert_eq!(retires, 1, "{kind:?} High unrealize must emit one Retire");
					}
					GpuFieldKind::Grass => {
						let ids = entities_with::<GrassPatch>(app.world_mut());
						assert_eq!(ids.len(), 1, "{kind:?} High realize must spawn one GrassPatch");
						app.world_mut()
							.run_system_once(collect_grass_catalog_ops)
							.expect("grass catalog collect");
						assert_eq!(
							app.world().resource::<GrassGroveCatalog>().live_grove_count(),
							1
						);
						let admits = app
							.world()
							.resource::<GrassCatalogOps>()
							.ops
							.iter()
							.filter(|op| matches!(op, GrassCatalogOp::Admit { .. }))
							.count();
						assert_eq!(admits, 1, "{kind:?} High realize must emit one Admit");
						app.world_mut().entity_mut(ids[0]).despawn();
						app.world_mut()
							.run_system_once(collect_grass_catalog_ops)
							.expect("grass catalog retire");
						assert_eq!(
							app.world().resource::<GrassGroveCatalog>().live_grove_count(),
							0
						);
						let retires = app
							.world()
							.resource::<GrassCatalogOps>()
							.ops
							.iter()
							.filter(|op| matches!(op, GrassCatalogOp::Retire { .. }))
							.count();
						assert_eq!(retires, 1, "{kind:?} High unrealize must emit one Retire");
					}
				}
			});
		}
		crate::GroveGpuVisuals::admit(false);
	}

	#[test]
	fn gpu_host_cpu_lod_is_high_only_and_culls_far_bands() {
		let _lock = crate::GroveGpuVisuals::lock_admission();
		crate::GroveGpuVisuals::admit(true);
		let stacked = [
			(ForestGroveKind::TemperateLowerMassives, ForestLayer::LowerCanopy),
			(ForestGroveKind::LowBush, ForestLayer::Understory),
			(ForestGroveKind::TallGrass, ForestLayer::Tufts),
		];
		for (kind, layer) in stacked {
			let host = grow_host(kind, layer);
			assert!(host.gpu_visuals().replaces_kits(), "{kind:?}");
			assert_gpu_cpu_lod_high_only(&host);
		}
		crate::GroveGpuVisuals::admit(false);
	}

	#[test]
	fn legacy_host_still_swaps_cpu_lod_bands() {
		use crate::GROVE_PRESENT_RADIUS_M;
		let _lock = crate::GroveGpuVisuals::lock_admission();
		crate::GroveGpuVisuals::admit(false);
		let host = grow_host(ForestGroveKind::TemperateLowerMassives, ForestLayer::LowerCanopy);
		assert!(!host.gpu_visuals().replaces_kits());
		let near = Vec3::new(50.0, 20.0, 80.0);
		let far = Vec3::new(50.0, 20.0, GROVE_PRESENT_RADIUS_M + 500.0);
		let near_level = with_lod(near, |lod_ref| host.scene_lod_level(lod_ref));
		let far_level = with_lod(far, |lod_ref| host.scene_lod_level(lod_ref));
		assert_eq!(near_level, LodSceneLevel::High);
		assert_ne!(far_level, LodSceneLevel::High);
		with_lod_move(near, far, |lod_ref| {
			assert_eq!(host.scene_lod_status(lod_ref), LodSceneStatus::Changed(far_level));
		});
		assert_eq!(host.scene_lod_level_from_levels(&[]), LodSceneLevel::UltraLow);
	}
}
