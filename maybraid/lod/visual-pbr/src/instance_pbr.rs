//! Instanced submit: spatial grove first, then `(mesh, material)`.
//!
//! Each grove owns its batch entities (parented under [`VisualLodRoot`]). A band
//! or present change only rebuilds that grove.
//!
//! Invariant: instance matrices are world (grove host is identity today). The
//! shader does `clip_from_world * instance * vertex` and must not read
//! `mesh[0]` via `get_world_from_local(0u)`. The batch [`Aabb`] is the grove
//! footprint with a ±1000 m Y slab so instance poses that sit above the
//! structural pancake stay in frustum. `NoAutoAabb` stops `Mesh3d` from
//! replacing that box with kit-local bounds at the origin. Opaque vegetation
//! queues [`Opaque3d`].

use std::mem::size_of;

use bevy::asset::{load_internal_asset, uuid_handle, AssetId};
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{add_visibility_class, NoAutoAabb, ViewVisibility, VisibilityClass};
use bevy::color::ColorToComponents;
use bevy::core_pipeline::core_3d::{Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey};
use bevy::ecs::system::lifetimeless::{Read, SRes};
use bevy::ecs::system::{SystemParam, SystemParamItem};
use bevy::mesh::{MeshVertexBufferLayoutRef, VertexBufferLayout};
use bevy::pbr::{
	MeshPipeline, MeshPipelineKey, MeshPipelineSystems, RenderMeshInstances, SetMeshBindGroup,
	SetMeshViewBindGroup, SetMeshViewBindingArrayBindGroup, ViewKeyCache,
};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::mesh::allocator::{MeshAllocator, MeshSlabs};
use bevy::render::mesh::{RenderMesh, RenderMeshBufferInfo};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_phase::{
	AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, InputUniformIndex, PhaseItem,
	RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewBinnedRenderPhases,
};
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::sync_world::{MainEntity, RenderEntity, SyncToRenderWorld};
use bevy::render::view::{ExtractedView, RetainedViewEntity};
use bevy::render::Extract;
use bevy::render::{ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::Shader;
use bytemuck::{Pod, Zeroable};
use lod::{
	LodHostBounds, LodSceneLevel, LodViewer, NamedVisualLevel, ProjectedBoundsPolicy,
	VisualLodPolicy, VisualLodRoot,
};
use material_ref::MaterialRef;
use scene_ref::{ScenePrototypeCache, SceneRef, SceneRefHandles};

use crate::{ForestGroveVisual, SelectedVisualBand};

const INSTANCE_SHADER_HANDLE: Handle<Shader> = uuid_handle!("6e2c1a0f-4b8d-4e91-9c3a-7f1d2b8e5a04");

/// Records the policy pick. The plugin's render systems submit the draws.
pub struct InstancePbrRenderer;

impl lod::VisualLodRenderer<ForestGroveVisual> for InstancePbrRenderer {
	fn queue(
		_scene: &ForestGroveVisual,
		selection: <ProjectedBoundsPolicy as lod::VisualLodPolicy<ForestGroveVisual>>::Selection,
		ctx: &mut lod::VisualLodRenderContext,
	) {
		ctx.insert(SelectedVisualBand(selection.clamp_to_packed()));
	}
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
	col0: [f32; 4],
	col1: [f32; 4],
	col2: [f32; 4],
	col3: [f32; 4],
	color: [f32; 4],
}

impl InstanceData {
	fn new(transform: Mat4, color: [f32; 4]) -> Self {
		let c = transform.to_cols_array_2d();
		Self { col0: c[0], col1: c[1], col2: c[2], col3: c[3], color }
	}
}

#[derive(Component, Deref, DerefMut, Clone)]
struct InstanceMaterialData(Vec<InstanceData>);

#[derive(Component)]
#[require(VisibilityClass)]
#[component(on_add = add_visibility_class::<InstancePbrBatch>)]
struct InstancePbrBatch;

/// Live visual/present counters. Distinguishes admission backlog from dead submit.
#[derive(Resource, Debug, Clone, Default)]
pub struct VisualHorizonStats {
	pub visual_roots: u32,
	pub semantic_high: u32,
	pub semantic_other: u32,
	pub packed_instances: u32,
	pub pending_groves: u32,
	pub batch_entities: u32,
	pub batches_visible: u32,
	pub items: u32,
	pub items_ultralow: u32,
	pub items_low: u32,
	pub items_medium: u32,
	pub prototype_cache: usize,
	pub desired_in_keep: u32,
	pub presented: u32,
	pub present_pending: u32,
	pub generate_pending: u32,
	pub sample_instance: [f32; 3],
	pub sample_aabb: [f32; 3],
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct BatchKey {
	mesh: AssetId<Mesh>,
	color: [u32; 4],
}

struct GroveSubmit {
	band: NamedVisualLevel,
	semantic_high: bool,
	pending: bool,
	items: Vec<(BatchKey, Handle<Mesh>, InstanceData)>,
	batches: HashMap<BatchKey, Entity>,
	debug_marker: Option<Entity>,
}

#[derive(Resource, Default)]
struct InstancePbrState {
	groves: HashMap<Entity, GroveSubmit>,
}

pub struct InstancePbrPlugin;

impl Plugin for InstancePbrPlugin {
	fn build(&self, app: &mut App) {
		load_internal_asset!(app, INSTANCE_SHADER_HANDLE, "instance.wgsl", Shader::from_wgsl);
		app.init_resource::<InstancePbrState>()
			.init_resource::<VisualHorizonStats>()
			.init_resource::<BandDebugAssets>()
			.add_systems(Update, (count_instance_visibility, sync_instance_pbr_batches).chain());
		let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
			return;
		};
		render_app
			.add_render_command::<Opaque3d, DrawInstancePbr>()
			.init_resource::<SpecializedMeshPipelines<InstancePbrPipeline>>()
			.add_systems(ExtractSchedule, extract_instance_data)
			.add_systems(RenderStartup, init_instance_pipeline.after(MeshPipelineSystems))
			.add_systems(
				Render,
				(
					queue_instance_pbr.in_set(RenderSystems::QueueMeshes),
					prepare_instance_buffers.in_set(RenderSystems::PrepareResources),
				),
			);
	}
}

/// Grove footprint XZ; Y is at least ±1000 m so instance poses above the
/// structural pancake stay inside the CPU frustum test.
fn batch_cull_aabb(footprint: Aabb) -> Aabb {
	let mut aabb = footprint;
	aabb.half_extents.y = aabb.half_extents.y.max(1000.0);
	aabb
}

pub fn selected_packed_band(
	visual: &ForestGroveVisual,
	bounds: &LodHostBounds,
	views: &Query<(&Camera, &GlobalTransform), With<LodViewer>>,
) -> NamedVisualLevel {
	views
		.iter()
		.filter_map(|(camera, transform)| lod::VisualLodView::from_camera(camera, transform))
		.map(|view| ProjectedBoundsPolicy::select(visual, &view, bounds).clamp_to_packed())
		.max()
		.unwrap_or(NamedVisualLevel::Low)
}

fn material_color(material: &MaterialRef) -> [f32; 4] {
	material
		.palette
		.first()
		.map(|c| c.to_linear().to_f32_array())
		.unwrap_or([0.25, 0.55, 0.28, 1.0])
}

fn color_key(color: [f32; 4]) -> [u32; 4] {
	[color[0].to_bits(), color[1].to_bits(), color[2].to_bits(), color[3].to_bits()]
}

/// Default on. `VEG_BAND_DEBUG=0` turns off poles and instance tints.
fn band_debug_enabled() -> bool {
	match std::env::var("VEG_BAND_DEBUG") {
		Ok(value) => {
			!matches!(value.trim().to_ascii_lowercase().as_str(), "0" | "off" | "false" | "no")
		}
		Err(_) => true,
	}
}

/// Loud packed-band colors. Medium is gold so it cannot be mistaken for High plants.
fn band_debug_color(band: NamedVisualLevel) -> [f32; 4] {
	match band {
		NamedVisualLevel::UltraLow => [1.0, 0.15, 0.95, 1.0],
		NamedVisualLevel::Low => [0.1, 0.95, 1.0, 1.0],
		NamedVisualLevel::Medium | NamedVisualLevel::High => [1.0, 0.82, 0.05, 1.0],
	}
}

fn host_debug_color(band: NamedVisualLevel, semantic_high: bool) -> [f32; 4] {
	if semantic_high {
		[0.9, 0.12, 0.12, 1.0]
	} else {
		band_debug_color(band)
	}
}

#[derive(Component)]
struct VisualBandDebugMarker;

#[derive(Resource, Default)]
struct BandDebugAssets {
	mesh: Option<Handle<Mesh>>,
}

#[derive(SystemParam)]
struct BandDebugCtx<'w> {
	materials: ResMut<'w, Assets<StandardMaterial>>,
	assets: ResMut<'w, BandDebugAssets>,
}

