//! One-time ordinary-to-packed handoff and revision/retirement ownership.

use super::view_policy::PackedViewPolicy;
use crate::ChicoGroveHost;
use crate::packed::cache::*;
use crate::packed::gpu::PackedBatchInstance;
use crate::packed::kit::{KitState, PackedKitMeshes};
use crate::packed::material::{PackedLeafMaterial, PackedShadowMaterial, PackedStickMaterial};
use crate::packed::mode::PackMode;
use crate::packed::select::select_band;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{NoAutoAabb, RenderLayers};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::batching::NoAutomaticBatching;
use lod::{LodLevelRoot, LodLevelRoots, LodSceneHost, LodSceneLevel, VisualMaterialKind};
use std::sync::Arc;
use vegetation_components::{CanopyShadowMode, SHADOW_PROXY_RENDER_LAYER};

#[derive(Component)]
pub struct PackedGroveCell;

#[derive(Component)]
pub struct PackedGroveProxy;

fn ordinary_level(world: &mut World, payload: &TilePayload) -> LodSceneLevel {
	let mut cameras = world
		.query_filtered::<(&GlobalTransform, &Camera, Option<&RenderLayers>), With<Camera3d>>();
	cameras
		.iter(world)
		.filter(|(_, camera, layers)| PackedViewPolicy::is_scene_camera(camera, *layers))
		.map(|(xf, _, _)| select_band(payload.structural_lod, xf.translation()))
		.max()
		.unwrap_or(LodSceneLevel::UltraLow)
}

#[derive(Component)]
pub(crate) struct RestoringOrdinary;

fn restore_ordinary(world: &mut World, entity: Entity, tile: &ResidentTile) {
	let active = tile.signal.phase.load(std::sync::atomic::Ordering::Acquire) == ACTIVE;
	if !active {
		tile.signal.retire();
		for &proxy in &tile.proxies {
			let _ = world.despawn(proxy);
		}
		return;
	}
	if world.get::<RestoringOrdinary>(entity).is_some() {
		return;
	}
	// Keep packed draws and collision owners until the ordinary lazy scene has
	// completed. Its new level-root bag stays hidden during this reverse handoff.
	let source = world
		.get::<ChicoGroveHost>(entity)
		.cloned()
		.unwrap_or_else(|| tile.original.clone());
	let level = ordinary_level(world, &tile.payload);
	if let Ok(mut host) = world.get_entity_mut(entity) {
		host.insert((
			RestoringOrdinary,
			source,
			LodSceneHost,
			level,
			lod::LodLevelSpawnRequest { level },
		));
	}
}

pub fn hide_restoring_bag(
	insert: On<Add<LodLevelRoots>>,
	parents: Query<&ChildOf>,
	restoring: Query<(), With<RestoringOrdinary>>,
	mut commands: Commands,
) {
	if parents
		.get(insert.entity)
		.is_ok_and(|parent| restoring.contains(parent.parent()))
	{
		commands.entity(insert.entity).insert(Visibility::Hidden);
	}
}

fn finish_restore(world: &mut World, entity: Entity, tile: &ResidentTile) -> bool {
	if tile.signal.phase.load(std::sync::atomic::Ordering::Acquire) != ACTIVE {
		return true;
	}
	let Some(children) = world.get::<Children>(entity) else {
		return false;
	};
	let Some(bag) = children.iter().find(|child| world.get::<LodLevelRoots>(*child).is_some())
	else {
		return false;
	};
	let mut pending = vec![bag];
	let mut streamed = false;
	while let Some(node) = pending.pop() {
		if world.get::<lod::LodLevelRootPending>(node).is_some()
			|| world.get::<lod::LodLazyPending>(node).is_some()
		{
			return false;
		}
		streamed |= world.get::<lod::LodLevelRootStreamed>(node).is_some();
		if let Some(children) = world.get::<Children>(node) {
			pending.extend(children.iter());
		}
	}
	if !streamed {
		return false;
	}
	if matches!(
		world.get::<LodSceneLevel>(entity),
		Some(LodSceneLevel::High | LodSceneLevel::Medium)
	) {
		if !crate::stick_physics::ordinary_packed_physics_ready(world, bag) {
			return false;
		}
	}
	tile.signal.retire();
	for &proxy in &tile.proxies {
		let _ = world.despawn(proxy);
	}
	crate::stick_physics::remove_packed_sources(world, entity);
	world.entity_mut(bag).insert(Visibility::Inherited);
	world.entity_mut(entity).remove::<(RestoringOrdinary, PackedGroveCell)>();
	true
}

