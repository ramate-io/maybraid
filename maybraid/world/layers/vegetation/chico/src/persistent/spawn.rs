//! Loading creates only the selected band; the root's ordinary LOD/HCSG lifecycle owns it.
use crate::persistent::{
	gpu::PackedBatchInstance,
	kit::{KitState, PackedKitMeshes},
	material::*,
	payload::{MaterialKind, PackedBatch, SelectedBand},
	FixtureMode,
};
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{NoAutoAabb, RenderLayers};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::batching::NoAutomaticBatching;
use bevy::shape::Aabb3d;
use std::{collections::HashMap, sync::Arc};
use vegetation_components::SHADOW_PROXY_RENDER_LAYER;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixtureState {
	Pending,
	/// CPU construction completed; the harness separately verifies GPU/pass readiness.
	Ready,
	Failed,
}

#[derive(Clone, Component)]
pub struct FixtureStatus {
	pub mode: FixtureMode,
	pub source_plants: usize,
	pub batches: usize,
	pub instances: usize,
	pub bounds: Aabb3d,
	pub state: FixtureState,
	pub reason: &'static str,
	pub source_digest: u64,
	pub selected_proxies: usize,
	pub resident_bytes: u64,
}
impl FixtureStatus {
	pub fn failed(mode: FixtureMode, bounds: Aabb3d, reason: &'static str) -> Self {
		Self {
			mode,
			bounds,
			reason,
			source_plants: 0,
			batches: 0,
			instances: 0,
			state: FixtureState::Failed,
			source_digest: 0,
			selected_proxies: 0,
			resident_bytes: 0,
		}
	}
	pub(crate) fn pending(mode: FixtureMode, bounds: Aabb3d, band: &SelectedBand) -> Self {
		Self {
			mode,
			bounds,
			reason: "waiting for source kit meshes",
			source_plants: band.source_plants,
			batches: band.batches.len(),
			instances: band.batches.iter().map(|batch| batch.instances.len()).sum(),
			state: FixtureState::Pending,
			source_digest: band.source_digest,
			selected_proxies: 0,
			resident_bytes: 0,
		}
	}
}

#[derive(Clone, Component)]
pub struct PendingBand(pub SelectedBand);

#[derive(Resource, Default)]
pub struct SharedMaterials {
	leaves: HashMap<[u32; 4], Handle<PackedLeafMaterial>>,
	sticks: HashMap<[u32; 4], Handle<PackedStickMaterial>>,
	shadows: HashMap<[u32; 4], Handle<PackedShadowMaterial>>,
}
impl SharedMaterials {
	#[cfg(test)]
	pub(crate) fn len(&self) -> usize {
		self.leaves.len() + self.sticks.len() + self.shadows.len()
	}
	fn insert(&mut self, world: &mut World, entity: Entity, batch: &PackedBatch) {
		let color = batch.key.color;
		match batch.key.material_kind {
			MaterialKind::Leaf => {
				let material = self
					.leaves
					.entry(color)
					.or_insert_with(|| {
						world
							.resource_mut::<Assets<PackedLeafMaterial>>()
							.add(PackedLeafMaterial { base_color: batch.base_color })
					})
					.clone();
				world.entity_mut(entity).insert(MeshMaterial3d(material));
			}
			MaterialKind::Stick => {
				let material = self
					.sticks
					.entry(color)
					.or_insert_with(|| {
						world
							.resource_mut::<Assets<PackedStickMaterial>>()
							.add(PackedStickMaterial { base_color: batch.base_color })
					})
					.clone();
				world.entity_mut(entity).insert(MeshMaterial3d(material));
			}
			MaterialKind::ShadowProxy => {
				let material = self
					.shadows
					.entry(color)
					.or_insert_with(|| {
						world
							.resource_mut::<Assets<PackedShadowMaterial>>()
							.add(PackedShadowMaterial { base_color: batch.base_color })
					})
					.clone();
				world.entity_mut(entity).insert(MeshMaterial3d(material));
			}
		}
	}
}

