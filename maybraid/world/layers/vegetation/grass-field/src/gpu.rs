//! Render-world compute compaction, indirect args, and opaque draws.

use std::borrow::Cow;
use std::collections::HashMap;

use bevy::core_pipeline::core_3d::{
	Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey, CORE_3D_DEPTH_FORMAT,
};
use bevy::core_pipeline::schedule::camera_driver;
use bevy::ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy::mesh::VertexBufferLayout;
use bevy::prelude::*;
use bevy::render::camera::{DirtySpecializations, PendingQueues};
use bevy::render::mesh::allocator::MeshSlabs;
use bevy::render::render_phase::{
	AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, InputUniformIndex, PhaseItem,
	RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewBinnedRenderPhases,
};
use bevy::render::render_resource::binding_types::{
	storage_buffer_read_only_sized, storage_buffer_sized, uniform_buffer,
};
use bevy::render::render_resource::{
	BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, Buffer,
	BufferDescriptor, BufferInitDescriptor, BufferUsages, CachedComputePipelineId,
	CachedPipelineState, Canonical, ColorTargetState, ColorWrites, CompareFunction,
	ComputePassDescriptor, ComputePipelineDescriptor, DepthStencilState, FragmentState,
	IndexFormat, PipelineCache, PrimitiveState, RenderPipeline, RenderPipelineDescriptor,
	ShaderStages, Specializer, SpecializerKey, TextureFormat, Variants, VertexAttribute,
	VertexFormat, VertexState, VertexStepMode,
};
use bevy::render::renderer::{
	RenderAdapter, RenderContext, RenderDevice, RenderGraph, RenderQueue,
};
use bevy::render::sync_world::MainEntityHashSet;
use bevy::render::view::{ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewTarget};
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use vegetation_components::CanopyShadowMode;
use vegetation_shaders::wesl::{
	GeometryLod, GrassDrawParams, GrassViewUniform, LodThresholds, BLADE_DESCRIPTOR_STRIDE,
	DRAW_INDEXED_INDIRECT_STRIDE, GRASS_LOD_COUNT, GRASS_VIEW_KIND_CAMERA, GRASS_VIEW_KIND_SHADOW,
};

use crate::capability::{GrassGpuCapability, GrassGpuMode};
use crate::catalog::{select_dispatch_size, CatalogApply, GroveEncodeStats, GroveId};
use crate::diagnostics::GrassFieldStats;
use crate::gpu_arena::{
	encode_arena_copies, grow_list_buffers, grow_storage_buffer, stage_buffer_write,
	upload_visible_spans, PendingBufferCopy, PendingBufferWrite, INITIAL_LIST_ITEMS,
};
use crate::grove_catalog::{GrassCatalogOps, GrassGroveCatalog};
use crate::grove_cull::cull_grass_patches;
use crate::grove_record::GroveVisibleSpan;
use crate::grove_retain::{remember_bands, GroveRetainPolicy, GroveSubmit, RetainedGroveFrame};
use crate::plugin::{ExtractedGrassField, GrassFieldPresenter};
use crate::select::GrassSelector;
use crate::shaders::{BLADE_SHADER, FINALIZE_SHADER, SELECT_SHADER};
use crate::template::BladeTemplate;
use crate::SELECT_WORKGROUP;

pub struct GrassGpuPlugin;

impl Plugin for GrassGpuPlugin {
	fn build(&self, app: &mut App) {
		let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
			return;
		};
		render_app
			.init_resource::<PendingGrassQueues>()
			.init_resource::<GrassGpuState>()
			.insert_resource(GrassGroveCatalog::render())
			.init_resource::<GroveEncodeStats>()
			.add_render_command::<Opaque3d, DrawGrass>()
			.add_systems(RenderStartup, (detect_capability, init_grass_pipelines, init_templates))
			.add_systems(
				Render,
				(
					prepare_grass_buffers.in_set(RenderSystems::PrepareResources),
					queue_grass.in_set(RenderSystems::Queue),
				),
			)
			.add_systems(RenderGraph, grass_compute.before(camera_driver));
	}
}

#[derive(Default, Deref, DerefMut, Resource)]
struct PendingGrassQueues(PendingQueues);

fn detect_capability(mut commands: Commands, adapter: Res<RenderAdapter>) {
	commands.insert_resource(GrassGpuCapability::detect(&adapter));
}

#[derive(Resource)]
struct GrassPipelines {
	select_layout: BindGroupLayoutDescriptor,
	finalize_layout: BindGroupLayoutDescriptor,
	draw_layout: BindGroupLayoutDescriptor,
	select_pipeline: CachedComputePipelineId,
	finalize_pipeline: CachedComputePipelineId,
	draw_variants: Variants<RenderPipeline, GrassDrawSpecializer>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, SpecializerKey)]
