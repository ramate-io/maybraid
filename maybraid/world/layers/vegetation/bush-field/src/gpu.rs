//! Render-world compute compaction, indirect args, and opaque wood/foliage draws.

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
use vegetation_grass_field::{
	select_dispatch_size, CatalogApply, GrassGpuCapability, GrassGpuMode, GroveEncodeStats,
};
use vegetation_shaders::wesl::{
	GrassDrawParams, GrassViewUniform, LodThresholds, DRAW_INDEXED_INDIRECT_STRIDE,
	GRASS_VIEW_KIND_CAMERA, GRASS_VIEW_KIND_SHADOW,
};

use crate::catalog::{BushCatalogOps, BushGroveCatalog};
use crate::diagnostics::BushFieldStats;
use crate::layout::{
	BranchDescriptor, LeafDescriptor, PlantDescriptor, WoodLod, BRANCH_DESCRIPTOR_STRIDE,
	BUSH_LOD_COUNT, LEAF_DESCRIPTOR_STRIDE, PLANT_DESCRIPTOR_STRIDE,
};
use crate::plugin::{BushFieldPresenter, ExtractedBushField};
use crate::select::BushSelector;
use crate::shaders::{
	FINALIZE_SHADER, FOLIAGE_SHADER, SELECT_FOLIAGE_SHADER, SELECT_WOOD_SHADER, WOOD_SHADER,
};
use crate::template::{LeafTemplate, WoodTemplate};
use crate::SELECT_WORKGROUP;

pub struct BushGpuPlugin;

impl Plugin for BushGpuPlugin {
	fn build(&self, app: &mut App) {
		let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
			return;
		};
		render_app
			.init_resource::<PendingBushQueues>()
			.init_resource::<BushGpuState>()
			.init_resource::<BushGroveCatalog>()
			.init_resource::<GroveEncodeStats>()
			.add_render_command::<Opaque3d, DrawBushWood>()
			.add_render_command::<Opaque3d, DrawBushFoliage>()
			.add_systems(RenderStartup, (detect_capability, init_bush_pipelines, init_templates))
			.add_systems(
				Render,
				(
					prepare_bush_buffers.in_set(RenderSystems::PrepareResources),
					queue_bush.in_set(RenderSystems::Queue),
				),
			)
			.add_systems(RenderGraph, bush_compute.before(camera_driver));
	}
}

#[derive(Default, Deref, DerefMut, Resource)]
struct PendingBushQueues(PendingQueues);

fn detect_capability(mut commands: Commands, adapter: Res<RenderAdapter>) {
	commands.insert_resource(GrassGpuCapability::detect(&adapter));
}

#[derive(Resource)]
struct BushPipelines {
	select_wood_layout: BindGroupLayoutDescriptor,
	select_foliage_layout: BindGroupLayoutDescriptor,
	finalize_layout: BindGroupLayoutDescriptor,
	wood_draw_layout: BindGroupLayoutDescriptor,
	foliage_draw_layout: BindGroupLayoutDescriptor,
	select_wood: CachedComputePipelineId,
	select_foliage: CachedComputePipelineId,
	finalize: CachedComputePipelineId,
	wood_variants: Variants<RenderPipeline, BushDrawSpecializer>,
	foliage_variants: Variants<RenderPipeline, BushDrawSpecializer>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, SpecializerKey)]
struct BushDrawKey {
	msaa: u32,
	format: TextureFormat,
	hdr: bool,
}

struct BushDrawSpecializer;