pub fn load_selected_band(world: &mut World) {
	let mut roots = world.query::<&FixtureStatus>();
	if roots.iter(world).next().is_none() {
		world.resource_mut::<PackedKitMeshes>().states.clear();
		world.resource_mut::<PackedKitMeshes>().local_bounds.clear();
		*world.resource_mut::<SharedMaterials>() = SharedMaterials::default();
		return;
	}
	let pending: Vec<_> = {
		let mut query = world.query::<(Entity, &PendingBand, &FixtureStatus)>();
		query
			.iter(world)
			.map(|(entity, band, status)| (entity, band.0.clone(), status.mode))
			.collect()
	};
	if pending.is_empty() {
		return;
	}
	let mut kits = world.remove_resource::<PackedKitMeshes>().unwrap_or_default();
	let mut materials = world.remove_resource::<SharedMaterials>().unwrap_or_default();
	for (root, band, mode) in pending {
		for batch in band.batches.iter() {
			kits.request(world.resource::<AssetServer>(), batch.key.kit);
		}
		let failed = band
			.batches
			.iter()
			.any(|batch| matches!(kits.states.get(batch.key.kit), Some(KitState::Unsupported)));
		if failed {
			if let Some(mut status) = world.get_mut::<FixtureStatus>(root) {
				status.state = FixtureState::Failed;
				status.reason = "unsupported or failed source kit asset";
			}
			world.entity_mut(root).remove::<PendingBand>();
			continue;
		}
		let Some(bounds) = band.geometric_bounds(&kits) else {
			continue;
		};
		let view = FixtureViewContract::validate(world);
		if matches!(view, Ok(false)) {
			if let Some(mut status) = world.get_mut::<FixtureStatus>(root) {
				status.reason = "waiting for active scene camera";
			}
			continue;
		}
		if let Err(reason) = view {
			if let Some(mut status) = world.get_mut::<FixtureStatus>(root) {
				status.state = FixtureState::Failed;
				status.reason = reason;
			}
			world.entity_mut(root).remove::<PendingBand>();
			continue;
		}
		let encoded = band
			.batches
			.iter()
			.map(|batch| PackedBatchInstance::encode(&batch.instances))
			.collect::<Option<Vec<_>>>();
		let Some(encoded) = encoded else {
			if let Some(mut status) = world.get_mut::<FixtureStatus>(root) {
				status.state = FixtureState::Failed;
				status.reason = "invalid selected-band transform payload";
			}
			world.entity_mut(root).remove::<PendingBand>();
			continue;
		};
		let aabb = Aabb::from_min_max(bounds.min.into(), bounds.max.into());
		let mut resident_bytes = 0;
		if mode == FixtureMode::Persistent {
			for (batch, bytes) in band.batches.iter().zip(encoded) {
				let Some(mesh) = kits.mesh(batch.key.kit) else {
					continue;
				};
				resident_bytes += bytes.len() as u64;
				let entity = world
					.spawn((
						Mesh3d(mesh.clone()),
						PackedBatchInstance {
							mesh: mesh.id(),
							bytes: Arc::clone(&bytes),
							instances: batch.instances.len() as u32,
						},
						NoAutomaticBatching,
						NoAutoAabb,
						Transform::IDENTITY,
						Visibility::Inherited,
						aabb,
						ChildOf(root),
					))
					.id();
				if batch.key.material_kind == MaterialKind::ShadowProxy {
					world.entity_mut(entity).insert(RenderLayers::layer(SHADOW_PROXY_RENDER_LAYER));
				} else {
					world.entity_mut(entity).insert(NotShadowCaster);
				}
				materials.insert(world, entity, batch);
			}
		}
		if let Some(mut status) = world.get_mut::<FixtureStatus>(root) {
			status.bounds = bounds;
			status.state = FixtureState::Ready;
			status.reason = "";
			status.resident_bytes = resident_bytes;
			status.selected_proxies =
				if mode == FixtureMode::Persistent { band.batches.len() } else { 0 };
		}
		world.entity_mut(root).remove::<PendingBand>();
	}
	world.insert_resource(kits);
	world.insert_resource(materials);
}

/// A broken explicit fixture is a failed benchmark run in clean and diagnostic builds.
/// This is an error path, not a performance subscriber or per-frame metric.
pub fn fail_invalid_fixture(
	statuses: Query<&FixtureStatus>,
	mut exit: MessageWriter<AppExit>,
	mut reported: Local<bool>,
) {
	if *reported {
		return;
	}
	if let Some(status) = statuses.iter().find(|status| status.state == FixtureState::Failed) {
		*reported = true;
		eprintln!("vegetation instance fixture failed: {}", status.reason);
		exit.write(AppExit::error());
	}
}

/// One bounded startup validation shared by A/B; preview lights follow actual layers.
pub(crate) struct FixtureViewContract;
impl FixtureViewContract {
	pub(crate) fn validate(world: &mut World) -> Result<bool, &'static str> {
		use bevy::core_pipeline::prepass::DeferredPrepass;
		let cameras: Vec<_> = world
			.query_filtered::<(&Camera, Option<&RenderLayers>, Has<DeferredPrepass>), With<Camera3d>>(
			)
			.iter(world)
			.filter(|(camera, layers, _)| {
				camera.is_active && layers.unwrap_or_default().intersects(&RenderLayers::default())
			})
			.map(|(_, layers, deferred)| (layers.cloned().unwrap_or_default(), deferred))
			.take(2)
			.collect();
		if cameras.is_empty() {
			return Ok(false);
		}
		if cameras.len() != 1 {
			return Err("fixture requires exactly one active scene camera");
		}
		let (layers, deferred) = &cameras[0];
		if *deferred {
			return Err("fixture does not support deferred scene cameras");
		}
		if layers.intersects(&RenderLayers::layer(SHADOW_PROXY_RENDER_LAYER)) {
			return Err("fixture camera must exclude canopy shadow proxy layer");
		}
		let relevant = |light_layers: Option<&RenderLayers>| {
			light_layers.unwrap_or_default().intersects(layers)
		};
		let directionals = world
			.query::<(&DirectionalLight, Option<&RenderLayers>)>()
			.iter(world)
			.filter(|(light, layers)| light.shadow_maps_enabled && relevant(*layers))
			.take(2)
			.count();
		if directionals > 1 {
			return Err("fixture supports at most one relevant shadow directional");
		}
		if world
			.query::<(&PointLight, Option<&RenderLayers>)>()
			.iter(world)
			.any(|(light, layers)| light.shadow_maps_enabled && relevant(layers))
		{
			return Err("fixture does not support relevant point shadows");
		}
		if world
			.query::<(&SpotLight, Option<&RenderLayers>)>()
			.iter(world)
			.any(|(light, layers)| light.shadow_maps_enabled && relevant(layers))
		{
			return Err("fixture does not support relevant spot shadows");
		}
		Ok(true)
	}
}