struct GrassDrawKey {
	msaa: u32,
	format: TextureFormat,
}

struct GrassDrawSpecializer;

impl Specializer<RenderPipeline> for GrassDrawSpecializer {
	type Key = GrassDrawKey;

	fn specialize(
		&self,
		key: Self::Key,
		descriptor: &mut RenderPipelineDescriptor,
	) -> Result<Canonical<Self::Key>, BevyError> {
		descriptor.multisample.count = key.msaa;
		if let Some(fragment) = descriptor.fragment.as_mut() {
			if let Some(Some(target)) = fragment.targets.first_mut() {
				target.format = key.format;
			}
		}
		Ok(key)
	}
}

fn init_grass_pipelines(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	pipeline_cache: Res<PipelineCache>,
) {
	let select_layout = BindGroupLayoutDescriptor::new(
		"grass_select_layout",
		&BindGroupLayoutEntries::sequential(
			ShaderStages::COMPUTE,
			(
				uniform_buffer::<GrassViewUniform>(false),
				storage_buffer_read_only_sized(false, None),
				storage_buffer_sized(false, None),
				storage_buffer_sized(false, None),
				storage_buffer_sized(false, None),
				storage_buffer_sized(false, None),
				storage_buffer_read_only_sized(false, None),
			),
		),
	);
	let finalize_layout = BindGroupLayoutDescriptor::new(
		"grass_finalize_layout",
		&BindGroupLayoutEntries::sequential(
			ShaderStages::COMPUTE,
			(
				uniform_buffer::<GrassViewUniform>(false),
				storage_buffer_sized(false, None),
				storage_buffer_sized(false, None),
			),
		),
	);
	let draw_layout = BindGroupLayoutDescriptor::new(
		"grass_draw_layout",
		&BindGroupLayoutEntries::sequential(
			ShaderStages::VERTEX | ShaderStages::FRAGMENT,
			(
				uniform_buffer::<GrassViewUniform>(false),
				storage_buffer_read_only_sized(false, None),
				storage_buffer_read_only_sized(false, None),
				uniform_buffer::<GrassDrawParams>(false),
			),
		),
	);

	let select_shader = asset_server.load(SELECT_SHADER);
	let finalize_shader = asset_server.load(FINALIZE_SHADER);
	let blade_shader = asset_server.load(BLADE_SHADER);

	let select_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
		label: Some("grass_select".into()),
		layout: vec![select_layout.clone()],
		shader: select_shader,
		entry_point: Some(Cow::from("select")),
		..default()
	});
	let finalize_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
		label: Some("grass_finalize".into()),
		layout: vec![finalize_layout.clone()],
		shader: finalize_shader,
		entry_point: Some(Cow::from("finalize")),
		..default()
	});

	let draw_layout_for_pipeline = draw_layout.clone();
	let base_descriptor = RenderPipelineDescriptor {
		label: Some("grass_blade".into()),
		layout: vec![draw_layout_for_pipeline],
		vertex: VertexState {
			shader: blade_shader.clone(),
			buffers: vec![VertexBufferLayout {
				array_stride: BladeTemplate::VERTEX_STRIDE,
				step_mode: VertexStepMode::Vertex,
				attributes: vec![VertexAttribute {
					format: VertexFormat::Float32x2,
					offset: 0,
					shader_location: 0,
				}],
			}],
			..default()
		},
		fragment: Some(FragmentState {
			shader: blade_shader,
			targets: vec![Some(ColorTargetState {
				format: TextureFormat::Rgba8UnormSrgb,
				blend: None,
				write_mask: ColorWrites::ALL,
			})],
			..default()
		}),
		primitive: PrimitiveState { cull_mode: None, ..default() },
		depth_stencil: Some(DepthStencilState {
			format: CORE_3D_DEPTH_FORMAT,
			depth_write_enabled: Some(true),
			depth_compare: Some(CompareFunction::GreaterEqual),
			stencil: default(),
			bias: default(),
		}),
		..default()
	};

	commands.insert_resource(GrassPipelines {
		select_layout,
		finalize_layout,
		draw_layout,
		select_pipeline,
		finalize_pipeline,
		draw_variants: Variants::new(GrassDrawSpecializer, base_descriptor),
	});
}

#[derive(Resource)]
struct GrassTemplates {
	meshes: [GpuTemplate; GRASS_LOD_COUNT],
}