impl Specializer<RenderPipeline> for BushDrawSpecializer {
	type Key = BushDrawKey;

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

fn vertex_layout() -> VertexBufferLayout {
	VertexBufferLayout {
		array_stride: WoodTemplate::VERTEX_STRIDE,
		step_mode: VertexStepMode::Vertex,
		attributes: vec![VertexAttribute {
			format: VertexFormat::Float32x2,
			offset: 0,
			shader_location: 0,
		}],
	}
}

fn init_bush_pipelines(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	pipeline_cache: Res<PipelineCache>,
) {
	let select_wood_layout = BindGroupLayoutDescriptor::new(
		"bush_select_wood_layout",
		&BindGroupLayoutEntries::sequential(
			ShaderStages::COMPUTE,
			(
				uniform_buffer::<GrassViewUniform>(false),
				storage_buffer_read_only_sized(false, None),
				storage_buffer_read_only_sized(false, None),
				storage_buffer_sized(false, None),
				storage_buffer_sized(false, None),
				storage_buffer_sized(false, None),
				storage_buffer_sized(false, None),
			),
		),
	);
	let select_foliage_layout = BindGroupLayoutDescriptor::new(
		"bush_select_foliage_layout",
		&BindGroupLayoutEntries::sequential(
			ShaderStages::COMPUTE,
			(
				uniform_buffer::<GrassViewUniform>(false),
				storage_buffer_read_only_sized(false, None),
				storage_buffer_read_only_sized(false, None),
				storage_buffer_read_only_sized(false, None),
				storage_buffer_sized(false, None),
				storage_buffer_sized(false, None),
				storage_buffer_sized(false, None),
				storage_buffer_sized(false, None),
			),
		),
	);
	let finalize_layout = BindGroupLayoutDescriptor::new(
		"bush_finalize_layout",
		&BindGroupLayoutEntries::sequential(
			ShaderStages::COMPUTE,
			(
				uniform_buffer::<GrassViewUniform>(false),
				storage_buffer_sized(false, None),
				storage_buffer_sized(false, None),
			),
		),
	);
	let wood_draw_layout = BindGroupLayoutDescriptor::new(
		"bush_wood_draw_layout",
		&BindGroupLayoutEntries::sequential(
			ShaderStages::VERTEX | ShaderStages::FRAGMENT,
			(
				uniform_buffer::<GrassViewUniform>(false),
				storage_buffer_read_only_sized(false, None),
				storage_buffer_read_only_sized(false, None),
				storage_buffer_read_only_sized(false, None),
				uniform_buffer::<GrassDrawParams>(false),
			),
		),
	);
	let foliage_draw_layout = BindGroupLayoutDescriptor::new(
		"bush_foliage_draw_layout",
		&BindGroupLayoutEntries::sequential(
			ShaderStages::VERTEX | ShaderStages::FRAGMENT,
			(
				uniform_buffer::<GrassViewUniform>(false),
				storage_buffer_read_only_sized(false, None),
				storage_buffer_read_only_sized(false, None),
				storage_buffer_read_only_sized(false, None),
				storage_buffer_read_only_sized(false, None),
				uniform_buffer::<GrassDrawParams>(false),
			),
		),
	);

	let select_wood = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
		label: Some("bush_select_wood".into()),
		layout: vec![select_wood_layout.clone()],
		shader: asset_server.load(SELECT_WOOD_SHADER),
		entry_point: Some(Cow::from("select")),
		..default()
	});
	let select_foliage = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
		label: Some("bush_select_foliage".into()),
		layout: vec![select_foliage_layout.clone()],
		shader: asset_server.load(SELECT_FOLIAGE_SHADER),
		entry_point: Some(Cow::from("select")),
		..default()
	});
	let finalize = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
		label: Some("bush_finalize".into()),
		layout: vec![finalize_layout.clone()],
		shader: asset_server.load(FINALIZE_SHADER),
		entry_point: Some(Cow::from("finalize")),
		..default()
	});

	let wood_shader = asset_server.load(WOOD_SHADER);
	let foliage_shader = asset_server.load(FOLIAGE_SHADER);
	let wood_descriptor =
		opaque_descriptor("bush_wood", wood_shader, vec![wood_draw_layout.clone()]);
	let foliage_descriptor =
		opaque_descriptor("bush_foliage", foliage_shader, vec![foliage_draw_layout.clone()]);

	commands.insert_resource(BushPipelines {
		select_wood_layout,
		select_foliage_layout,
		finalize_layout,
		wood_draw_layout,
		foliage_draw_layout,
		select_wood,
		select_foliage,
		finalize,
		wood_variants: Variants::new(BushDrawSpecializer, wood_descriptor),
		foliage_variants: Variants::new(BushDrawSpecializer, foliage_descriptor),
	});
}