fn debug_color_material(
	color: [f32; 4],
	materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
	materials.add(StandardMaterial {
		base_color: Color::linear_rgba(color[0], color[1], color[2], color[3]),
		emissive: LinearRgba::new(color[0], color[1], color[2], 1.0) * 4.0,
		unlit: true,
		alpha_mode: AlphaMode::Opaque,
		..default()
	})
}

fn sync_instance_pbr_batches(
	mut commands: Commands,
	visuals: Query<(Entity, &ForestGroveVisual, &LodHostBounds, &ChildOf), With<VisualLodRoot>>,
	views: Query<(&Camera, &GlobalTransform), With<LodViewer>>,
	host_levels: Query<&LodSceneLevel>,
	mut scene_handles: ResMut<SceneRefHandles>,
	mut prototypes: ResMut<ScenePrototypeCache>,
	asset_server: Res<AssetServer>,
	mut world_assets: ResMut<Assets<bevy::world_serialization::WorldAsset>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut debug: BandDebugCtx,
	type_registry: Res<AppTypeRegistry>,
	mut state: ResMut<InstancePbrState>,
	mut existing: Query<&mut InstanceMaterialData, With<InstancePbrBatch>>,
	mut stats: ResMut<VisualHorizonStats>,
	time: Option<Res<Time>>,
	mut last_log: Local<f64>,
) {
	let mut seen = HashMap::<Entity, (NamedVisualLevel, bool, Aabb)>::new();
	for (entity, visual, bounds, child_of) in &visuals {
		let semantic_high = host_levels
			.get(child_of.parent())
			.is_ok_and(|level| *level == LodSceneLevel::High);
		seen.insert(
			entity,
			(
				selected_packed_band(visual, bounds, &views),
				semantic_high,
				batch_cull_aabb(Aabb::from(bounds.0)),
			),
		);
	}

	state.groves.retain(|entity, submit| {
		if seen.contains_key(entity) {
			true
		} else {
			for batch in submit.batches.values() {
				if let Ok(mut entity) = commands.get_entity(*batch) {
					entity.despawn();
				}
			}
			if let Some(marker) = submit.debug_marker {
				if let Ok(mut entity) = commands.get_entity(marker) {
					entity.despawn();
				}
			}
			false
		}
	});

	let mut preload = Vec::new();
	for (entity, visual, _, _) in &visuals {
		let Some(&(band, semantic_high, aabb)) = seen.get(&entity) else {
			continue;
		};
		if state.groves.get(&entity).is_some_and(|prev| {
			prev.band == band && prev.semantic_high == semantic_high && !prev.pending
		}) {
			continue;
		}
		let prev_empty = state.groves.get(&entity).is_none_or(|prev| prev.items.is_empty());
		let mut submit = compile_grove_submit(
			visual,
			band,
			semantic_high,
			&mut scene_handles,
			&mut prototypes,
			&asset_server,
			&mut world_assets,
			&mut meshes,
			&type_registry,
			&mut preload,
		);
		if let Some(prev) = state.groves.remove(&entity) {
			submit.batches = prev.batches;
			submit.debug_marker = prev.debug_marker;
		}
		if !(submit.pending && submit.items.is_empty() && prev_empty) {
			apply_grove_batches(&mut commands, entity, aabb, &mut submit, &mut existing);
		}
		sync_band_debug_marker(
			&mut commands,
			entity,
			band,
			semantic_high,
			aabb,
			&mut submit,
			&mut meshes,
			&mut debug,
		);
		state.groves.insert(entity, submit);
	}

	if !preload.is_empty() {
		prototypes.preload(&preload, &mut scene_handles, &asset_server);
	}

	stats.visual_roots = 0;
	stats.semantic_high = 0;
	stats.semantic_other = 0;
	stats.packed_instances = 0;
	stats.pending_groves = 0;
	stats.batch_entities = 0;
	stats.items = 0;
	stats.items_ultralow = 0;
	stats.items_low = 0;
	stats.items_medium = 0;
	stats.prototype_cache = prototypes.len();
	stats.sample_instance = [0.0; 3];
	stats.sample_aabb = [0.0; 3];
	for (entity, visual, _, _) in &visuals {
		stats.visual_roots += 1;
		stats.packed_instances += visual.representation.instances.len() as u32;
		let Some(&(band, semantic_high, _)) = seen.get(&entity) else {
			continue;
		};
		if semantic_high {
			stats.semantic_high += 1;
		} else {
			stats.semantic_other += 1;
		}
		if let Some(submit) = state.groves.get(&entity) {
			if submit.pending {
				stats.pending_groves += 1;
			}
			stats.batch_entities += submit.batches.len() as u32;
			let n = submit.items.len() as u32;
			stats.items += n;
			if stats.sample_instance == [0.0; 3] {
				if let Some((_, _, instance)) = submit.items.first() {
					stats.sample_instance = [instance.col3[0], instance.col3[1], instance.col3[2]];
				}
				if let Some(&(_, _, aabb)) = seen.get(&entity) {
					stats.sample_aabb = aabb.center.to_array();
				}
			}
			match band {
				NamedVisualLevel::UltraLow => stats.items_ultralow += n,
				NamedVisualLevel::Low => stats.items_low += n,
				NamedVisualLevel::Medium | NamedVisualLevel::High => stats.items_medium += n,
			}
		}
	}
	if let Some(time) = time.as_deref() {
		let now = time.elapsed_secs_f64();
		if now - *last_log >= 2.0 {
			*last_log = now;
			info!(
				target: "veg.horizon",
				"roots={} high={} other={} packed={} pending={} batches={} vis={} items={} ul={} low={} med={} cache={} desired={} presented={} present_q={} generate_q={} inst0=({:.1},{:.1},{:.1}) aabb0=({:.1},{:.1},{:.1}) tint={}",
				stats.visual_roots,
				stats.semantic_high,
				stats.semantic_other,
				stats.packed_instances,
				stats.pending_groves,
				stats.batch_entities,
				stats.batches_visible,
				stats.items,
				stats.items_ultralow,
				stats.items_low,
				stats.items_medium,
				stats.prototype_cache,
				stats.desired_in_keep,
				stats.presented,
				stats.present_pending,
				stats.generate_pending,
				stats.sample_instance[0],
				stats.sample_instance[1],
				stats.sample_instance[2],
				stats.sample_aabb[0],
				stats.sample_aabb[1],
				stats.sample_aabb[2],
				if band_debug_enabled() { "gold=med cyan=low mag=ul red=high" } else { "off" },
			);
		}
	}
}