fn activate_tile(world: &mut World, entity: Entity, tile: &ResidentTile) {
	if lod::scene::host::lod_scene_host_or_ancestor_hidden_world(world, entity) {
		return;
	}
	if !tile.signal.is_ready() {
		return;
	}
	// Stop the ordinary producer before removing its level-root bag. Old queued
	// fulfillment jobs reference despawned roots and cannot publish new visuals.
	let roots = world
		.get::<Children>(entity)
		.map(|children| children.iter().collect::<Vec<_>>())
		.unwrap_or_default();
	if let Ok(mut host) = world.get_entity_mut(entity) {
		host.remove::<ChicoGroveHost>()
			.remove::<lod::scene::host::LodLevelSpawnRequest>()
			.insert(PackedGroveCell);
	}
	crate::stick_physics::attach_packed_source(world, entity, tile.original.clone());
	for child in roots {
		if world.get::<LodLevelRoots>(child).is_some() || world.get::<LodLevelRoot>(child).is_some()
		{
			let _ = world.despawn(child);
		}
	}
	// Per-plant semantic sources and existing compounds now belong to the cell.
	let _ = tile.signal.activate();
}

fn spawn_proxies(
	world: &mut World,
	entity: Entity,
	tile: &mut ResidentTile,
	kits: &PackedKitMeshes,
	policy: &PackedViewPolicy,
) -> bool {
	// Validate every mesh bound before any proxy or new pass signal is published.
	let Some(bounds) = tile.payload.geometric_bounds(kits) else { return false };
	Arc::make_mut(&mut tile.payload).bounds = bounds;
	let needs_prepass = policy.needs_prepass;
	let shadows = policy.shadow_directionals_relevant > 0;

	let mut required = Vec::new();
	for batch in tile.payload.lods.iter().flat_map(|batches| batches.iter()) {
		let mask = match batch.key.material_kind {
			VisualMaterialKind::Stick if needs_prepass => READY_MAIN | READY_PREPASS,
			VisualMaterialKind::ShadowProxy => {
				if shadows {
					READY_SHADOW
				} else {
					0
				}
			}
			_ => READY_MAIN,
		};
		required.push(mask | READY_BUFFER);
	}
	tile.signal = Arc::new(TileSignal::new(required));
	let bounds = tile.payload.bounds;
	let aabb = Aabb {
		center: (bounds.min + bounds.max) * 0.5,
		half_extents: (bounds.max - bounds.min) * 0.5,
	};
	let mut ordinal = 0;
	for (level, batches) in LEVELS.into_iter().zip(&tile.payload.lods) {
		for batch in batches.iter() {
			let Some(mesh) = kits.mesh(batch.key.kit) else { return false };
			let Some(bytes) = PackedBatchInstance::encode(&batch.instances) else { return false };
			let shadow_only = batch.key.material_kind == VisualMaterialKind::ShadowProxy;
			let instance = PackedBatchInstance {
				cell: entity,
				mesh: mesh.id(),
				payload: Arc::clone(&tile.payload),
				signal: Arc::clone(&tile.signal),
				extracted_phase: PREPARING,
				ordinal,
				level,
				bytes,
				#[cfg(feature = "packed-diagnostics")]
				diagnostic_main_draws: Arc::new(std::sync::atomic::AtomicU64::new(0)),
				instances: batch.instances.len() as u32,
				shadow_only,
			};
			let mut proxy = world.spawn((
				PackedGroveProxy,
				Mesh3d(mesh),
				instance,
				NoAutomaticBatching,
				// Native Changed<Mesh3d> bounds must not replace the instance union.
				NoAutoAabb,
				Transform::IDENTITY,
				Visibility::Inherited,
				aabb,
				ChildOf(entity),
			));
			let proxy_id = proxy.id();
			if shadow_only {
				proxy.insert(RenderLayers::layer(SHADOW_PROXY_RENDER_LAYER));
			} else {
				proxy.insert(NotShadowCaster);
			}
			match batch.key.material_kind {
				VisualMaterialKind::Leaf => {
					let material = world
						.resource_mut::<Assets<PackedLeafMaterial>>()
						.add(PackedLeafMaterial { base_color: batch.base_color });
					world.entity_mut(proxy_id).insert(MeshMaterial3d(material));
				}
				VisualMaterialKind::Stick => {
					let material = world
						.resource_mut::<Assets<PackedStickMaterial>>()
						.add(PackedStickMaterial { base_color: batch.base_color });
					world.entity_mut(proxy_id).insert(MeshMaterial3d(material));
				}
				VisualMaterialKind::ShadowProxy => {
					let material = world
						.resource_mut::<Assets<PackedShadowMaterial>>()
						.add(PackedShadowMaterial { base_color: batch.base_color });
					world.entity_mut(proxy_id).insert(MeshMaterial3d(material));
				}
				VisualMaterialKind::Frond => return false,
			}
			tile.proxies.push(proxy_id);
			ordinal += 1;
		}
	}
	true
}

/// Initial admission supports one active scene camera on the ordinary grove
/// layer and directional shadow views. Other camera/light configurations retain
/// ordinary rendering until their distinct pipeline readiness can be proven.
#[cfg(test)]
pub(crate) fn supported_view_configuration(world: &mut World) -> bool {
	PackedViewPolicy::read(world).supported()
}