struct GpuTemplate {
	vertices: Buffer,
	indices: Buffer,
	index_count: u32,
}

fn init_templates(mut commands: Commands, device: Res<RenderDevice>) {
	let lods = [GeometryLod::Near, GeometryLod::Mid, GeometryLod::Card];
	let meshes = lods.map(|lod| {
		let template = BladeTemplate::for_lod(lod);
		GpuTemplate {
			vertices: device.create_buffer_with_data(&BufferInitDescriptor {
				label: Some("grass_template_vertices"),
				contents: &template.vertex_bytes(),
				usage: BufferUsages::VERTEX,
			}),
			indices: device.create_buffer_with_data(&BufferInitDescriptor {
				label: Some("grass_template_indices"),
				contents: &template.index_bytes(),
				usage: BufferUsages::INDEX,
			}),
			index_count: template.indices.len() as u32,
		}
	});
	commands.insert_resource(GrassTemplates { meshes });
}

#[derive(Resource, Default)]
struct GrassGpuState {
	blades: Option<Buffer>,
	blade_count: u32,
	generation: u64,
	pending_copies: Vec<PendingBufferCopy>,
	pending_writes: Vec<PendingBufferWrite>,
	views: std::collections::HashMap<RetainedViewEntity, ViewWork>,
}

struct ViewWork {
	view_uniform: Buffer,
	counters: Buffer,
	lists: [Buffer; GRASS_LOD_COUNT],
	indirect: Buffer,
	/// Kept so GPU buffer handles outlive the bind groups that reference them.
	#[allow(dead_code)]
	draw_params: [Buffer; GRASS_LOD_COUNT],
	select_bind_group: BindGroup,
	finalize_bind_group: BindGroup,
	draw_bind_groups: [BindGroup; GRASS_LOD_COUNT],
	cpu_instance_counts: [u32; GRASS_LOD_COUNT],
	span_buffer: Buffer,
	dirty: bool,
	retained: RetainedGroveFrame,
	last_bands: std::collections::HashMap<GroveId, u32>,
	visible_blades: u32,
	span_count: u32,
	list_capacity: u32,
}

fn prepare_grass_buffers(
	extracted: Option<Res<ExtractedGrassField>>,
	mut ops: ResMut<GrassCatalogOps>,
	mut catalog: ResMut<GrassGroveCatalog>,
	capability: Option<Res<GrassGpuCapability>>,
	pipelines: Option<Res<GrassPipelines>>,
	pipeline_cache: Res<PipelineCache>,
	device: Res<RenderDevice>,
	queue: Res<RenderQueue>,
	views: Query<&ExtractedView>,
	mut stats: Option<ResMut<GrassFieldStats>>,
	mut encode: ResMut<GroveEncodeStats>,
	mut state: ResMut<GrassGpuState>,
	policy: Option<Res<GroveRetainPolicy>>,
) {
	let Some(extracted) = extracted else {
		return;
	};
	let Some(pipelines) = pipelines else {
		return;
	};
	let Some(capability) = capability else {
		return;
	};
	if !capability.submits() {
		return;
	}
	let CatalogApply { leftover, .. } = catalog.apply(ops.take(), crate::ADMISSION_BUDGET);
	ops.ops.extend(leftover);
	encode.catalog_writes = catalog.last_writes;
	encode.catalog_bytes = catalog.last_write_bytes;
	encode.catalog_clones = catalog.clones;
	encode.truncated_plants = catalog.truncated_plants;
	encode.passes = 0;
	encode.dispatches = 0;
	encode.draws = 0;
	encode.uniform_writes = 0;
	encode.span_uploads = 0;
	encode.retained_clean = 0;

	let mut pending_copies = std::mem::take(&mut state.pending_copies);
	let mut pending_writes = std::mem::take(&mut state.pending_writes);
	let mut grew = false;
	if catalog.last_writes > 0 {
		let blade_bytes =
			(catalog.blade_slots() as u64 * BLADE_DESCRIPTOR_STRIDE).max(BLADE_DESCRIPTOR_STRIDE);
		grew = grow_storage_buffer(
			&device,
			&mut state.blades,
			blade_bytes,
			BLADE_DESCRIPTOR_STRIDE,
			"grass_blades",
			&mut pending_copies,
		);
		{
			for upload in &catalog.last_uploads {
				if upload.blades.is_empty() {
					continue;
				}
				let bytes = catalog.encode_blades(upload);
				if bytes.is_empty() {
					continue;
				}
				if let Some(buffer) = state.blades.as_ref() {
					stage_buffer_write(
						&device,
						buffer,
						u64::from(upload.blades.offset) * BLADE_DESCRIPTOR_STRIDE,
						&bytes,
						&mut pending_writes,
					);
				}
			}
		}
		state.blade_count = catalog.blade_slots() as u32;
		state.generation = catalog.generation;
	}
	if catalog.is_empty() {
		state.views.clear();
		state.pending_copies = pending_copies;
		state.pending_writes = pending_writes;
		return;
	}

	let Some(blades) = state.blades.clone() else {
		state.pending_copies = pending_copies;
		state.pending_writes = pending_writes;
		return;
	};
	let force_rebuild = policy.map(|policy| policy.forces_rebuild()).unwrap_or(false);
	let mut live = std::collections::HashSet::new();
	let mut missing = Vec::new();
	for view in &views {
		if !view_submits_grass(view, extracted.shadow_mode) {
			continue;
		}
		live.insert(view.retained_view_entity);
		if !state.views.contains_key(&view.retained_view_entity) {
			missing.push(view.retained_view_entity);
		}
	}
	for retained in missing {
		if let Some(view) = views.iter().find(|view| view.retained_view_entity == retained) {
			let work = create_view_work(
				&device,
				&queue,
				&pipeline_cache,
				&pipelines,
				&blades,
				view,
				&extracted,
			);
			state.views.insert(retained, work);
		}
	}
	state.views.retain(|key, _| live.contains(key));
	for view in &views {
		if !view_submits_grass(view, extracted.shadow_mode) {
			continue;
		}
		let Some(work) = state.views.get_mut(&view.retained_view_entity) else {
			continue;
		};
		let (span_grew, list_grew) = stamp_grass_view(
			&device,
			&queue,
			view,
			&extracted,
			&catalog,
			work,
			&mut encode,
			force_rebuild,
			&mut pending_copies,
		);
		if grew || span_grew || list_grew {
			rebind_grass_select(&device, &pipeline_cache, &pipelines, &blades, work);
		}
	}
	state.pending_copies = pending_copies;
	state.pending_writes = pending_writes;

	if capability.mode == GrassGpuMode::CpuSelect {
		cpu_fill_lists(
			&device,
			&mut state,
			&extracted,
			&catalog,
			views.iter(),
			stats.as_deref_mut(),
		);
	} else if extracted.settings.diagnostics {
		record_cpu_stats(&extracted, &catalog, views.iter(), stats.as_deref_mut());
	}
}