fn count_instance_visibility(
	batches: Query<&ViewVisibility, With<InstancePbrBatch>>,
	mut stats: ResMut<VisualHorizonStats>,
) {
	stats.batches_visible = batches.iter().filter(|vis| vis.get()).count() as u32;
}

fn apply_grove_batches(
	commands: &mut Commands,
	grove: Entity,
	aabb: Aabb,
	submit: &mut GroveSubmit,
	existing: &mut Query<&mut InstanceMaterialData, With<InstancePbrBatch>>,
) {
	let mut buckets: HashMap<BatchKey, (Handle<Mesh>, Vec<InstanceData>)> = HashMap::new();
	for (key, mesh, instance) in &submit.items {
		buckets
			.entry(key.clone())
			.or_insert_with(|| (mesh.clone(), Vec::new()))
			.1
			.push(*instance);
	}

	submit.batches.retain(|key, entity| {
		if buckets.contains_key(key) {
			true
		} else {
			if let Ok(mut entity) = commands.get_entity(*entity) {
				entity.despawn();
			}
			false
		}
	});

	for (key, (mesh, instances)) in buckets {
		if let Some(&entity) = submit.batches.get(&key) {
			if let Ok(mut data) = existing.get_mut(entity) {
				*data = InstanceMaterialData(instances);
				if let Ok(mut entity) = commands.get_entity(entity) {
					entity.insert((aabb, NoAutoAabb));
				}
				continue;
			}
		}
		let entity = commands
			.spawn((
				InstancePbrBatch,
				Mesh3d(mesh),
				InstanceMaterialData(instances),
				Transform::IDENTITY,
				Visibility::Visible,
				aabb,
				NoAutoAabb,
				SyncToRenderWorld,
			))
			.id();
		commands.entity(grove).add_child(entity);
		submit.batches.insert(key, entity);
	}
}