fn opaque_descriptor(
	label: &'static str,
	shader: Handle<Shader>,
	layout: Vec<BindGroupLayoutDescriptor>,
) -> RenderPipelineDescriptor {
	RenderPipelineDescriptor {
		label: Some(label.into()),
		layout,
		vertex: VertexState { shader: shader.clone(), buffers: vec![vertex_layout()], ..default() },
		fragment: Some(FragmentState {
			shader,
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
	}
}

#[derive(Resource)]
struct BushTemplates {
	wood: [GpuTemplate; BUSH_LOD_COUNT],
	leaf: GpuTemplate,
}

struct GpuTemplate {
	vertices: Buffer,
	indices: Buffer,
	index_count: u32,
}

fn init_templates(mut commands: Commands, device: Res<RenderDevice>) {
	let lods = [WoodLod::Near, WoodLod::Mid, WoodLod::Far];
	let wood = lods.map(|lod| {
		gpu_template(
			&device,
			&WoodTemplate::for_lod(lod).vertex_bytes(),
			&WoodTemplate::for_lod(lod).index_bytes(),
			WoodTemplate::for_lod(lod).indices.len() as u32,
			"bush_wood_template",
		)
	});
	let leaf = LeafTemplate::teardrop();
	commands.insert_resource(BushTemplates {
		wood,
		leaf: gpu_template(
			&device,
			&leaf.vertex_bytes(),
			&leaf.index_bytes(),
			leaf.index_count(),
			"bush_leaf_template",
		),
	});
}

fn gpu_template(
	device: &RenderDevice,
	vertices: &[u8],
	indices: &[u8],
	index_count: u32,
	label: &'static str,
) -> GpuTemplate {
	GpuTemplate {
		vertices: device.create_buffer_with_data(&BufferInitDescriptor {
			label: Some(label),
			contents: vertices,
			usage: BufferUsages::VERTEX,
		}),
		indices: device.create_buffer_with_data(&BufferInitDescriptor {
			label: Some(label),
			contents: indices,
			usage: BufferUsages::INDEX,
		}),
		index_count,
	}
}

#[derive(Resource, Default)]
struct BushGpuState {
	plants: Option<Buffer>,
	branches: Option<Buffer>,
	leaves: Option<Buffer>,
	generation: u64,
	plant_count: usize,
	branch_count: usize,
	leaf_count: usize,
	views: std::collections::HashMap<RetainedViewEntity, ViewWork>,
}

struct ViewWork {
	view_uniform: Buffer,
	counters: Buffer,
	wood_lists: [Buffer; BUSH_LOD_COUNT],
	foliage_lists: [Buffer; BUSH_LOD_COUNT],
	indirect: Buffer,
	#[allow(dead_code)]
	wood_params: [Buffer; BUSH_LOD_COUNT],
	#[allow(dead_code)]
	foliage_params: [Buffer; BUSH_LOD_COUNT],
	select_wood_bg: BindGroup,
	select_foliage_bg: BindGroup,
	finalize_bg: BindGroup,
	wood_draw_bgs: [BindGroup; BUSH_LOD_COUNT],
	foliage_draw_bgs: [BindGroup; BUSH_LOD_COUNT],
	cpu_wood_counts: [u32; BUSH_LOD_COUNT],
	cpu_foliage_counts: [u32; BUSH_LOD_COUNT],
}

fn prepare_bush_buffers(
	extracted: Option<Res<ExtractedBushField>>,
	mut ops: ResMut<BushCatalogOps>,
	mut catalog: ResMut<BushGroveCatalog>,
	capability: Option<Res<GrassGpuCapability>>,
	pipelines: Option<Res<BushPipelines>>,
	pipeline_cache: Res<PipelineCache>,
	device: Res<RenderDevice>,
	queue: Res<RenderQueue>,
	views: Query<&ExtractedView>,
	mut stats: Option<ResMut<BushFieldStats>>,
	mut encode: ResMut<GroveEncodeStats>,
	mut state: ResMut<BushGpuState>,
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
	let CatalogApply { leftover, .. } =
		catalog.apply(ops.take(), vegetation_grass_field::ADMISSION_BUDGET);
	ops.ops.extend(leftover);
	encode.catalog_writes = catalog.last_writes;
	encode.catalog_bytes = catalog.last_write_bytes;
	encode.catalog_clones = catalog.clones;
	encode.truncated_plants = catalog.truncated_plants;
	encode.passes = 0;
	encode.dispatches = 0;
	encode.draws = 0;
	encode.uniform_writes = 0;

	if catalog.last_writes > 0 {
		let plant_bytes =
			(catalog.plants.len() as u64 * PLANT_DESCRIPTOR_STRIDE).max(PLANT_DESCRIPTOR_STRIDE);
		let branch_bytes = (catalog.branches.len() as u64 * BRANCH_DESCRIPTOR_STRIDE)
			.max(BRANCH_DESCRIPTOR_STRIDE);
		let leaf_bytes =
			(catalog.leaves.len() as u64 * LEAF_DESCRIPTOR_STRIDE).max(LEAF_DESCRIPTOR_STRIDE);
		let grew = state.plants.as_ref().is_none_or(|buf| buf.size() < plant_bytes)
			|| state.branches.as_ref().is_none_or(|buf| buf.size() < branch_bytes)
			|| state.leaves.as_ref().is_none_or(|buf| buf.size() < leaf_bytes);
		if grew {
			state.plants = Some(pack_buffer(
				&device,
				"bush_plants",
				catalog.plants.iter().flat_map(|p| p.to_std430_bytes()).collect(),
				PLANT_DESCRIPTOR_STRIDE,
			));
			state.branches = Some(pack_buffer(
				&device,
				"bush_branches",
				catalog.branches.iter().flat_map(|b| b.to_std430_bytes()).collect(),
				BRANCH_DESCRIPTOR_STRIDE,
			));
			state.leaves = Some(pack_buffer(
				&device,
				"bush_leaves",
				catalog.leaves.iter().flat_map(|l| l.to_std430_bytes()).collect(),
				LEAF_DESCRIPTOR_STRIDE,
			));
			state.views.clear();
		} else {
			for upload in &catalog.last_uploads {
				write_span(
					&queue,
					state.plants.as_ref(),
					upload.plants,
					&catalog.plants,
					PLANT_DESCRIPTOR_STRIDE,
					PlantDescriptor::to_std430_bytes,
				);
				write_span(
					&queue,
					state.branches.as_ref(),
					upload.branches,
					&catalog.branches,
					BRANCH_DESCRIPTOR_STRIDE,
					BranchDescriptor::to_std430_bytes,
				);
				write_span(
					&queue,
					state.leaves.as_ref(),
					upload.leaves,
					&catalog.leaves,
					LEAF_DESCRIPTOR_STRIDE,
					LeafDescriptor::to_std430_bytes,
				);
			}
		}
		state.generation = catalog.generation;
		state.plant_count = catalog.plants.len();
		state.branch_count = catalog.branches.len();
		state.leaf_count = catalog.leaves.len();
	}
	if catalog.is_empty() {
		state.views.clear();
		return;
	}

	let (Some(plants), Some(branches), Some(leaves)) =
		(state.plants.clone(), state.branches.clone(), state.leaves.clone())
	else {
		return;
	};
	let capacity = extracted.settings.max_per_lod;
	let mut live = std::collections::HashSet::new();
	let mut missing = Vec::new();
	for view in &views {
		if !view_submits_bush(view, extracted.shadow_mode) {
			continue;
		}
		live.insert(view.retained_view_entity);
		if let Some(work) = state.views.get(&view.retained_view_entity) {
			write_view_uniform(&queue, &work.view_uniform, &make_view_uniform(view, &extracted));
			encode.uniform_writes += 1;
		} else {
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
				&plants,
				&branches,
				&leaves,
				capacity,
				view,
				&extracted,
			);
			state.views.insert(retained, work);
			encode.uniform_writes += 1;
		}
	}
	state.views.retain(|key, _| live.contains(key));

	if capability.mode == GrassGpuMode::CpuSelect {
		cpu_fill_lists(
			&queue,
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

fn pack_buffer(device: &RenderDevice, label: &'static str, bytes: Vec<u8>, stride: u64) -> Buffer {
	let packed = if bytes.is_empty() { vec![0u8; stride as usize] } else { bytes };
	device.create_buffer_with_data(&BufferInitDescriptor {
		label: Some(label),
		contents: &packed,
		usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
	})
}

fn write_span<T, const N: usize>(
	queue: &RenderQueue,
	buffer: Option<&Buffer>,
	span: vegetation_grass_field::RangeSpan,
	items: &[T],
	stride: u64,
	encode: fn(T) -> [u8; N],
) where
	T: Copy,
{
	if span.is_empty() {
		return;
	}
	let Some(buffer) = buffer else {
		return;
	};
	let start = span.offset as usize;
	let end = (start + span.count as usize).min(items.len());
	if start >= end {
		return;
	}
	let bytes: Vec<u8> = items[start..end].iter().copied().flat_map(encode).collect();
	queue.write_buffer(buffer, span.offset as u64 * stride, &bytes);
}

fn make_view_uniform(view: &ExtractedView, extracted: &ExtractedBushField) -> GrassViewUniform {
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
		LodThresholds::grove_bush()
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
			extracted.branch_count as f32,
			extracted.settings.max_per_lod as f32,
			extracted.leaf_count as f32,
			extracted.plant_count as f32,
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
	pipelines: &BushPipelines,
	plants: &Buffer,
	branches: &Buffer,
	leaves: &Buffer,
	capacity: u32,
	view: &ExtractedView,
	extracted: &ExtractedBushField,
) -> ViewWork {
	let view_uniform = device.create_buffer(&BufferDescriptor {
		label: Some("bush_view_uniform"),
		size: 256,
		usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
		mapped_at_creation: false,
	});
	let counters = device.create_buffer(&BufferDescriptor {
		label: Some("bush_counters"),
		size: 64,
		usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
		mapped_at_creation: false,
	});
	let list_size = (capacity as u64).max(1) * 4;
	let wood_lists = [0, 1, 2].map(|_| {
		device.create_buffer(&BufferDescriptor {
			label: Some("bush_wood_list"),
			size: list_size,
			usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
			mapped_at_creation: false,
		})
	});
	let foliage_lists = [0, 1, 2].map(|_| {
		device.create_buffer(&BufferDescriptor {
			label: Some("bush_foliage_list"),
			size: list_size,
			usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
			mapped_at_creation: false,
		})
	});
	let indirect = device.create_buffer(&BufferDescriptor {
		label: Some("bush_indirect"),
		size: DRAW_INDEXED_INDIRECT_STRIDE * 6,
		usage: BufferUsages::STORAGE
			| BufferUsages::INDIRECT
			| BufferUsages::COPY_DST
			| BufferUsages::COPY_SRC,
		mapped_at_creation: false,
	});
	let wood_params = [0u32, 1, 2].map(|lod| {
		params_buffer(
			device,
			lod,
			WoodLod::from_bucket(lod).map(|w| w.index_count()).unwrap_or(0),
			WoodLod::from_bucket(lod).map(|w| w.vertex_count()).unwrap_or(0),
		)
	});
	let foliage_params = [0u32, 1, 2].map(|lod| params_buffer(device, lod, 6, 4));

	let uniform = make_view_uniform(view, extracted);
	write_view_uniform(queue, &view_uniform, &uniform);

	let select_wood_bg = device.create_bind_group(
		Some("bush_select_wood_bg"),
		&pipeline_cache.get_bind_group_layout(&pipelines.select_wood_layout),
		&BindGroupEntries::sequential((
			view_uniform.as_entire_buffer_binding(),
			plants.as_entire_buffer_binding(),
			branches.as_entire_buffer_binding(),
			counters.as_entire_buffer_binding(),
			wood_lists[0].as_entire_buffer_binding(),
			wood_lists[1].as_entire_buffer_binding(),
			wood_lists[2].as_entire_buffer_binding(),
		)),
	);
	let select_foliage_bg = device.create_bind_group(
		Some("bush_select_foliage_bg"),
		&pipeline_cache.get_bind_group_layout(&pipelines.select_foliage_layout),
		&BindGroupEntries::sequential((
			view_uniform.as_entire_buffer_binding(),
			plants.as_entire_buffer_binding(),
			branches.as_entire_buffer_binding(),
			leaves.as_entire_buffer_binding(),
			counters.as_entire_buffer_binding(),
			foliage_lists[0].as_entire_buffer_binding(),
			foliage_lists[1].as_entire_buffer_binding(),
			foliage_lists[2].as_entire_buffer_binding(),
		)),
	);
	let finalize_bg = device.create_bind_group(
		Some("bush_finalize_bg"),
		&pipeline_cache.get_bind_group_layout(&pipelines.finalize_layout),
		&BindGroupEntries::sequential((
			view_uniform.as_entire_buffer_binding(),
			counters.as_entire_buffer_binding(),
			indirect.as_entire_buffer_binding(),
		)),
	);
	let wood_draw_bgs = [0, 1, 2].map(|lod| {
		device.create_bind_group(
			Some("bush_wood_draw_bg"),
			&pipeline_cache.get_bind_group_layout(&pipelines.wood_draw_layout),
			&BindGroupEntries::sequential((
				view_uniform.as_entire_buffer_binding(),
				plants.as_entire_buffer_binding(),
				branches.as_entire_buffer_binding(),
				wood_lists[lod].as_entire_buffer_binding(),
				wood_params[lod].as_entire_buffer_binding(),
			)),
		)
	});
	let foliage_draw_bgs = [0, 1, 2].map(|lod| {
		device.create_bind_group(
			Some("bush_foliage_draw_bg"),
			&pipeline_cache.get_bind_group_layout(&pipelines.foliage_draw_layout),
			&BindGroupEntries::sequential((
				view_uniform.as_entire_buffer_binding(),
				plants.as_entire_buffer_binding(),
				branches.as_entire_buffer_binding(),
				leaves.as_entire_buffer_binding(),
				foliage_lists[lod].as_entire_buffer_binding(),
				foliage_params[lod].as_entire_buffer_binding(),
			)),
		)
	});

	ViewWork {
		view_uniform,
		counters,
		wood_lists,
		foliage_lists,
		indirect,
		wood_params,
		foliage_params,
		select_wood_bg,
		select_foliage_bg,
		finalize_bg,
		wood_draw_bgs,
		foliage_draw_bgs,
		cpu_wood_counts: [0; BUSH_LOD_COUNT],
		cpu_foliage_counts: [0; BUSH_LOD_COUNT],
	}
}

fn params_buffer(device: &RenderDevice, lod: u32, index_count: u32, vertex_count: u32) -> Buffer {
	let params = GrassDrawParams { lod_bucket: lod, index_count, vertex_count, view_kind: 0 };
	device.create_buffer_with_data(&BufferInitDescriptor {
		label: Some("bush_draw_params"),
		contents: &params.to_std140_bytes(),
		usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
	})
}

fn write_view_uniform(queue: &RenderQueue, buffer: &Buffer, uniform: &GrassViewUniform) {
	queue.write_buffer(buffer, 0, &uniform.to_std140_bytes());
}

fn cpu_fill_lists<'a>(
	queue: &RenderQueue,
	state: &mut BushGpuState,
	extracted: &ExtractedBushField,
	catalog: &BushGroveCatalog,
	views: impl Iterator<Item = &'a ExtractedView>,
	mut stats: Option<&mut BushFieldStats>,
) {
	for view in views {
		if !view_submits_bush(view, extracted.shadow_mode) {
			continue;
		}
		let Some(work) = state.views.get_mut(&view.retained_view_entity) else {
			continue;
		};
		let uniform = make_view_uniform(view, extracted);
		let selector = BushSelector::from_view(uniform, &extracted.settings);
		let lists = selector.compact(&catalog.plants, &catalog.branches, &catalog.leaves);
		for lod in 0..BUSH_LOD_COUNT {
			if !lists.wood[lod].is_empty() {
				let bytes: Vec<u8> = lists.wood[lod].iter().flat_map(|i| i.to_le_bytes()).collect();
				queue.write_buffer(&work.wood_lists[lod], 0, &bytes);
			}
			if !lists.foliage[lod].is_empty() {
				let bytes: Vec<u8> =
					lists.foliage[lod].iter().flat_map(|i| i.to_le_bytes()).collect();
				queue.write_buffer(&work.foliage_lists[lod], 0, &bytes);
			}
			work.cpu_wood_counts[lod] = lists.wood[lod].len() as u32;
			work.cpu_foliage_counts[lod] = lists.foliage[lod].len() as u32;
		}
		if extracted.settings.diagnostics {
			record_lists(extracted, &lists, stats.as_deref_mut());
		}
	}
}

fn record_cpu_stats<'a>(
	extracted: &ExtractedBushField,
	catalog: &BushGroveCatalog,
	views: impl Iterator<Item = &'a ExtractedView>,
	mut stats: Option<&mut BushFieldStats>,
) {
	for view in views {
		if !view_submits_bush(view, extracted.shadow_mode) {
			continue;
		}
		let uniform = make_view_uniform(view, extracted);
		let selector = BushSelector::from_view(uniform, &extracted.settings);
		let lists = selector.compact(&catalog.plants, &catalog.branches, &catalog.leaves);
		record_lists(extracted, &lists, stats.as_deref_mut());
		break;
	}
}

fn record_lists(
	extracted: &ExtractedBushField,
	lists: &crate::select::CompactedBushLists,
	stats: Option<&mut BushFieldStats>,
) {
	if let Some(stats) = stats {
		*stats = BushFieldStats::from_lists(
			extracted.plant_count,
			extracted.branch_count,
			extracted.leaf_count,
			0,
			lists,
		);
	}
}

fn view_submits_bush(view: &ExtractedView, shadow_mode: CanopyShadowMode) -> bool {
	view.retained_view_entity.subview_index == 0 || shadow_mode.canopy_casts()
}

fn bush_compute(
	mut render_context: RenderContext,
	pipelines: Option<Res<BushPipelines>>,
	pipeline_cache: Res<PipelineCache>,
	state: Option<Res<BushGpuState>>,
	capability: Option<Res<GrassGpuCapability>>,
	extracted: Option<Res<ExtractedBushField>>,
	device: Res<RenderDevice>,
	mut encode: Option<ResMut<GroveEncodeStats>>,
) {
	let Some(pipelines) = pipelines else {
		return;
	};
	let Some(state) = state else {
		return;
	};
	let Some(capability) = capability else {
		return;
	};
	let Some(extracted) = extracted else {
		return;
	};
	if capability.mode != GrassGpuMode::GpuDriven || extracted.plant_count == 0 {
		return;
	}
	match pipeline_cache.get_compute_pipeline_state(pipelines.select_wood) {
		CachedPipelineState::Ok(_) => {}
		_ => return,
	};
	let Some(select_wood) = pipeline_cache.get_compute_pipeline(pipelines.select_wood) else {
		return;
	};
	let Some(select_foliage) = pipeline_cache.get_compute_pipeline(pipelines.select_foliage) else {
		return;
	};
	let Some(finalize) = pipeline_cache.get_compute_pipeline(pipelines.finalize) else {
		return;
	};

	let max_dim = device.limits().max_compute_workgroups_per_dimension.max(1);
	let wood = select_dispatch_size(extracted.branch_count, SELECT_WORKGROUP, max_dim);
	let leaf = select_dispatch_size(extracted.leaf_count, SELECT_WORKGROUP, max_dim);
	for work in state.views.values() {
		render_context.command_encoder().clear_buffer(&work.counters, 0, None);
		{
			let mut pass =
				render_context.command_encoder().begin_compute_pass(&ComputePassDescriptor {
					label: Some("bush_select"),
					..default()
				});
			pass.set_bind_group(0, &work.select_wood_bg, &[]);
			pass.set_pipeline(select_wood);
			pass.dispatch_workgroups(wood.x, wood.y, wood.z);
			pass.set_bind_group(0, &work.select_foliage_bg, &[]);
			pass.set_pipeline(select_foliage);
			pass.dispatch_workgroups(leaf.x, leaf.y, leaf.z);
			pass.set_bind_group(0, &work.finalize_bg, &[]);
			pass.set_pipeline(finalize);
			pass.dispatch_workgroups(1, 1, 1);
		}
		if let Some(encode) = encode.as_deref_mut() {
			encode.passes += 1;
			encode.dispatches += 3;
		}
	}
}

fn bush_color_format(view: &ExtractedView, target: Option<&ViewTarget>) -> TextureFormat {
	target.map(ViewTarget::main_texture_format).unwrap_or(view.target_format)
}

fn queue_bush(
	pipeline_cache: Res<PipelineCache>,
	mut pipelines: Option<ResMut<BushPipelines>>,
	mut opaque_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
	opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
	views: Query<(&ExtractedView, &RenderVisibleEntities, &Msaa, Option<&ViewTarget>)>,
	presenters: Query<&BushFieldPresenter>,
	dirty: Res<DirtySpecializations>,
	mut pending: ResMut<PendingBushQueues>,
	mut queued: Local<MainEntityHashSet>,
	mut queued_format: Local<HashMap<RetainedViewEntity, TextureFormat>>,
	extracted: Option<Res<ExtractedBushField>>,
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
	if !capability.submits() || extracted.plant_count == 0 {
		return;
	}
	let draw_wood = opaque_draw_functions.read().id::<DrawBushWood>();
	let draw_foliage = opaque_draw_functions.read().id::<DrawBushFoliage>();
	for (view, visible, msaa, view_target) in &views {
		if !view_submits_bush(view, extracted.shadow_mode) {
			continue;
		}
		let Some(phase) = opaque_phases.get_mut(&view.retained_view_entity) else {
			continue;
		};
		let Some(visible_entities) = visible.get::<BushFieldPresenter>() else {
			continue;
		};
		let format = bush_color_format(view, view_target);
		let hdr = format == TextureFormat::Rgba16Float;
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
		// Offscreen screenshots reuse the retained view with a new color format
		// (HDR `Rgba16Float` vs Sharp/force-offscreen `Bgra8UnormSrgb`). Dirty
		// specializations do not requeue those presenters, so force every
		// visible entity through the new key.
		let force_queue: Vec<_> = if format_changed {
			visible_entities.iter_visible().map(|(render, main)| (*render, *main)).collect()
		} else {
			Vec::new()
		};
		let queue_targets: Vec<(Entity, bevy::render::sync_world::MainEntity)> = if format_changed {
			force_queue
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
			let Ok(presenter) = presenters.get(*render_entity) else {
				continue;
			};
			let key = BushDrawKey { msaa: msaa.samples(), format, hdr };
			let (variants, draw_function, asset_id) = match presenter {
				BushFieldPresenter::Wood => {
					(&mut pipelines.wood_variants, draw_wood, AssetId::<Mesh>::default().untyped())
				}
				BushFieldPresenter::Foliage => (
					&mut pipelines.foliage_variants,
					draw_foliage,
					AssetId::<StandardMaterial>::default().untyped(),
				),
			};
			let Ok(pipeline) = variants.specialize(&pipeline_cache, key) else {
				continue;
			};
			phase.add(
				Opaque3dBatchSetKey {
					draw_function,
					pipeline,
					material_bind_group_index: None,
					lightmap_slab: None,
					slabs: MeshSlabs::default(),
				},
				Opaque3dBinKey { asset_id },
				(*render_entity, *main_entity),
				InputUniformIndex::default(),
				BinnedRenderPhaseType::NonMesh,
			);
			encode.draws += BUSH_LOD_COUNT as u32;
		}
	}
}

type DrawBushWood = (SetItemPipeline, DrawBushWoodCommand);
type DrawBushFoliage = (SetItemPipeline, DrawBushFoliageCommand);

struct DrawBushWoodCommand;
struct DrawBushFoliageCommand;

impl<P: PhaseItem> RenderCommand<P> for DrawBushWoodCommand {
	type Param = (SRes<BushGpuState>, SRes<BushTemplates>, SRes<GrassGpuCapability>);
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
		for lod in 0..BUSH_LOD_COUNT {
			pass.set_bind_group(0, &work.wood_draw_bgs[lod], &[]);
			pass.set_vertex_buffer(0, templates.wood[lod].vertices.slice(..));
			pass.set_index_buffer(templates.wood[lod].indices.slice(..), IndexFormat::Uint32);
			draw_lod(
				pass,
				capability,
				work,
				templates.wood[lod].index_count,
				work.cpu_wood_counts[lod],
				lod,
			);
		}
		RenderCommandResult::Success
	}
}

impl<P: PhaseItem> RenderCommand<P> for DrawBushFoliageCommand {
	type Param = (SRes<BushGpuState>, SRes<BushTemplates>, SRes<GrassGpuCapability>);
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
		for lod in 0..BUSH_LOD_COUNT {
			pass.set_bind_group(0, &work.foliage_draw_bgs[lod], &[]);
			pass.set_vertex_buffer(0, templates.leaf.vertices.slice(..));
			pass.set_index_buffer(templates.leaf.indices.slice(..), IndexFormat::Uint32);
			draw_lod(
				pass,
				capability,
				work,
				templates.leaf.index_count,
				work.cpu_foliage_counts[lod],
				lod + BUSH_LOD_COUNT,
			);
		}
		RenderCommandResult::Success
	}
}