fn make_view_uniform(view: &ExtractedView, extracted: &ExtractedGrassField) -> GrassViewUniform {
	let world_from_view = view.world_from_view.to_matrix();
	let view_from_world = world_from_view.inverse();
	let clip_from_world = view.clip_from_world.unwrap_or(view.clip_from_view * view_from_world);
	let world_from_clip = clip_from_world.inverse();
	let kind = if view.retained_view_entity.subview_index == 0 {
		GRASS_VIEW_KIND_CAMERA
	} else {
		GRASS_VIEW_KIND_SHADOW
	};
	let thresholds = if extracted.grove_residency {
		LodThresholds::grove_grass()
	} else {
		extracted.settings.lod
	};
	GrassViewUniform {
		clip_from_world,
		world_from_clip,
		world_position: view.world_from_view.translation().extend(extracted.time_s),
		prev_world_position: view.world_from_view.translation().extend(extracted.prev_time_s),
		resolution: Vec4::new(
			view.viewport.z as f32,
			view.viewport.w as f32,
			fov_y(&view.clip_from_view),
			kind as f32,
		),
		wind: Vec4::new(
			extracted.settings.wind.direction_xz.x,
			extracted.settings.wind.direction_xz.y,
			extracted.settings.wind.speed,
			extracted.settings.wind.strength_m,
		),
		lod_distances: Vec4::new(
			extracted.settings.wind.speed,
			thresholds.near_m,
			thresholds.mid_m,
			thresholds.card_m,
		),
		lod_density: Vec4::new(
			thresholds.near_density,
			thresholds.mid_density,
			thresholds.card_density,
			thresholds.min_height_px,
		),
		counts: Vec4::new(
			extracted.blade_count as f32,
			extracted.settings.max_per_lod as f32,
			0.0,
			0.0,
		),
		light_direction: extracted.light_direction.extend(0.0),
	}
}

fn fov_y(clip_from_view: &Mat4) -> f32 {
	let f = clip_from_view.y_axis.y.abs().max(1e-5);
	2.0 * (1.0 / f).atan()
}