fn compile_grove_submit(
	visual: &ForestGroveVisual,
	band: NamedVisualLevel,
	semantic_high: bool,
	scene_handles: &mut SceneRefHandles,
	prototypes: &mut ScenePrototypeCache,
	asset_server: &AssetServer,
	world_assets: &mut Assets<bevy::world_serialization::WorldAsset>,
	meshes: &mut Assets<Mesh>,
	type_registry: &AppTypeRegistry,
	preload: &mut Vec<SceneRef>,
) -> GroveSubmit {
	let mut items = Vec::new();
	let mut pending = false;
	if !semantic_high {
		for instance in visual.representation.instances.iter() {
			let Some(scene_ref) = instance.scene_at_or_coarser(band) else {
				continue;
			};
			let Some(prototype) = prototypes.try_resolve(
				scene_ref,
				scene_handles,
				asset_server,
				world_assets,
				meshes,
				type_registry,
			) else {
				pending = true;
				preload.push(scene_ref.clone());
				continue;
			};
			let color = if band_debug_enabled() {
				band_debug_color(band)
			} else {
				material_color(&instance.material)
			};
			let key_color = color_key(color);
			for part in &prototype.parts {
				let local = instance.transform * part.local_transform;
				items.push((
					BatchKey { mesh: part.mesh.id(), color: key_color },
					part.mesh.clone(),
					InstanceData::new(Mat4::from(local), color),
				));
			}
		}
	}
	GroveSubmit { band, semantic_high, pending, items, batches: HashMap::new(), debug_marker: None }
}

