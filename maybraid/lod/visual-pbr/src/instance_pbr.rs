//! Instanced submit: spatial visual root first, then `(mesh, material)`.
//!
//! Each visual root owns its batch entities. A band or representation change
//! only rebuilds that root.
//!
//! Invariant: instance matrices are world-space. The
//! shader does `clip_from_world * instance * vertex` and must not read
//! `mesh[0]` via `get_world_from_local(0u)`. The batch [`Aabb`] is the root
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
use bevy::ecs::system::SystemParamItem;
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
use lod::{LodHostBounds, NamedVisualLevel, VisualLodRoot, VisualLodSelection, VisualLodSystems};
use material_ref::{MaterialRef, MaterialRefKey};
use scene_ref::{ScenePrototypeCache, SceneRef, SceneRefHandles};

use crate::InstancePbrVisual;

const INSTANCE_SHADER_HANDLE: Handle<Shader> = uuid_handle!("6e2c1a0f-4b8d-4e91-9c3a-7f1d2b8e5a04");

/// Instanced renderer for ordinary banded [`lod::VisualInstanceList`] scenes.
pub struct InstancePbrRenderer;

impl lod::VisualLodRenderer<InstancePbrVisual> for InstancePbrRenderer {
	fn register(app: &mut App) {
		if !app.is_plugin_added::<InstancePbrPlugin>() {
			app.add_plugins(InstancePbrPlugin);
		}
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

#[derive(Clone, PartialEq, Eq, Hash)]
struct BatchKey {
	mesh: AssetId<Mesh>,
	material: MaterialRefKey,
}

#[derive(Resource, Default)]
struct InstancePbrMaterialCache {
	colors: HashMap<MaterialRefKey, [f32; 4]>,
}

impl InstancePbrMaterialCache {
	fn resolve(&mut self, material: &MaterialRef) -> (MaterialRefKey, [f32; 4]) {
		let key = MaterialRefKey::from(material);
		let color = *self.colors.entry(key.clone()).or_insert_with(|| material_color(material));
		(key, color)
	}
}

struct VisualSubmit {
	band: NamedVisualLevel,
	batches: HashMap<BatchKey, Entity>,
}

type BatchBuckets = HashMap<BatchKey, (Handle<Mesh>, Vec<InstanceData>)>;

struct VisualCompile {
	band: NamedVisualLevel,
	next_instance: usize,
	buckets: BatchBuckets,
}

#[derive(Resource, Default)]
struct InstancePbrState {
	roots: HashMap<Entity, VisualSubmit>,
	dirty_roots: HashSet<Entity>,
	compiles: HashMap<Entity, VisualCompile>,
}

/// Per-frame limits for incrementally compiling visual roots.
#[derive(Resource, Debug, Clone, Copy)]
pub struct InstancePbrCompileBudget {
	pub roots_per_frame: u32,
	pub instances_per_frame: u32,
}

impl Default for InstancePbrCompileBudget {
	fn default() -> Self {
		Self { roots_per_frame: 4, instances_per_frame: 4096 }
	}
}

pub struct InstancePbrPlugin;

impl Plugin for InstancePbrPlugin {
	fn build(&self, app: &mut App) {
		load_internal_asset!(app, INSTANCE_SHADER_HANDLE, "instance.wgsl", Shader::from_wgsl);
		app.init_resource::<InstancePbrState>()
			.init_resource::<InstancePbrMaterialCache>()
			.init_resource::<InstancePbrCompileBudget>()
			.add_systems(Update, sync_instance_pbr_batches.after(VisualLodSystems::Select));
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

/// Visual-root footprint XZ; Y is at least ±1000 m so instance poses above the
/// structural pancake stay inside the CPU frustum test.
fn batch_cull_aabb(footprint: Aabb) -> Aabb {
	let mut aabb = footprint;
	aabb.half_extents.y = aabb.half_extents.y.max(1000.0);
	aabb
}

fn material_color(material: &MaterialRef) -> [f32; 4] {
	material
		.palette
		.first()
		.map(|c| c.to_linear().to_f32_array())
		.unwrap_or([0.25, 0.55, 0.28, 1.0])
}

fn sync_instance_pbr_batches(
	mut commands: Commands,
	visuals: Query<
		(Entity, Ref<InstancePbrVisual>, Ref<LodHostBounds>, &VisualLodSelection<NamedVisualLevel>),
		With<VisualLodRoot>,
	>,
	mut scene_handles: ResMut<SceneRefHandles>,
	mut prototypes: ResMut<ScenePrototypeCache>,
	asset_server: Res<AssetServer>,
	mut world_assets: ResMut<Assets<bevy::world_serialization::WorldAsset>>,
	mut meshes: ResMut<Assets<Mesh>>,
	type_registry: Res<AppTypeRegistry>,
	mut material_cache: ResMut<InstancePbrMaterialCache>,
	mut state: ResMut<InstancePbrState>,
	budget: Res<InstancePbrCompileBudget>,
	mut existing: Query<&mut InstanceMaterialData, With<InstancePbrBatch>>,
) {
	let mut seen = HashMap::<Entity, (NamedVisualLevel, Aabb)>::new();
	for (entity, visual, bounds, selection) in &visuals {
		if visual.is_changed() || bounds.is_changed() {
			state.dirty_roots.insert(entity);
		}
		seen.insert(entity, (selection.0, batch_cull_aabb(Aabb::from(bounds.0))));
	}

	state.roots.retain(|entity, submit| {
		if seen.contains_key(entity) {
			true
		} else {
			for batch in submit.batches.values() {
				if let Ok(mut entity) = commands.get_entity(*batch) {
					entity.despawn();
				}
			}
			false
		}
	});
	state.dirty_roots.retain(|entity| seen.contains_key(entity));
	state.compiles.retain(|entity, _| seen.contains_key(entity));

	let mut preload = Vec::new();
	let mut roots_remaining = budget.roots_per_frame.max(1);
	let mut instances_remaining = budget.instances_per_frame.max(1);
	for (entity, visual, _, _) in &visuals {
		let Some(&(band, aabb)) = seen.get(&entity) else {
			continue;
		};
		let dirty = state.dirty_roots.remove(&entity);
		let stable = state.roots.get(&entity).is_some_and(|prev| prev.band == band);
		let compiling_same =
			state.compiles.get(&entity).is_some_and(|compile| compile.band == band);
		if dirty || (!stable && !compiling_same) {
			state
				.compiles
				.insert(entity, VisualCompile { band, next_instance: 0, buckets: HashMap::new() });
		} else if stable && !compiling_same {
			continue;
		}
		if roots_remaining == 0 || instances_remaining == 0 {
			continue;
		}
		roots_remaining -= 1;
		let Some(compile) = state.compiles.get_mut(&entity) else {
			continue;
		};
		let consumed = advance_visual_compile(
			&visual,
			compile,
			instances_remaining,
			&mut scene_handles,
			&mut prototypes,
			&asset_server,
			&mut world_assets,
			&mut meshes,
			&type_registry,
			&mut material_cache,
			&mut preload,
		);
		instances_remaining = instances_remaining.saturating_sub(consumed);
		if compile.next_instance < visual.representation.instances.len() {
			continue;
		}
		let Some(complete) = state.compiles.remove(&entity) else {
			continue;
		};
		let batches = state.roots.remove(&entity).map(|prev| prev.batches).unwrap_or_default();
		let mut submit = VisualSubmit { band, batches };
		apply_visual_batches(
			&mut commands,
			entity,
			aabb,
			complete.buckets,
			&mut submit,
			&mut existing,
		);
		state.roots.insert(entity, submit);
	}

	if !preload.is_empty() {
		prototypes.preload(&preload, &mut scene_handles, &asset_server);
	}
}

fn apply_visual_batches(
	commands: &mut Commands,
	root: Entity,
	aabb: Aabb,
	buckets: BatchBuckets,
	submit: &mut VisualSubmit,
	existing: &mut Query<&mut InstanceMaterialData, With<InstancePbrBatch>>,
) {
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
		commands.entity(root).add_child(entity);
		submit.batches.insert(key, entity);
	}
}

fn advance_visual_compile(
	visual: &InstancePbrVisual,
	compile: &mut VisualCompile,
	instance_budget: u32,
	scene_handles: &mut SceneRefHandles,
	prototypes: &mut ScenePrototypeCache,
	asset_server: &AssetServer,
	world_assets: &mut Assets<bevy::world_serialization::WorldAsset>,
	meshes: &mut Assets<Mesh>,
	type_registry: &AppTypeRegistry,
	materials: &mut InstancePbrMaterialCache,
	preload: &mut Vec<SceneRef>,
) -> u32 {
	let mut consumed = 0;
	while compile.next_instance < visual.representation.instances.len()
		&& consumed < instance_budget
	{
		let instance = &visual.representation.instances[compile.next_instance];
		let Some(scene_ref) = instance.scene_at_or_coarser(compile.band) else {
			compile.next_instance += 1;
			consumed += 1;
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
			preload.push(scene_ref.clone());
			break;
		};
		let (material, color) = materials.resolve(&instance.material);
		for part in &prototype.parts {
			let local = instance.transform * part.local_transform;
			compile
				.buckets
				.entry(BatchKey { mesh: part.mesh.id(), material: material.clone() })
				.or_insert_with(|| (part.mesh.clone(), Vec::new()))
				.1
				.push(InstanceData::new(Mat4::from(local), color));
		}
		compile.next_instance += 1;
		consumed += 1;
	}
	consumed
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
) {
	let draw_custom = opaque_draw_functions.read().id::<DrawInstancePbr>();
	let mut live_views = HashSet::<RetainedViewEntity>::default();
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
				continue;
			}
			let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*main_entity)
			else {
				continue;
			};
			let Some(mesh) = meshes.get(mesh_instance.mesh_asset_id()) else {
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
				continue;
			};
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