fn create_view_work(
	device: &RenderDevice,
	queue: &RenderQueue,
	pipeline_cache: &PipelineCache,
	pipelines: &GrassPipelines,
	blades: &Buffer,
	view: &ExtractedView,
	extracted: &ExtractedGrassField,
) -> ViewWork {
	let view_uniform = device.create_buffer(&BufferDescriptor {
		label: Some("grass_view_uniform"),
		size: 256,
		usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
		mapped_at_creation: false,
	});
	let counters = device.create_buffer(&BufferDescriptor {
		label: Some("grass_counters"),
		size: 32,
		usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
		mapped_at_creation: false,
	});
	let list_size = u64::from(INITIAL_LIST_ITEMS) * 4;
	let lists = [0, 1, 2].map(|_| {
		device.create_buffer(&BufferDescriptor {
			label: Some("grass_lod_list"),
			size: list_size,
			usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
			mapped_at_creation: false,
		})
	});
	let indirect = device.create_buffer(&BufferDescriptor {
		label: Some("grass_indirect"),
		size: DRAW_INDEXED_INDIRECT_STRIDE * GRASS_LOD_COUNT as u64,
		usage: BufferUsages::STORAGE
			| BufferUsages::INDIRECT
			| BufferUsages::COPY_DST
			| BufferUsages::COPY_SRC,
		mapped_at_creation: false,
	});
	let draw_params = [0u32, 1, 2].map(|lod| {
		let params = GrassDrawParams {
			lod_bucket: lod,
			index_count: GeometryLod::from_bucket(lod).expect("lod").index_count(),
			vertex_count: GeometryLod::from_bucket(lod).expect("lod").vertex_count(),
			view_kind: 0,
		};
		device.create_buffer_with_data(&BufferInitDescriptor {
			label: Some("grass_draw_params"),
			contents: &params.to_std140_bytes(),
			usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
		})
	});

	let span_buffer = device.create_buffer(&BufferDescriptor {
		label: Some("grass_visible_spans"),
		size: GroveVisibleSpan::STRIDE * 256,
		usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
		mapped_at_creation: false,
	});

	let uniform = make_view_uniform(view, extracted);
	write_view_uniform(queue, &view_uniform, &uniform);

	let select_bind_group = device.create_bind_group(
		Some("grass_select_bg"),
		&pipeline_cache.get_bind_group_layout(&pipelines.select_layout),
		&BindGroupEntries::sequential((
			view_uniform.as_entire_buffer_binding(),
			blades.as_entire_buffer_binding(),
			counters.as_entire_buffer_binding(),
			lists[0].as_entire_buffer_binding(),
			lists[1].as_entire_buffer_binding(),
			lists[2].as_entire_buffer_binding(),
			span_buffer.as_entire_buffer_binding(),
		)),
	);
	let finalize_bind_group = device.create_bind_group(
		Some("grass_finalize_bg"),
		&pipeline_cache.get_bind_group_layout(&pipelines.finalize_layout),
		&BindGroupEntries::sequential((
			view_uniform.as_entire_buffer_binding(),
			counters.as_entire_buffer_binding(),
			indirect.as_entire_buffer_binding(),
		)),
	);
	let draw_bind_groups = [0, 1, 2].map(|lod| {
		device.create_bind_group(
			Some("grass_draw_bg"),
			&pipeline_cache.get_bind_group_layout(&pipelines.draw_layout),
			&BindGroupEntries::sequential((
				view_uniform.as_entire_buffer_binding(),
				blades.as_entire_buffer_binding(),
				lists[lod].as_entire_buffer_binding(),
				draw_params[lod].as_entire_buffer_binding(),
			)),
		)
	});

	ViewWork {
		view_uniform,
		counters,
		lists,
		indirect,
		draw_params,
		select_bind_group,
		finalize_bind_group,
		draw_bind_groups,
		cpu_instance_counts: [0; GRASS_LOD_COUNT],
		span_buffer,
		dirty: true,
		retained: RetainedGroveFrame::default(),
		last_bands: std::collections::HashMap::new(),
		visible_blades: 0,
		span_count: 0,
		list_capacity: INITIAL_LIST_ITEMS,
	}
}

fn write_view_uniform(queue: &RenderQueue, buffer: &Buffer, uniform: &GrassViewUniform) {
	queue.write_buffer(buffer, 0, &uniform.to_std140_bytes());
}