fn sync_band_debug_marker(
	commands: &mut Commands,
	grove: Entity,
	band: NamedVisualLevel,
	semantic_high: bool,
	aabb: Aabb,
	submit: &mut GroveSubmit,
	meshes: &mut Assets<Mesh>,
	debug: &mut BandDebugCtx,
) {
	if !band_debug_enabled() {
		if let Some(entity) = submit.debug_marker.take() {
			if let Ok(mut entity) = commands.get_entity(entity) {
				entity.despawn();
			}
		}
		return;
	}
	let mesh = debug
		.assets
		.mesh
		.get_or_insert_with(|| meshes.add(Mesh::from(Cuboid::new(12.0, 56.0, 12.0))))
		.clone();
	let material =
		debug_color_material(host_debug_color(band, semantic_high), &mut debug.materials);
	let transform = Transform::from_translation(Vec3::from(aabb.center));
	if let Some(entity) = submit.debug_marker {
		if let Ok(mut entity) = commands.get_entity(entity) {
			entity.insert((Mesh3d(mesh), MeshMaterial3d(material), transform));
			return;
		}
	}
	let entity = commands
		.spawn((
			VisualBandDebugMarker,
			Mesh3d(mesh),
			MeshMaterial3d(material),
			transform,
			Visibility::Visible,
		))
		.id();
	commands.entity(grove).add_child(entity);
	submit.debug_marker = Some(entity);
}

/// Copy instance lists when the render entity exists.
///
/// `Changed` plus `RenderEntity` on the same frame misses the spawn race:
/// the first change happens before the render entity is mapped, and later
/// frames are not `Changed`, so the render world stays empty (`queue ok=0`).
fn extract_instance_data(
	mut commands: Commands,
	main: Extract<Query<(RenderEntity, &InstanceMaterialData, Ref<InstanceMaterialData>)>>,
	existing: Query<(), With<InstanceMaterialData>>,
) {
	for (entity, data, changed) in &main {
		if existing.get(entity).is_ok() && !changed.is_changed() {
			continue;
		}
		commands.entity(entity).insert(data.clone());
	}
}

