//! Instanced PBR submit: bucket resolved prototypes and queue one draw per
//! `(mesh, material)`.
//!
//! Dummy batch entities exist only so Bevy uploads the shared mesh. Grove hosts
//! stay data-only — no tree [`Mesh3d`] children, no camera-driven cook.

use std::mem::size_of;

use bevy::asset::{load_internal_asset, uuid_handle, AssetId};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::color::ColorToComponents;
use bevy::core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d};
use bevy::ecs::query::QueryItem;
use bevy::ecs::system::lifetimeless::{Read, SRes};
use bevy::ecs::system::SystemParamItem;
use bevy::mesh::{MeshVertexBufferLayoutRef, VertexBufferLayout};
use bevy::pbr::{
	get_mesh_instance_world_from_local, MeshInputUniform, MeshPipeline, MeshPipelineKey,
	MeshPipelineSystems, MeshUniform, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup,
	SetMeshViewBindingArrayBindGroup, ViewKeyCache,
};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::batching::gpu_preprocessing::BatchedInstanceBuffers;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::mesh::allocator::MeshAllocator;
use bevy::render::mesh::{RenderMesh, RenderMeshBufferInfo};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_phase::{
	AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
	RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
};
use bevy::render::render_resource::*;
use bevy::render::renderer::RenderDevice;
use bevy::render::sync_component::SyncComponent;
use bevy::render::sync_world::{MainEntity, SyncToRenderWorld};
use bevy::render::view::ExtractedView;
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
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

#[derive(Component, Deref, Clone)]
struct InstanceMaterialData(Vec<InstanceData>);

impl SyncComponent for InstanceMaterialData {
	type Target = Self;
}

impl ExtractComponent for InstanceMaterialData {
	type QueryData = &'static InstanceMaterialData;
	type QueryFilter = ();
	type Out = Self;

	fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
		Some(InstanceMaterialData(item.0.clone()))
	}
}

#[derive(Component)]
struct InstancePbrBatch;

#[derive(Clone, PartialEq, Eq, Hash)]
struct BatchKey {
	mesh: AssetId<Mesh>,
	color: [u32; 4],
}

#[derive(Resource, Default)]
struct InstancePbrBatchEntities {
	by_key: HashMap<BatchKey, Entity>,
}

#[derive(Resource, Default)]
struct InstancePbrSubmitKey {
	groves: Vec<(Entity, NamedVisualLevel, bool)>,
}

pub struct InstancePbrPlugin;