fn stamp_grass_view(
	device: &RenderDevice,
	queue: &RenderQueue,
	view: &ExtractedView,
	extracted: &ExtractedGrassField,
	catalog: &GrassGroveCatalog,
	work: &mut ViewWork,
	encode: &mut GroveEncodeStats,
	force_rebuild: bool,
	pending_copies: &mut Vec<PendingBufferCopy>,
) -> (bool, bool) {
	let thresholds = if extracted.grove_residency {
		LodThresholds::grove_grass()
	} else {
		extracted.settings.lod
	};
	let probe = make_view_uniform(view, extracted);
	let mut groves = if extracted.grove_residency {
		cull_grass_patches(&probe, thresholds, catalog.live_patches(), &|id| {
			work.last_bands.get(&id).copied()
		})
	} else {
		Vec::new()
	};
	catalog.stamp_content_gen(&mut groves);
	remember_bands(&mut work.last_bands, &groves);
	let submit = GroveSubmit::plan(
		&work.retained,
		view.viewport.z,
		view.viewport.w,
		fov_y(&view.clip_from_view),
		thresholds.card_m,
		groves,
		extracted.grove_residency,
		extracted.blade_count,
		0,
		force_rebuild,
	);
	let _span = if submit.dirty {
		bevy::log::info_span!("grove rebuild").entered()
	} else {
		bevy::log::info_span!("grove retained (clean)").entered()
	};
	let stamped = {
		let span_buffer = &mut work.span_buffer;
		submit.stamp(&mut work.retained, encode, |groves| {
			upload_visible_spans(device, queue, span_buffer, groves, "grass_visible_spans")
		})
	};
	work.dirty = stamped.dirty;
	work.visible_blades = stamped.work_branches;
	work.span_count = stamped.span_count;
	let list_grew = grow_list_buffers(
		device,
		work.lists.iter_mut(),
		stamped.work_branches.max(1),
		"grass_instance_list",
		pending_copies,
	);
	if list_grew {
		work.list_capacity = (work.lists[0].size() / 4) as u32;
	}
	let mut uniform = make_view_uniform(view, extracted);
	uniform.counts.x = stamped.work_branches as f32;
	uniform.counts.y = work.list_capacity as f32;
	uniform.light_direction.w = stamped.span_count as f32;
	write_view_uniform(queue, &work.view_uniform, &uniform);
	(stamped.span_grew, list_grew)
}

fn rebind_grass_select(
	device: &RenderDevice,
	pipeline_cache: &PipelineCache,
	pipelines: &GrassPipelines,
	blades: &Buffer,
	work: &mut ViewWork,
) {
	work.select_bind_group = device.create_bind_group(
		Some("grass_select_bg"),
		&pipeline_cache.get_bind_group_layout(&pipelines.select_layout),
		&BindGroupEntries::sequential((
			work.view_uniform.as_entire_buffer_binding(),
			blades.as_entire_buffer_binding(),
			work.counters.as_entire_buffer_binding(),
			work.lists[0].as_entire_buffer_binding(),
			work.lists[1].as_entire_buffer_binding(),
			work.lists[2].as_entire_buffer_binding(),
			work.span_buffer.as_entire_buffer_binding(),
		)),
	);
	work.draw_bind_groups = [0, 1, 2].map(|lod| {
		device.create_bind_group(
			Some("grass_draw_bg"),
			&pipeline_cache.get_bind_group_layout(&pipelines.draw_layout),
			&BindGroupEntries::sequential((
				work.view_uniform.as_entire_buffer_binding(),
				blades.as_entire_buffer_binding(),
				work.lists[lod].as_entire_buffer_binding(),
				work.draw_params[lod].as_entire_buffer_binding(),
			)),
		)
	});
}

fn cpu_fill_lists<'a>(
	device: &RenderDevice,
	state: &mut GrassGpuState,
	extracted: &ExtractedGrassField,
	catalog: &GrassGroveCatalog,
	views: impl Iterator<Item = &'a ExtractedView>,
	mut stats: Option<&mut GrassFieldStats>,
) {
	for view in views {
		if !view_submits_grass(view, extracted.shadow_mode) {
			continue;
		}
		let Some(work) = state.views.get_mut(&view.retained_view_entity) else {
			continue;
		};
		if !work.dirty {
			continue;
		}
		let uniform = make_view_uniform(view, extracted);
		let selector = GrassSelector::from_view(uniform, &extracted.settings);
		let spans = work.retained.groves.iter().map(|grove| grove.span).collect::<Vec<_>>();
		let lists = if extracted.grove_residency {
			selector.compact_catalog(catalog, Some(&spans))
		} else {
			selector.compact_catalog(catalog, None)
		};
		for (lod, indices) in lists.indices.iter().enumerate() {
			if !indices.is_empty() {
				let bytes: Vec<u8> = indices.iter().flat_map(|i| i.to_le_bytes()).collect();
				stage_buffer_write(device, &work.lists[lod], 0, &bytes, &mut state.pending_writes);
			}
			work.cpu_instance_counts[lod] = indices.len() as u32;
		}
		if extracted.settings.diagnostics {
			record_lists(extracted, &lists, stats.as_deref_mut());
		}
	}
}