#[derive(Component)]
struct InstanceBuffer {
	buffer: Buffer,
	length: usize,
}

fn prepare_instance_buffers(
	mut commands: Commands,
	query: Query<
		(Entity, &InstanceMaterialData, Option<&InstanceBuffer>),
		Changed<InstanceMaterialData>,
	>,
	render_device: Res<RenderDevice>,
	render_queue: Res<RenderQueue>,
) {
	for (entity, instance_data, existing) in &query {
		if instance_data.is_empty() {
			continue;
		}
		let bytes = bytemuck::cast_slice(instance_data.as_slice());
		if let Some(existing) = existing {
			if existing.length == instance_data.len() {
				render_queue.write_buffer(&existing.buffer, 0, bytes);
				continue;
			}
		}
		let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
			label: Some("instance pbr data"),
			contents: bytes,
			usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
		});
		commands
			.entity(entity)
			.insert(InstanceBuffer { buffer, length: instance_data.len() });
	}
}

#[derive(Resource)]
struct InstancePbrPipeline {
	shader: Handle<Shader>,
	mesh_pipeline: MeshPipeline,
}

fn init_instance_pipeline(mut commands: Commands, mesh_pipeline: Res<MeshPipeline>) {
	commands.insert_resource(InstancePbrPipeline {
		shader: INSTANCE_SHADER_HANDLE,
		mesh_pipeline: mesh_pipeline.clone(),
	});
}

impl SpecializedMeshPipeline for InstancePbrPipeline {
	type Key = MeshPipelineKey;

	fn specialize(
		&self,
		key: Self::Key,
		layout: &MeshVertexBufferLayoutRef,
	) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
		let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
		descriptor.vertex.shader = self.shader.clone();
		descriptor.vertex.buffers.push(VertexBufferLayout {
			array_stride: size_of::<InstanceData>() as u64,
			step_mode: VertexStepMode::Instance,
			attributes: vec![
				VertexAttribute { format: VertexFormat::Float32x4, offset: 0, shader_location: 8 },
				VertexAttribute { format: VertexFormat::Float32x4, offset: 16, shader_location: 9 },
				VertexAttribute {
					format: VertexFormat::Float32x4,
					offset: 32,
					shader_location: 10,
				},
				VertexAttribute {
					format: VertexFormat::Float32x4,
					offset: 48,
					shader_location: 11,
				},
				VertexAttribute {
					format: VertexFormat::Float32x4,
					offset: 64,
					shader_location: 12,
				},
			],
		});
		if let Some(fragment) = descriptor.fragment.as_mut() {
			fragment.shader = self.shader.clone();
		}
		Ok(descriptor)
	}
}

#[derive(Default)]
struct QueueSkipCounts {
	query_n: u32,
	vis_skip: u32,
	no_mesh: u32,
	no_gpu: u32,
	specialize_fail: u32,
	ok: u32,
	ticks: u32,
}

