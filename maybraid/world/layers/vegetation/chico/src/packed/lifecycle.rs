//! One-time ordinary-to-packed handoff and revision/retirement ownership.

use crate::packed::cache::*;
use crate::packed::gpu::PackedBatchInstance;
use crate::packed::kit::{KitState, PackedKitMeshes};
use crate::packed::material::{PackedLeafMaterial, PackedShadowMaterial, PackedStickMaterial};
use crate::packed::mode::PackMode;
use crate::packed::select::select_band;
use crate::ChicoGroveHost;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass};
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
	let mut cameras = world.query_filtered::<(&GlobalTransform, &Camera), With<Camera3d>>();
	cameras
		.iter(world)
		.filter(|(_, camera)| camera.is_active)
		.map(|(xf, _)| select_band(payload.center, payload.radius, xf.translation()))
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
		let expected = world
			.get::<ChicoGroveHost>(entity)
			.and_then(|host| host.tile.as_orchard())
			.map_or(0, |orchard| orchard.plants.len());
		if !crate::stick_physics::ordinary_packed_physics_ready(world, entity, expected) {
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
) -> bool {
	let needs_prepass = {
		let mut cameras =
			world.query_filtered::<(Has<DepthPrepass>, Has<NormalPrepass>), With<Camera3d>>();
		cameras.iter(world).any(|(depth, normal)| depth || normal)
	};
	let shadows = world
		.query::<&DirectionalLight>()
		.iter(world)
		.any(|light| light.shadow_maps_enabled);
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
fn supported_view_configuration(world: &mut World) -> bool {
	let mut cameras = world.query_filtered::<(&Camera, Option<&RenderLayers>), With<Camera3d>>();
	let ordinary = RenderLayers::default();
	let views = cameras
		.iter(world)
		.filter(|(camera, layers)| {
			camera.is_active && layers.is_none_or(|layers| layers.intersects(&ordinary))
		})
		.count();
	if views != 1 {
		return false;
	}
	let directional = world
		.query::<&DirectionalLight>()
		.iter(world)
		.filter(|light| light.shadow_maps_enabled)
		.count();
	directional <= 1
		&& !world.query::<&PointLight>().iter(world).any(|light| light.shadow_maps_enabled)
		&& !world.query::<&SpotLight>().iter(world).any(|light| light.shadow_maps_enabled)
}

pub fn sync_packed_tiles(world: &mut World) {
	let mode = world.resource::<PackMode>().packs_orchard();
	if !mode && world.resource::<PackedGroveCache>().tiles.is_empty() {
		return;
	}
	let _span =
		bevy::log::tracing::trace_span!(target: "maybraid.packed.profile", "packed_sync_tiles")
			.entered();
	let canopy = world.get_resource::<CanopyShadowMode>().copied().unwrap_or_default();
	let enabled = mode && !canopy.canopy_casts() && supported_view_configuration(world);
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
	cache.denied.retain(|entity, _| world.get_entity(*entity).is_ok());
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
		if let Some(tile) = cache.tiles.remove(&entity) {
			restore_ordinary(world, entity, &tile);
			cache.unsupported.insert(entity, kits.epoch);
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
				|| world.get::<ChicoGroveHost>(**entity).is_some_and(|host| {
					host.layer != tile.original.layer
						|| match (host.tile.as_orchard(), tile.original.tile.as_orchard()) {
							(Some(a), Some(b)) => !Arc::ptr_eq(&a.plants, &b.plants),
							_ => true,
						}
				})
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
	let sources = {
		let mut query = world.query::<(Entity, Ref<ChicoGroveHost>)>();
		query
			.iter(world)
			.filter(|(entity, host)| {
				host.tile.as_orchard().is_some()
					&& !cache.tiles.contains_key(entity)
					&& (host.is_changed()
						|| (cache.denied.get(entity) != Some(&cache.epoch)
							&& cache.unsupported.get(entity) != Some(&kits.epoch)))
			})
			.map(|(entity, host)| (entity, (*host).clone()))
			.collect::<Vec<_>>()
	};
	for (entity, host) in sources {
		cache.revision = cache.revision.wrapping_add(1);
		let Some(payload) = TilePayload::from_host(&host, cache.revision, canopy.emit_proxies())
		else {
			cache.unsupported.insert(entity, kits.epoch);
			continue;
		};
		if payload
			.lods
			.iter()
			.flat_map(|batches| batches.iter())
			.any(|batch| batch.key.material_kind == VisualMaterialKind::Frond)
		{
			cache.unsupported.insert(entity, kits.epoch);
			continue;
		}
		if !cache.admit(entity, payload.gpu_payload_bytes) {
			continue;
		}
		for batch in payload.lods.iter().flat_map(|batches| batches.iter()) {
			kits.request(world.resource::<AssetServer>(), batch.key.kit);
		}
		cache.builds += 1;
		cache.tiles.insert(
			entity,
			ResidentTile {
				layer: host.layer,
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
			let all_ready =
				tile.payload.lods.iter().flat_map(|batches| batches.iter()).all(|batch| {
					matches!(kits.states.get(batch.key.kit), Some(KitState::Ready(_)))
				});
			if all_ready && !spawn_proxies(world, entity, tile, &kits) {
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
			revision: 1,
			proxy_shadows: false,
			bounds: Aabb3d { min: Vec3::splat(-1.0).into(), max: Vec3::ONE.into() },
			center: Vec3::ZERO,
			radius: 1.0,
			lods: LEVELS.map(|_| Arc::from([])),
			cpu_payload_bytes: 0,
			gpu_payload_bytes: 0,
		};
		(
			cell,
			ResidentTile {
				original: host,
				payload: Arc::new(payload),
				signal,
				proxies: vec![proxy],
				layer: crate::ForestLayer::UpperCanopy,
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
}