fn record_cpu_stats<'a>(
	extracted: &ExtractedGrassField,
	catalog: &GrassGroveCatalog,
	views: impl Iterator<Item = &'a ExtractedView>,
	mut stats: Option<&mut GrassFieldStats>,
) {
	for view in views {
		if !view_submits_grass(view, extracted.shadow_mode) {
			continue;
		}
		let uniform = make_view_uniform(view, extracted);
		let selector = GrassSelector::from_view(uniform, &extracted.settings);
		let lists = selector.compact_catalog(catalog, None);
		record_lists(extracted, &lists, stats.as_deref_mut());
		break;
	}
}

fn record_lists(
	extracted: &ExtractedGrassField,
	lists: &crate::select::CompactedLists,
	stats: Option<&mut GrassFieldStats>,
) {
	if let Some(stats) = stats {
		*stats =
			GrassFieldStats::from_lists(extracted.blade_count, extracted.patch_count, 0, lists);
	}
}

fn view_submits_grass(view: &ExtractedView, shadow_mode: CanopyShadowMode) -> bool {
	view.retained_view_entity.subview_index == 0 || shadow_mode.canopy_casts()
}

fn grass_compute(
	mut render_context: RenderContext,
	pipelines: Option<Res<GrassPipelines>>,
	pipeline_cache: Res<PipelineCache>,
	mut state: Option<ResMut<GrassGpuState>>,
	capability: Option<Res<GrassGpuCapability>>,
	extracted: Option<Res<ExtractedGrassField>>,
	device: Res<RenderDevice>,
	mut encode: Option<ResMut<GroveEncodeStats>>,
) {
	if let Some(state) = state.as_deref_mut() {
		encode_arena_copies(
			render_context.command_encoder(),
			&mut state.pending_copies,
			&mut state.pending_writes,
		);
	}
	let Some(pipelines) = pipelines else {
		return;
	};
	let Some(state) = state.as_deref() else {
		return;
	};
	let Some(capability) = capability else {
		return;
	};
	let Some(extracted) = extracted else {
		return;
	};
	if capability.mode != GrassGpuMode::GpuDriven || extracted.blade_count == 0 {
		return;
	}
	match pipeline_cache.get_compute_pipeline_state(pipelines.select_pipeline) {
		CachedPipelineState::Ok(_) => {}
		_ => return,
	};
	let Some(select_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.select_pipeline)
	else {
		return;
	};
	let Some(finalize_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.finalize_pipeline)
	else {
		return;
	};

	let max_dim = device.limits().max_compute_workgroups_per_dimension.max(1);
	for work in state.views.values() {
		if !work.dirty {
			continue;
		}
		let groups = select_dispatch_size(work.visible_blades, SELECT_WORKGROUP, max_dim);
		render_context.command_encoder().clear_buffer(&work.counters, 0, None);
		{
			let mut pass =
				render_context.command_encoder().begin_compute_pass(&ComputePassDescriptor {
					label: Some("grass_select"),
					..default()
				});
			if work.visible_blades > 0 {
				pass.set_bind_group(0, &work.select_bind_group, &[]);
				pass.set_pipeline(select_pipeline);
				pass.dispatch_workgroups(groups.x, groups.y, groups.z);
			}
			pass.set_bind_group(0, &work.finalize_bind_group, &[]);
			pass.set_pipeline(finalize_pipeline);
			pass.dispatch_workgroups(1, 1, 1);
		}
		if let Some(encode) = encode.as_deref_mut() {
			GroveSubmit::record_select(encode, true, false, work.visible_blades, 0);
		}
	}
}

fn grass_color_format(view: &ExtractedView, target: Option<&ViewTarget>) -> TextureFormat {
	target.map(ViewTarget::main_texture_format).unwrap_or(view.target_format)
}