impl Plugin for InstancePbrPlugin {
	fn build(&self, app: &mut App) {
		load_internal_asset!(app, INSTANCE_SHADER_HANDLE, "instance.wgsl", Shader::from_wgsl);
		app.init_resource::<InstancePbrBatchEntities>()
			.init_resource::<InstancePbrSubmitKey>()
			.add_plugins(ExtractComponentPlugin::<InstanceMaterialData>::default())
			.add_systems(Update, sync_instance_pbr_batches);
		let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
			return;
		};
		render_app
			.add_render_command::<Transparent3d, DrawInstancePbr>()
			.init_resource::<SpecializedMeshPipelines<InstancePbrPipeline>>()
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

fn sync_instance_pbr_batches(
	mut commands: Commands,
	visuals: Query<
		(Entity, &ForestGroveVisual, &LodHostBounds, &GlobalTransform, &ChildOf),
		With<VisualLodRoot>,
	>,
	views: Query<(&Camera, &GlobalTransform), With<LodViewer>>,
	host_levels: Query<&LodSceneLevel>,
	mut scene_handles: ResMut<SceneRefHandles>,
	mut prototypes: ResMut<ScenePrototypeCache>,
	asset_server: Res<AssetServer>,
	mut world_assets: ResMut<Assets<bevy::world_serialization::WorldAsset>>,
	mut meshes: ResMut<Assets<Mesh>>,
	type_registry: Res<AppTypeRegistry>,
	mut batch_entities: ResMut<InstancePbrBatchEntities>,
	mut submit_key: ResMut<InstancePbrSubmitKey>,
	mut existing: Query<&mut InstanceMaterialData, With<InstancePbrBatch>>,
) {
	let mut next_key = Vec::new();
	let mut buckets: HashMap<BatchKey, (Handle<Mesh>, Vec<InstanceData>)> = HashMap::new();
	let mut preload: Vec<SceneRef> = Vec::new();

	for (entity, visual, bounds, grove_xf, child_of) in &visuals {
		let semantic_high = host_levels
			.get(child_of.parent())
			.is_ok_and(|level| *level == LodSceneLevel::High);
		let band = selected_packed_band(visual, bounds, &views);
		next_key.push((entity, band, semantic_high));
		if semantic_high {
			continue;
		}
		let grove_world = grove_xf.affine();
		for instance in visual.representation.instances.iter() {
			let Some(scene_ref) = instance.scene_for(band).cloned() else {
				continue;
			};
			let Some(prototype) = prototypes.try_resolve(
				&scene_ref,
				&mut scene_handles,
				&asset_server,
				&mut world_assets,
				&mut meshes,
				&type_registry,
			) else {
				preload.push(scene_ref);
				continue;
			};
			let color = material_color(&instance.material);
			let key_color = color_key(color);
			for part in &prototype.parts {
				let world = grove_world * instance.transform * part.local_transform;
				let key = BatchKey { mesh: part.mesh.id(), color: key_color };
				buckets
					.entry(key)
					.or_insert_with(|| (part.mesh.clone(), Vec::new()))
					.1
					.push(InstanceData::new(Mat4::from(world), color));
			}
		}
	}

	if !preload.is_empty() {
		prototypes.preload(&preload, &mut scene_handles, &asset_server);
	}

	if next_key == submit_key.groves
		&& buckets.len() == batch_entities.by_key.len()
		&& buckets.keys().all(|k| batch_entities.by_key.contains_key(k))
	{
		return;
	}
	submit_key.groves = next_key;

	let mut stale: Vec<BatchKey> = batch_entities
		.by_key
		.keys()
		.filter(|key| !buckets.contains_key(*key))
		.cloned()
		.collect();
	for key in stale.drain(..) {
		if let Some(entity) = batch_entities.by_key.remove(&key) {
			commands.entity(entity).despawn();
		}
	}

	for (key, (mesh, instances)) in buckets {
		if let Some(&entity) = batch_entities.by_key.get(&key) {
			if let Ok(mut data) = existing.get_mut(entity) {
				*data = InstanceMaterialData(instances);
				continue;
			}
		}
		let entity = commands
			.spawn((
				InstancePbrBatch,
				Mesh3d(mesh),
				InstanceMaterialData(instances),
				Transform::IDENTITY,
				Visibility::Inherited,
				NoFrustumCulling,
				SyncToRenderWorld,
			))
			.id();
		batch_entities.by_key.insert(key, entity);
	}
}

#[derive(Component)]
struct InstanceBuffer {
	buffer: Buffer,
	length: usize,
}

fn prepare_instance_buffers(
	mut commands: Commands,
	query: Query<(Entity, &InstanceMaterialData)>,
	render_device: Res<RenderDevice>,
) {
	for (entity, instance_data) in &query {
		if instance_data.is_empty() {
			continue;
		}
		let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
			label: Some("instance pbr data"),
			contents: bytemuck::cast_slice(instance_data.as_slice()),
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
	transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
	custom_pipeline: Res<InstancePbrPipeline>,
	mut pipelines: ResMut<SpecializedMeshPipelines<InstancePbrPipeline>>,
	pipeline_cache: Res<PipelineCache>,
	meshes: Res<RenderAssets<RenderMesh>>,
	render_mesh_instances: Res<RenderMeshInstances>,
	maybe_batched_instance_buffers: Option<
		Res<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
	>,
	material_meshes: Query<(Entity, &MainEntity), With<InstanceMaterialData>>,
	mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
	views: Query<&ExtractedView>,
	view_key_cache: Res<ViewKeyCache>,
) {
	let draw_custom = transparent_3d_draw_functions.read().id::<DrawInstancePbr>();
	for view in &views {
		let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
		else {
			continue;
		};
		let Some(&view_key) = view_key_cache.get(&view.retained_view_entity) else {
			continue;
		};
		for (entity, main_entity) in &material_meshes {
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
			transparent_phase.add_retained(Transparent3d {
				sorting_info: TransparentSortingInfo3d::Sorted {
					mesh_center: get_mesh_instance_world_from_local(
						*main_entity,
						mesh_instance.current_uniform_index,
						&render_mesh_instances,
						maybe_batched_instance_buffers.as_deref(),
					)
					.transform_point3(mesh.aabb_center),
					depth_bias: 0.0,
				},
				entity: (entity, *main_entity),
				pipeline,
				draw_function: draw_custom,
				distance: 0.0,
				batch_range: 0..1,
				extra_index: PhaseItemExtraIndex::None,
				indexed: true,
			});
		}
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