fn source_revision_changed(current: &ChicoGroveHost, original: &ChicoGroveHost) -> bool {
	current.layer != original.layer
		|| current.tile.family_name() != original.tile.family_name()
		|| current.tile.plant_list_identity() != original.tile.plant_list_identity()
		|| current.tile.structural_lod() != original.tile.structural_lod()
		|| current.tile.scene_bounds() != original.tile.scene_bounds()
}

pub fn sync_packed_tiles(world: &mut World) {
	let mode = *world.resource::<PackMode>();
	if mode.is_off() && world.resource::<PackedGroveCache>().tiles.is_empty() {
		return;
	}
	let _span =
		bevy::log::tracing::trace_span!(target: "maybraid.packed.profile", "packed_sync_tiles")
			.entered();
	let canopy = world.get_resource::<CanopyShadowMode>().copied().unwrap_or_default();
	let policy = PackedViewPolicy::read(world);
	let readiness_policy = policy.readiness_key();
	let enabled = !mode.is_off() && !canopy.canopy_casts() && policy.supported();
	let Some(mut cache) = world.remove_resource::<PackedGroveCache>() else { return };
	let Some(mut kits) = world.remove_resource::<PackedKitMeshes>() else {
		world.insert_resource(cache);
		return;
	};
	let stale: Vec<_> = cache
		.tiles
		.keys()
		.copied()
		.filter(|entity| world.get_entity(*entity).is_err())
		.collect();
	for entity in stale {
		cache.remove(entity);
	}
	cache.denied.retain(|entity, _| world.get::<ChicoGroveHost>(*entity).is_some());
	cache.unsupported.retain(|entity, _| world.get_entity(*entity).is_ok());
	let rejected = cache
		.tiles
		.iter()
		.filter(|(_, tile)| {
			tile.signal.phase.load(std::sync::atomic::Ordering::Acquire) != ACTIVE
				&& tile.payload.lods.iter().flat_map(|batches| batches.iter()).any(|batch| {
					matches!(kits.states.get(batch.key.kit), Some(KitState::Unsupported))
				})
		})
		.map(|(&entity, _)| entity)
		.collect::<Vec<_>>();
	for entity in rejected {
		if let Some(tile) = cache.remove(entity) {
			restore_ordinary(world, entity, &tile);
			cache.unsupported.insert(entity, (kits.epoch, AdmissionReason::UnsupportedKit));
		}
	}
	if !enabled {
		for (&entity, tile) in &cache.tiles {
			restore_ordinary(world, entity, tile);
		}
		let finished = cache
			.tiles
			.iter()
			.filter(|(entity, tile)| finish_restore(world, **entity, tile))
			.map(|(&entity, _)| entity)
			.collect::<Vec<_>>();
		for entity in finished {
			cache.remove(entity);
		}
		world.insert_resource(cache);
		world.insert_resource(kits);
		return;
	}
	let changed: Vec<_> = cache
		.tiles
		.iter()
		.filter(|(entity, tile)| {
			tile.last_kit_epoch != kits.epoch
				|| tile.payload.proxy_shadows != canopy.emit_proxies()
				|| tile.readiness_policy != readiness_policy
				|| world
					.get::<ChicoGroveHost>(**entity)
					.is_some_and(|host| source_revision_changed(host, &tile.original))
		})
		.map(|(&entity, _)| entity)
		.collect();
	for entity in changed {
		if let Some(tile) = cache.tiles.get(&entity) {
			restore_ordinary(world, entity, tile);
		}
	}
	let restored = cache
		.tiles
		.iter()
		.filter(|(entity, tile)| {
			tile.signal.phase.load(std::sync::atomic::Ordering::Acquire) == RETIRED
				|| (world.get::<RestoringOrdinary>(**entity).is_some()
					&& finish_restore(world, **entity, tile))
		})
		.map(|(&entity, _)| entity)
		.collect::<Vec<_>>();
	for entity in restored {
		cache.remove(entity);
	}
	// Snapshot once for the initial filter; recompute before each candidate
	// because earlier candidates consume capacity. No resident is reclaimable
	// until the normal handoff/retirement path actually removes it.
	let available = cache.available_gpu_bytes();
	let sources = {
		let mut query = world.query::<(Entity, Ref<ChicoGroveHost>)>();
		query
			.iter(world)
			.filter(|(entity, host)| {
				mode.accepts(&host.tile)
					&& !cache.tiles.contains_key(entity)
					&& (host.is_changed()
						|| cache.unsupported.get(entity).map(|(epoch, _)| *epoch)
							!= Some(kits.epoch))
			})
			.filter_map(|(entity, host)| {
				let key = AdmissionKey::from_host(
					&host,
					host.last_changed().get(),
					canopy.emit_proxies(),
				);
				cache
					.should_build_payload(entity, &key, available)
					.then(|| (entity, (*host).clone(), key))
			})
			.collect::<Vec<_>>()
	};
	for (entity, host, key) in sources {
		if !cache.should_build_payload(entity, &key, cache.available_gpu_bytes()) {
			continue;
		}
		// A stale identity must not label a newly unsupported revision as a
		// budget denial. Only the small key/size survives a fresh rejection.
		cache.denied.remove(&entity);
		cache.unsupported.remove(&entity);
		cache.revision = cache.revision.wrapping_add(1);
		let payload = match TilePayload::from_host(&host, cache.revision, canopy.emit_proxies()) {
			Ok(payload) => payload,
			Err(reason) => {
				cache.unsupported.insert(entity, (kits.epoch, reason));
				continue;
			}
		};
		if payload
			.lods
			.iter()
			.flat_map(|batches| batches.iter())
			.any(|batch| batch.key.material_kind == VisualMaterialKind::Frond)
		{
			cache
				.unsupported
				.insert(entity, (kits.epoch, AdmissionReason::UnsupportedGeometry));
			continue;
		}
		if !cache.admit(entity, key, payload.gpu_payload_bytes) {
			continue;
		}
		for batch in payload.lods.iter().flat_map(|batches| batches.iter()) {
			kits.request(world.resource::<AssetServer>(), batch.key.kit);
		}
		cache.builds += 1;
		cache.tiles.insert(
			entity,
			ResidentTile {
				readiness_policy: readiness_policy.clone(),
				original: host,
				payload: Arc::new(payload),
				signal: Arc::new(TileSignal::new([])),
				proxies: Vec::new(),
				last_kit_epoch: kits.epoch,
			},
		);
	}
	for (&entity, tile) in &mut cache.tiles {
		if world.get::<RestoringOrdinary>(entity).is_some() {
			continue;
		}
		if tile.signal.phase.load(std::sync::atomic::Ordering::Acquire) == ACTIVE {
			let level = ordinary_level(world, &tile.payload);
			if world.get::<LodSceneLevel>(entity).copied() != Some(level) {
				world.entity_mut(entity).insert(level);
			}
		}
		if tile.proxies.is_empty() {
			let all_ready = tile
				.payload
				.lods
				.iter()
				.flat_map(|batches| batches.iter())
				.all(|batch| kits.ready_bounds(batch.key.kit).is_some());
			if all_ready && !spawn_proxies(world, entity, tile, &kits, &policy) {
				for proxy in tile.proxies.drain(..) {
					let _ = world.despawn(proxy);
				}
				tile.signal.retire();
			}
		} else if tile.signal.phase.load(std::sync::atomic::Ordering::Acquire) == PREPARING
			&& tile.signal.is_ready()
		{
			activate_tile(world, entity, tile);
		}
	}
	world.insert_resource(cache);
	world.insert_resource(kits);
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::shape::Aabb3d;
	use vegetation_groves::{GroveExtent, Orchard};
	fn fixture(world: &mut World) -> (Entity, ResidentTile) {
		let orchard = Orchard {
			plants: Arc::from([]),
			structural_center: Vec3::ZERO,
			footprint_radius: 1.0,
			extent: GroveExtent::new(Vec3::ZERO, Vec3::splat(10.0)),
		};
		let host = ChicoGroveHost::new(
			crate::ForestGroveTile::Orchard(orchard),
			crate::ForestLayer::UpperCanopy,
		);
		let cell = world.spawn((host.clone(), PackedGroveCell, LodSceneLevel::High)).id();
		let proxy = world.spawn(PackedGroveProxy).id();
		let signal = Arc::new(TileSignal::new([]));
		assert!(signal.activate());
		let payload = TilePayload {
			#[cfg(feature = "packed-diagnostics")]
			diagnostic_family: "Orchard",
			revision: 1,
			proxy_shadows: false,
			bounds: Aabb3d { min: Vec3::splat(-1.0).into(), max: Vec3::ONE.into() },
			center: Vec3::ZERO,
			radius: 1.0,
			structural_lod: vegetation_components::StructuralLod::new(Vec3::ZERO, 1.0),
			plant_count: 0,
			lods: LEVELS.map(|_| Arc::from([])),
			cpu_payload_bytes: 0,
			gpu_payload_bytes: 0,
		};
		(
			cell,
			ResidentTile {
				readiness_policy: Default::default(),
				original: host,
				payload: Arc::new(payload),
				signal,
				proxies: vec![proxy],
				last_kit_epoch: 0,
			},
		)
	}
	#[test]
	fn reverse_handoff_keeps_packed_until_ordinary_streaming_completes() {
		let mut world = World::new();
		let (cell, tile) = fixture(&mut world);
		restore_ordinary(&mut world, cell, &tile);
		assert_eq!(tile.signal.phase.load(std::sync::atomic::Ordering::Acquire), ACTIVE);
		assert!(world.get_entity(tile.proxies[0]).is_ok());
		assert!(!finish_restore(&mut world, cell, &tile));
		let bag = world.spawn((LodLevelRoots, ChildOf(cell), Visibility::Hidden)).id();
		let root = world.spawn((ChildOf(bag), lod::LodLevelRootPending)).id();
		assert!(!finish_restore(&mut world, cell, &tile));
		world
			.entity_mut(root)
			.remove::<lod::LodLevelRootPending>()
			.insert(lod::LodLevelRootStreamed);
		assert!(finish_restore(&mut world, cell, &tile));
		assert_eq!(tile.signal.phase.load(std::sync::atomic::Ordering::Acquire), RETIRED);
		assert!(world.get_entity(tile.proxies[0]).is_err());
		assert_eq!(world.get::<Visibility>(bag), Some(&Visibility::Inherited));
		assert!(world.get::<PackedGroveCell>(cell).is_none());
	}
	#[test]
	fn preparing_fallback_leaves_ordinary_source_untouched() {
		let mut world = World::new();
		let (cell, mut tile) = fixture(&mut world);
		tile.signal = Arc::new(TileSignal::new([READY_MAIN | READY_BUFFER]));
		let ordinary = world.spawn((LodLevelRoots, ChildOf(cell))).id();
		restore_ordinary(&mut world, cell, &tile);
		assert!(world.get::<ChicoGroveHost>(cell).is_some());
		assert!(world.get_entity(ordinary).is_ok());
		assert!(world.get::<RestoringOrdinary>(cell).is_none());
		assert!(!tile.signal.activate());
	}
	#[test]
	fn two_scene_cameras_fall_back_but_disjoint_ui_camera_does_not() {
		let mut world = World::new();
		world.spawn((
			Camera3d::default(),
			Camera::default(),
			Transform::from_translation(Vec3::Z * 10.0),
		));
		assert!(supported_view_configuration(&mut world));
		let opposite = world
			.spawn((
				Camera3d::default(),
				Camera::default(),
				Transform::from_translation(Vec3::NEG_Z * 10.0),
			))
			.id();
		assert!(!supported_view_configuration(&mut world));
		world.entity_mut(opposite).insert(RenderLayers::layer(29));
		assert!(supported_view_configuration(&mut world));
	}
	#[test]
	fn changed_source_count_and_unsupported_adapter_restore_after_new_bag_streams() {
		for replacement in [crate::ForestGroveKind::HighBush, crate::ForestGroveKind::CommonTufts] {
			let mut world = World::new();
			let (cell, tile) = fixture(&mut world);
			let source = replacement.grow_tile(
				GroveExtent::new(Vec3::ZERO, Vec3::splat(40.0)),
				&vegetation_groves::FlatTerrainSample::default(),
			);
			world
				.entity_mut(cell)
				.insert(ChicoGroveHost::new(source, crate::ForestLayer::Understory));
			restore_ordinary(&mut world, cell, &tile);
			let bag = world.spawn((LodLevelRoots, ChildOf(cell), Visibility::Hidden)).id();
			world.spawn((ChildOf(bag), lod::LodLevelRootStreamed));
			// Fully streamed replacements do not depend on the old packed count/type.
			assert!(finish_restore(&mut world, cell, &tile));
			assert!(world.get::<PackedGroveCell>(cell).is_none());
		}
	}
	#[test]
	fn retained_plant_arc_metadata_revisions_invalidate_old_selection_and_bounds() {
		let mut world = World::new();
		let (_, tile) = fixture(&mut world);
		let original = &tile.original;
		let mut replacement = original.clone();
		assert!(!source_revision_changed(&replacement, original));
		if let crate::ForestGroveTile::Orchard(grove) = &mut replacement.tile {
			grove.structural_center += Vec3::X * 50.0;
		}
		assert_eq!(replacement.tile.plant_list_identity(), original.tile.plant_list_identity());
		assert!(source_revision_changed(&replacement, original));
		let mut replacement = original.clone();
		if let crate::ForestGroveTile::Orchard(grove) = &mut replacement.tile {
			grove.footprint_radius *= 2.0;
		}
		assert_eq!(replacement.tile.plant_list_identity(), original.tile.plant_list_identity());
		assert!(source_revision_changed(&replacement, original));
	}
	#[test]
	fn denied_payload_is_not_reassembled_for_change_flags_or_noop_epochs() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PackMode::Compatible);
		world.insert_resource(CanopyShadowMode::Off);
		world.init_resource::<PackedKitMeshes>();
		world.spawn((Camera3d::default(), Camera::default(), Transform::IDENTITY));
		let host = ChicoGroveHost::new(
			crate::ForestGroveKind::HighBush.grow_tile(
				GroveExtent::new(Vec3::ZERO, Vec3::splat(40.0)),
				&vegetation_groves::FlatTerrainSample::default(),
			),
			crate::ForestLayer::UpperCanopy,
		);
		let payload = TilePayload::from_host(&host, 1, false)
			.map_err(|reason| anyhow::anyhow!("fixture rejected {reason:?}"))?;
		let cell = world.spawn(host).id();
		let mut query = world.query::<(Entity, Ref<ChicoGroveHost>)>();
		let key = query
			.iter(&world)
			.find(|(entity, _)| *entity == cell)
			.map(|(_, host)| AdmissionKey::from_host(&host, host.last_changed().get(), false))
			.ok_or_else(|| anyhow::anyhow!("fixture host absent"))?;
		let mut cache = PackedGroveCache::with_budget(payload.gpu_payload_bytes.saturating_sub(1));
		assert!(!cache.admit(cell, key, payload.gpu_payload_bytes));
		world.insert_resource(cache);
		// The host remains marked changed, and an unrelated epoch changes.
		// The old filter rebuilt in either case before rejecting at admit().
		for _ in 0..3 {
			world.resource_mut::<PackedGroveCache>().epoch += 1;
			sync_packed_tiles(&mut world);
			assert_eq!(world.resource::<PackedGroveCache>().revision, 0);
			assert_eq!(world.resource::<PackedGroveCache>().budget_denials, 1);
		}
		// Metadata records do not outlive a removed source component.
		world.entity_mut(cell).remove::<ChicoGroveHost>();
		sync_packed_tiles(&mut world);
		assert!(!world.resource::<PackedGroveCache>().denied.contains_key(&cell));
		Ok(())
	}

	fn admission_app(payload: &TilePayload) -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()));
		app.init_asset::<bevy::gltf::Gltf>();
		let world = app.world_mut();
		world.insert_resource(PackMode::Compatible);
		world.insert_resource(CanopyShadowMode::Off);
		world.spawn((Camera3d::default(), Camera::default(), Transform::IDENTITY));
		let mut kits = PackedKitMeshes::default();
		// Stop before proxy publication: this test exercises CPU admission,
		// without requiring renderer/material resources or a successful asset load.
		for batch in payload.lods.iter().flat_map(|batches| batches.iter()) {
			kits.states.insert(batch.key.kit, KitState::Loading(Handle::default()));
		}
		world.insert_resource(kits);
		app
	}

	#[test]
	fn smaller_replacement_passes_real_sync_admission_in_the_same_epoch() -> anyhow::Result<()> {
		let original = ChicoGroveHost::new(
			crate::ForestGroveKind::HighBush.grow_tile(
				GroveExtent::new(Vec3::ZERO, Vec3::splat(40.0)),
				&vegetation_groves::FlatTerrainSample::default(),
			),
			crate::ForestLayer::UpperCanopy,
		);
		let mut replacement = original.clone();
		replacement.layer = crate::ForestLayer::Tufts;
		let large = TilePayload::from_host(&original, 1, false)
			.map_err(|reason| anyhow::anyhow!("original rejected {reason:?}"))?;
		let small = TilePayload::from_host(&replacement, 2, false)
			.map_err(|reason| anyhow::anyhow!("replacement rejected {reason:?}"))?;
		assert!(small.gpu_payload_bytes > 0 && small.gpu_payload_bytes < large.gpu_payload_bytes);
		let mut app = admission_app(&small);
		let world = app.world_mut();
		let cell = world.spawn(original).id();
		let mut query = world.query::<(Entity, Ref<ChicoGroveHost>)>();
		let key = query
			.iter(world)
			.find(|(entity, _)| *entity == cell)
			.map(|(_, host)| AdmissionKey::from_host(&host, host.last_changed().get(), false))
			.ok_or_else(|| anyhow::anyhow!("original host absent"))?;
		let mut cache = PackedGroveCache::with_budget(small.gpu_payload_bytes);
		assert!(!cache.admit(cell, key, large.gpu_payload_bytes));
		world.insert_resource(cache);
		world.clear_trackers();
		world.entity_mut(cell).insert(replacement);
		sync_packed_tiles(world);
		let cache = world.resource::<PackedGroveCache>();
		assert_eq!(cache.epoch, 0);
		assert_eq!(cache.builds, 1);
		assert_eq!(cache.revision, 1);
		assert_eq!(cache.tiles[&cell].payload.gpu_payload_bytes, small.gpu_payload_bytes);
		assert!(!cache.denied.contains_key(&cell));
		sync_packed_tiles(world);
		assert_eq!(world.resource::<PackedGroveCache>().revision, 1);
		Ok(())
	}

	#[test]
	fn sufficient_capacity_retries_real_payload_assembly_once() -> anyhow::Result<()> {
		let host = ChicoGroveHost::new(
			crate::ForestGroveKind::HighBush.grow_tile(
				GroveExtent::new(Vec3::ZERO, Vec3::splat(40.0)),
				&vegetation_groves::FlatTerrainSample::default(),
			),
			crate::ForestLayer::UpperCanopy,
		);
		let payload = TilePayload::from_host(&host, 1, false)
			.map_err(|reason| anyhow::anyhow!("fixture rejected {reason:?}"))?;
		let mut app = admission_app(&payload);
		let world = app.world_mut();
		let cell = world.spawn(host).id();
		let mut query = world.query::<(Entity, Ref<ChicoGroveHost>)>();
		let key = query
			.iter(world)
			.find(|(entity, _)| *entity == cell)
			.map(|(_, host)| AdmissionKey::from_host(&host, host.last_changed().get(), false))
			.ok_or_else(|| anyhow::anyhow!("fixture host absent"))?;
		let mut cache = PackedGroveCache::with_budget(payload.gpu_payload_bytes.saturating_sub(1));
		assert!(!cache.admit(cell, key, payload.gpu_payload_bytes));
		world.insert_resource(cache);
		world.clear_trackers();
		sync_packed_tiles(world);
		assert_eq!(world.resource::<PackedGroveCache>().revision, 0);
		world.resource_mut::<PackedGroveCache>().budget_bytes = payload.gpu_payload_bytes;
		sync_packed_tiles(world);
		assert_eq!(world.resource::<PackedGroveCache>().revision, 1);
		assert_eq!(world.resource::<PackedGroveCache>().builds, 1);
		sync_packed_tiles(world);
		assert_eq!(world.resource::<PackedGroveCache>().revision, 1);
		Ok(())
	}

	#[test]
	fn preparing_and_active_tiles_stay_charged_until_actual_removal() {
		for active in [false, true] {
			let mut world = World::new();
			let (cell, mut tile) = fixture(&mut world);
			if !active {
				tile.signal = Arc::new(TileSignal::new([READY_MAIN | READY_BUFFER]));
			}
			let payload = TilePayload {
				#[cfg(feature = "packed-diagnostics")]
				diagnostic_family: "Orchard",
				revision: 1,
				proxy_shadows: false,
				bounds: Aabb3d { min: Vec3::NEG_ONE.into(), max: Vec3::ONE.into() },
				center: Vec3::ZERO,
				radius: 1.0,
				structural_lod: vegetation_components::StructuralLod::new(Vec3::ZERO, 1.0),
				plant_count: 0,
				lods: LEVELS.map(|_| Arc::from([])),
				cpu_payload_bytes: 0,
				gpu_payload_bytes: 96,
			};
			// No selection/phase makes this charge implicitly reclaimable.
			tile.payload = Arc::new(payload);
			let waiting = world.spawn_empty().id();
			let key = AdmissionKey::from_host(&tile.original, 0, false);
			let mut cache = PackedGroveCache::with_budget(128);
			cache.tiles.insert(cell, tile);
			assert_eq!(cache.available_gpu_bytes(), 32);
			assert!(!cache.admit(waiting, key.clone(), 64));
			assert!(!cache.should_build_payload(waiting, &key, cache.available_gpu_bytes()));
			let removed = cache.remove(cell);
			assert!(removed.is_some());
			assert_eq!(cache.available_gpu_bytes(), 128);
			assert_eq!(cache.epoch, 1);
			assert!(cache.should_build_payload(waiting, &key, cache.available_gpu_bytes()));
			assert!(cache.admit(waiting, key, 64));
		}
	}
	#[test]
	fn unavailable_ultra_low_kit_keeps_entire_ordinary_tile() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PackMode::Compatible);
		world.insert_resource(CanopyShadowMode::Off);
		world.spawn((Camera3d::default(), Camera::default(), Transform::IDENTITY));
		let host = ChicoGroveHost::new(
			crate::ForestGroveKind::HighBush.grow_tile(
				GroveExtent::new(Vec3::ZERO, Vec3::splat(40.0)),
				&vegetation_groves::FlatTerrainSample::default(),
			),
			crate::ForestLayer::UpperCanopy,
		);
		let payload = TilePayload::from_host(&host, 1, false)
			.map_err(|reason| anyhow::anyhow!("fixture rejected {reason:?}"))?;
		let missing = payload.lods[3]
			.first()
			.map(|batch| batch.key.kit)
			.ok_or_else(|| anyhow::anyhow!("fixture has no UltraLow kit"))?;
		let mut kits = PackedKitMeshes::default();
		for batch in payload.lods.iter().flat_map(|batches| batches.iter()) {
			kits.states.insert(batch.key.kit, KitState::Ready(Handle::default()));
		}
		kits.states.insert(missing, KitState::Loading(Handle::default()));
		let cell = world.spawn((host.clone(), LodSceneHost, LodSceneLevel::High)).id();
		let bag = world.spawn((ChildOf(cell), LodLevelRoots)).id();
		let readiness_policy = PackedViewPolicy::read(&mut world).readiness_key();
		let mut cache = PackedGroveCache::default();
		cache.tiles.insert(
			cell,
			ResidentTile {
				readiness_policy,
				original: host,
				payload: Arc::new(payload),
				signal: Arc::new(TileSignal::new([])),
				proxies: Vec::new(),
				last_kit_epoch: 0,
			},
		);
		world.insert_resource(cache);
		world.insert_resource(kits);
		world.clear_trackers();
		sync_packed_tiles(&mut world);
		assert!(world.get::<ChicoGroveHost>(cell).is_some());
		assert!(world.get_entity(bag).is_ok());
		assert!(world.get::<PackedGroveCell>(cell).is_none());
		assert!(world.resource::<PackedGroveCache>().tiles[&cell].proxies.is_empty());
		let capacity_before = world.resource::<PackedGroveCache>().available_gpu_bytes();
		let epoch_before = world.resource::<PackedGroveCache>().epoch;
		world
			.resource_mut::<PackedKitMeshes>()
			.states
			.insert(missing, KitState::Unsupported);
		sync_packed_tiles(&mut world);
		assert!(world.get::<ChicoGroveHost>(cell).is_some());
		assert!(world.get_entity(bag).is_ok());
		assert!(!world.resource::<PackedGroveCache>().tiles.contains_key(&cell));
		assert!(world.resource::<PackedGroveCache>().available_gpu_bytes() > capacity_before);
		assert_eq!(world.resource::<PackedGroveCache>().epoch, epoch_before + 1);
		assert_eq!(
			world
				.resource::<PackedGroveCache>()
				.unsupported
				.get(&cell)
				.map(|(_, reason)| *reason),
			Some(AdmissionReason::UnsupportedKit)
		);
		Ok(())
	}
	#[test]
	fn newly_relevant_second_shadow_light_restores_ordinary_before_retiring_packed() {
		let mut world = World::new();
		world.insert_resource(PackMode::Compatible);
		world.insert_resource(CanopyShadowMode::Off);
		world.init_resource::<PackedKitMeshes>();
		world.spawn((Camera3d::default(), Camera::default(), Transform::IDENTITY));
		world.spawn(DirectionalLight { shadow_maps_enabled: true, ..default() });
		let preview = world
			.spawn((
				DirectionalLight { shadow_maps_enabled: true, ..default() },
				RenderLayers::layer(1).with(30),
			))
			.id();
		let (cell, mut tile) = fixture(&mut world);
		tile.readiness_policy = PackedViewPolicy::read(&mut world).readiness_key();
		world.entity_mut(cell).remove::<ChicoGroveHost>();
		let proxy = tile.proxies[0];
		let signal = Arc::clone(&tile.signal);
		let mut cache = PackedGroveCache::default();
		cache.tiles.insert(cell, tile);
		world.insert_resource(cache);
		sync_packed_tiles(&mut world);
		assert!(world.get::<RestoringOrdinary>(cell).is_none());
		assert_eq!(signal.phase.load(std::sync::atomic::Ordering::Acquire), ACTIVE);
		world.entity_mut(preview).insert(RenderLayers::default());
		sync_packed_tiles(&mut world);
		assert!(world.get::<RestoringOrdinary>(cell).is_some());
		assert!(world.get::<ChicoGroveHost>(cell).is_some());
		assert!(world.get_entity(proxy).is_ok());
		assert_eq!(signal.phase.load(std::sync::atomic::Ordering::Acquire), ACTIVE);
	}
	#[test]
	fn supported_pass_change_restores_active_or_retires_preparing_readiness() {
		for preparing in [false, true] {
			for add_shadow in [false, true] {
				let mut world = World::new();
				world.insert_resource(PackMode::Compatible);
				world.insert_resource(CanopyShadowMode::Off);
				world.init_resource::<PackedKitMeshes>();
				let camera =
					world.spawn((Camera3d::default(), Camera::default(), Transform::IDENTITY)).id();
				let (cell, mut tile) = fixture(&mut world);
				tile.readiness_policy = PackedViewPolicy::read(&mut world).readiness_key();
				if preparing {
					tile.signal = Arc::new(TileSignal::new([READY_BUFFER | READY_MAIN]));
				} else {
					world.entity_mut(cell).remove::<ChicoGroveHost>();
				}
				let proxy = tile.proxies[0];
				let signal = Arc::clone(&tile.signal);
				let mut cache = PackedGroveCache::default();
				cache.tiles.insert(cell, tile);
				world.insert_resource(cache);
				if add_shadow {
					world.spawn(DirectionalLight { shadow_maps_enabled: true, ..default() });
				} else {
					world.entity_mut(camera).insert(bevy::core_pipeline::prepass::DepthPrepass);
				}
				assert!(PackedViewPolicy::read(&mut world).supported());
				sync_packed_tiles(&mut world);
				assert!(world.get::<ChicoGroveHost>(cell).is_some());
				if preparing {
					assert_eq!(signal.phase.load(std::sync::atomic::Ordering::Acquire), RETIRED);
					assert!(world.get_entity(proxy).is_err());
				} else {
					assert!(world.get::<RestoringOrdinary>(cell).is_some());
					assert_eq!(signal.phase.load(std::sync::atomic::Ordering::Acquire), ACTIVE);
					assert!(world.get_entity(proxy).is_ok());
				}
			}
		}
	}
}