fn queue_grass(
	pipeline_cache: Res<PipelineCache>,
	mut pipelines: Option<ResMut<GrassPipelines>>,
	mut opaque_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
	opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
	views: Query<(&ExtractedView, &RenderVisibleEntities, &Msaa, Option<&ViewTarget>)>,
	dirty: Res<DirtySpecializations>,
	mut pending: ResMut<PendingGrassQueues>,
	mut queued: Local<MainEntityHashSet>,
	mut queued_format: Local<HashMap<RetainedViewEntity, TextureFormat>>,
	extracted: Option<Res<ExtractedGrassField>>,
	capability: Option<Res<GrassGpuCapability>>,
	mut encode: ResMut<GroveEncodeStats>,
) {
	let Some(extracted) = extracted else {
		return;
	};
	let Some(capability) = capability else {
		return;
	};
	let Some(pipelines) = pipelines.as_mut() else {
		return;
	};
	if !capability.submits() || extracted.blade_count == 0 {
		return;
	}
	let draw_grass = opaque_draw_functions.read().id::<DrawGrass>();
	for (view, visible, msaa, view_target) in &views {
		if !view_submits_grass(view, extracted.shadow_mode) {
			continue;
		}
		let Some(phase) = opaque_phases.get_mut(&view.retained_view_entity) else {
			continue;
		};
		let Some(visible_entities) = visible.get::<GrassFieldPresenter>() else {
			continue;
		};
		let format = grass_color_format(view, view_target);
		let format_changed =
			queued_format.insert(view.retained_view_entity, format) != Some(format);
		let pending_queues = pending.prepare_for_new_frame(view.retained_view_entity);
		for &main_entity in dirty.iter_to_dequeue(view.retained_view_entity, visible_entities) {
			phase.remove(main_entity);
		}
		if format_changed {
			for (_render_entity, main_entity) in visible_entities.iter_visible() {
				phase.remove(*main_entity);
			}
		}
		let queue_targets: Vec<_> = if format_changed {
			visible_entities.iter_visible().map(|(render, main)| (*render, *main)).collect()
		} else {
			dirty
				.iter_to_queue(
					view.retained_view_entity,
					visible_entities,
					&pending_queues.prev_frame,
					&mut queued,
				)
				.map(|(render, main)| (*render, *main))
				.collect()
		};
		for (render_entity, main_entity) in &queue_targets {
			let Ok(pipeline_id) = pipelines
				.draw_variants
				.specialize(&pipeline_cache, GrassDrawKey { msaa: msaa.samples(), format })
			else {
				continue;
			};
			phase.add(
				Opaque3dBatchSetKey {
					draw_function: draw_grass,
					pipeline: pipeline_id,
					material_bind_group_index: None,
					lightmap_slab: None,
					slabs: MeshSlabs::default(),
				},
				Opaque3dBinKey { asset_id: AssetId::<Mesh>::default().untyped() },
				(*render_entity, *main_entity),
				InputUniformIndex::default(),
				BinnedRenderPhaseType::NonMesh,
			);
			encode.draws += GRASS_LOD_COUNT as u32;
		}
	}
}

type DrawGrass = (SetItemPipeline, DrawGrassCommand);

struct DrawGrassCommand;

impl<P: PhaseItem> RenderCommand<P> for DrawGrassCommand {
	type Param = (SRes<GrassGpuState>, SRes<GrassTemplates>, SRes<GrassGpuCapability>);
	type ViewQuery = &'static ExtractedView;
	type ItemQuery = ();

	fn render<'w>(
		_item: &P,
		view: bevy::ecs::query::ROQueryItem<'w, '_, Self::ViewQuery>,
		_entity: Option<bevy::ecs::query::ROQueryItem<'w, '_, Self::ItemQuery>>,
		(state, templates, capability): SystemParamItem<'w, '_, Self::Param>,
		pass: &mut TrackedRenderPass<'w>,
	) -> RenderCommandResult {
		let state = state.into_inner();
		let templates = templates.into_inner();
		let capability = capability.into_inner();
		let Some(work) = state.views.get(&view.retained_view_entity) else {
			return RenderCommandResult::Skip;
		};
		for lod in 0..GRASS_LOD_COUNT {
			pass.set_bind_group(0, &work.draw_bind_groups[lod], &[]);
			pass.set_vertex_buffer(0, templates.meshes[lod].vertices.slice(..));
			pass.set_index_buffer(templates.meshes[lod].indices.slice(..), IndexFormat::Uint32);
			match capability.mode {
				GrassGpuMode::GpuDriven => {
					pass.draw_indexed_indirect(
						&work.indirect,
						DRAW_INDEXED_INDIRECT_STRIDE * lod as u64,
					);
				}
				GrassGpuMode::CpuSelect => {
					let instances = work.cpu_instance_counts[lod];
					if instances > 0 {
						pass.draw_indexed(0..templates.meshes[lod].index_count, 0, 0..instances);
					}
				}
				GrassGpuMode::Unsupported => {}
			}
		}
		RenderCommandResult::Success
	}
}