fn draw_lod<'a>(
	pass: &mut TrackedRenderPass<'a>,
	capability: &GrassGpuCapability,
	work: &'a ViewWork,
	index_count: u32,
	cpu_instances: u32,
	indirect_slot: usize,
) {
	match capability.mode {
		GrassGpuMode::GpuDriven => {
			pass.draw_indexed_indirect(
				&work.indirect,
				DRAW_INDEXED_INDIRECT_STRIDE * indirect_slot as u64,
			);
		}
		GrassGpuMode::CpuSelect => {
			if cpu_instances > 0 {
				pass.draw_indexed(0..index_count, 0, 0..cpu_instances);
			}
		}
		GrassGpuMode::Unsupported => {}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use vegetation_grass_field::MAX_COMPUTE_WORKGROUPS_PER_DIM;

	fn specialize_format(format: TextureFormat) -> TextureFormat {
		let mut descriptor = opaque_descriptor("bush_format_test", Handle::default(), Vec::new());
		let key = BushDrawKey { msaa: 1, format, hdr: format == TextureFormat::Rgba16Float };
		BushDrawSpecializer.specialize(key, &mut descriptor).expect("specialize");
		descriptor.fragment.unwrap().targets[0].as_ref().unwrap().format
	}

	#[test]
	fn bush_pipeline_matches_offscreen_and_hdr_formats() {
		assert_eq!(specialize_format(TextureFormat::Rgba16Float), TextureFormat::Rgba16Float);
		assert_eq!(specialize_format(TextureFormat::Bgra8UnormSrgb), TextureFormat::Bgra8UnormSrgb);
		assert_eq!(specialize_format(TextureFormat::Rgba8UnormSrgb), TextureFormat::Rgba8UnormSrgb);
		assert_eq!(
			bush_color_format_preference(
				TextureFormat::Rgba16Float,
				Some(TextureFormat::Bgra8UnormSrgb)
			),
			TextureFormat::Bgra8UnormSrgb
		);
	}

	fn bush_color_format_preference(
		view_format: TextureFormat,
		target_format: Option<TextureFormat>,
	) -> TextureFormat {
		target_format.unwrap_or(view_format)
	}

	#[test]
	fn catalog_select_dispatch_fits_native_bush_leaf_overflow() {
		// Area 1746 Metal smoke: foliage 1D groups were 66744 > 65535.
		let leaf_count = 66_744 * SELECT_WORKGROUP;
		let one_d = leaf_count.div_ceil(SELECT_WORKGROUP);
		assert!(one_d > MAX_COMPUTE_WORKGROUPS_PER_DIM);
		let size =
			select_dispatch_size(leaf_count, SELECT_WORKGROUP, MAX_COMPUTE_WORKGROUPS_PER_DIM);
		assert!(size.x <= MAX_COMPUTE_WORKGROUPS_PER_DIM);
		assert!(size.y <= MAX_COMPUTE_WORKGROUPS_PER_DIM);
		assert_eq!(size.z, 1);
		assert!(
			u64::from(size.x) * u64::from(size.y) * u64::from(SELECT_WORKGROUP)
				>= u64::from(leaf_count)
		);
	}
}