fn queue_instance_pbr(
	opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
	custom_pipeline: Res<InstancePbrPipeline>,
	mut pipelines: ResMut<SpecializedMeshPipelines<InstancePbrPipeline>>,
	pipeline_cache: Res<PipelineCache>,
	meshes: Res<RenderAssets<RenderMesh>>,
	render_mesh_instances: Res<RenderMeshInstances>,
	material_meshes: Query<
		(Entity, &MainEntity, Option<&ViewVisibility>),
		With<InstanceMaterialData>,
	>,
	mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
	views: Query<&ExtractedView>,
	view_key_cache: Res<ViewKeyCache>,
	mut queued: Local<HashMap<RetainedViewEntity, HashSet<MainEntity>>>,
	mut skips: Local<QueueSkipCounts>,
) {
	let draw_custom = opaque_draw_functions.read().id::<DrawInstancePbr>();
	let mut live_views = HashSet::<RetainedViewEntity>::default();
	let ticks = skips.ticks.wrapping_add(1);
	*skips = QueueSkipCounts {
		query_n: material_meshes.iter().len() as u32,
		ticks,
		..Default::default()
	};
	for view in &views {
		let Some(opaque_phase) = opaque_render_phases.get_mut(&view.retained_view_entity) else {
			continue;
		};
		let Some(&view_key) = view_key_cache.get(&view.retained_view_entity) else {
			continue;
		};
		live_views.insert(view.retained_view_entity);
		let mut current = HashSet::<MainEntity>::default();
		for (entity, main_entity, vis) in &material_meshes {
			if vis.is_some_and(|vis| !vis.get()) {
				skips.vis_skip += 1;
				continue;
			}
			let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*main_entity)
			else {
				skips.no_mesh += 1;
				continue;
			};
			let Some(mesh) = meshes.get(mesh_instance.mesh_asset_id()) else {
				skips.no_gpu += 1;
				continue;
			};
			let key = view_key
				| MeshPipelineKey::from_primitive_topology_and_strip_index(
					mesh.primitive_topology(),
					mesh.index_format(),
				);
			let Ok(pipeline) =
				pipelines.specialize(&pipeline_cache, &custom_pipeline, key, &mesh.layout)
			else {
				skips.specialize_fail += 1;
				continue;
			};
			skips.ok += 1;
			opaque_phase.add(
				Opaque3dBatchSetKey {
					pipeline,
					draw_function: draw_custom,
					material_bind_group_index: None,
					slabs: MeshSlabs::default(),
					lightmap_slab: None,
				},
				Opaque3dBinKey { asset_id: mesh_instance.mesh_asset_id().into() },
				(entity, *main_entity),
				InputUniformIndex::default(),
				BinnedRenderPhaseType::NonMesh,
			);
			current.insert(*main_entity);
		}
		if let Some(previous) = queued.get(&view.retained_view_entity) {
			for old in previous {
				if !current.contains(old) {
					opaque_phase.remove(*old);
				}
			}
		}
		queued.insert(view.retained_view_entity, current);
	}
	queued.retain(|view, _| live_views.contains(view));
	if skips.ticks % 120 == 0 {
		info!(
			target: "veg.horizon",
			"queue query={} vis_skip={} no_mesh={} no_gpu={} spec_fail={} ok={}",
			skips.query_n,
			skips.vis_skip,
			skips.no_mesh,
			skips.no_gpu,
			skips.specialize_fail,
			skips.ok
		);
	}
}

type DrawInstancePbr = (
	SetItemPipeline,
	SetMeshViewBindGroup<0>,
	SetMeshViewBindingArrayBindGroup<1>,
	SetMeshBindGroup<2>,
	DrawMeshInstanced,
);

struct DrawMeshInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawMeshInstanced {
	type Param = (SRes<RenderAssets<RenderMesh>>, SRes<RenderMeshInstances>, SRes<MeshAllocator>);
	type ViewQuery = ();
	type ItemQuery = Read<InstanceBuffer>;

	#[inline]
	fn render<'w>(
		item: &P,
		_view: (),
		instance_buffer: Option<&'w InstanceBuffer>,
		(meshes, render_mesh_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
		pass: &mut TrackedRenderPass<'w>,
	) -> RenderCommandResult {
		let mesh_allocator = mesh_allocator.into_inner();
		let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(item.main_entity())
		else {
			return RenderCommandResult::Skip;
		};
		let Some(gpu_mesh) = meshes.into_inner().get(mesh_instance.mesh_asset_id()) else {
			return RenderCommandResult::Skip;
		};
		let Some(instance_buffer) = instance_buffer else {
			return RenderCommandResult::Skip;
		};
		let Some(vertex_buffer_slice) =
			mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id())
		else {
			return RenderCommandResult::Skip;
		};

		pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));
		pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

		match &gpu_mesh.buffer_info {
			RenderMeshBufferInfo::Indexed { index_format, count } => {
				let Some(index_buffer_slice) =
					mesh_allocator.mesh_index_slice(&mesh_instance.mesh_asset_id())
				else {
					return RenderCommandResult::Skip;
				};
				pass.set_index_buffer(index_buffer_slice.buffer.slice(..), *index_format);
				pass.draw_indexed(
					index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
					vertex_buffer_slice.range.start as i32,
					0..instance_buffer.length as u32,
				);
			}
			RenderMeshBufferInfo::NonIndexed => {
				pass.draw(vertex_buffer_slice.range, 0..instance_buffer.length as u32);
			}
		}
		RenderCommandResult::Success
	}
}
