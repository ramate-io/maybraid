//! Extract / upload / custom Core3d draw for packed orchard batches.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use bevy::asset::embedded_asset;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::core_3d::main_opaque_pass_3d;
use bevy::core_pipeline::core_3d::main_transparent_pass_3d;
use bevy::core_pipeline::{Core3d, Core3dSystems};
use bevy::log::info_span;
use bevy::mesh::VertexBufferLayout;
use bevy::prelude::*;
use bevy::render::mesh::allocator::MeshAllocator;
use bevy::render::mesh::RenderMesh;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::encase::{internal::WriteInto, ShaderSize, StorageBuffer, UniformBuffer};
use bevy::render::render_resource::{
	BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
	BindingType, Buffer, BufferBindingType, BufferInitDescriptor, BufferUsages, ColorTargetState,
	ColorWrites, CompareFunction, DepthStencilState, Face, FragmentState, FrontFace, IndexFormat,
	MultisampleState, PipelineCache, PolygonMode, PrimitiveState, PrimitiveTopology,
	RenderPipelineDescriptor, ShaderStages, ShaderType, SpecializedRenderPipeline,
	SpecializedRenderPipelines, TextureFormat, VertexState, VertexStepMode,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::view::{ExtractedView, ViewDepthTexture, ViewTarget};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems};
use lod::gen::Id;
use lod::{LodSceneLevel, VisualMaterialKind};

use crate::packed::cache::{attach_packed_orchard_physics, PackedGroveCache};
use crate::packed::kit::{resolve_kit_meshes, PackedKitMeshes};
use crate::packed::mode::PackMode;
use crate::packed::select::{
	evict_packed_tiles, select_packed_tiles, sync_packed_tiles, PackedGroveSelection, SelectedTile,
};

const SHADER_PATH: &str = concat!("embedded://", env!("CARGO_CRATE_NAME"), "/packed_grove.wgsl");

/// CPU packed cache + Core3d instanced draw. No-ops when the env switch is off.
pub struct PackedGrovePlugin;

impl Plugin for PackedGrovePlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "packed_grove.wgsl");
		app.insert_resource(PackMode::current())
			.init_resource::<PackedGroveCache>()
			.init_resource::<PackedGroveSelection>()
			.init_resource::<PackedKitMeshes>()
			.init_resource::<PackedGroveMetrics>()
			.add_observer(attach_packed_orchard_physics);
		if PackMode::current().is_off() {
			return;
		}
		app.add_systems(
			Update,
			(
				sync_packed_tiles,
				select_packed_tiles.after(sync_packed_tiles),
				resolve_kit_meshes.after(select_packed_tiles),
				evict_packed_tiles.after(select_packed_tiles),
				log_packed_metrics.after(select_packed_tiles),
			),
		);
		let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
			return;
		};
		render_app
			.init_resource::<ExtractedPackedGroves>()
			.init_resource::<PackedGroveGpu>()
			.init_resource::<SpecializedRenderPipelines<PackedGrovePipeline>>()
			.add_systems(RenderStartup, init_packed_pipeline)
			.add_systems(ExtractSchedule, extract_packed_view)
			.add_systems(Render, prepare_packed_gpu.in_set(RenderSystems::PrepareBindGroups))
			.add_systems(
				Core3d,
				packed_grove_pass.after(main_opaque_pass_3d).before(main_transparent_pass_3d),
			);
		let _ = Core3dSystems::MainPass;
		let _ = RenderLayers::default();
	}
}

#[derive(Resource, Default, Debug, Clone)]
pub struct PackedGroveMetrics {
	pub last_log: f32,
}

fn log_packed_metrics(
	time: Res<Time>,
	mut metrics: ResMut<PackedGroveMetrics>,
	selection: Res<PackedGroveSelection>,
	cache: Res<PackedGroveCache>,
	kits: Res<PackedKitMeshes>,
) {
	if !PackMode::metrics_from_env() {
		return;
	}
	if time.elapsed_secs() - metrics.last_log < 0.5 {
		return;
	}
	metrics.last_log = time.elapsed_secs();
	info!(
		target: "packed_grove",
		tiles = selection.tiles.len(),
		draws = selection.draws,
		instances = selection.instances,
		retained_bytes = cache.retained_bytes(),
		cached_tiles = cache.len(),
		kit_meshes = kits.ready_count(),
		"packed grove frame"
	);
}

#[derive(Clone, Debug)]
struct ExtractedBatch {
	id: Id,
	level: LodSceneLevel,
	kit: &'static str,
	material_hash: u64,
	material_kind: VisualMaterialKind,
	base_color: Vec4,
	mesh: AssetId<Mesh>,
	instances: Vec<GpuInstance>,
	upload: bool,
}

#[derive(Clone, Copy, Debug, ShaderType)]
struct GpuInstance {
	world_from_local: Mat4,
	plant_origin: Vec4,
}

#[derive(Clone, Copy, Debug, ShaderType, Default)]
struct GpuView {
	clip_from_world: Mat4,
	camera_position: Vec4,
	sun_direction: Vec4,
	time_day: Vec4,
}

#[derive(Clone, Copy, Debug, ShaderType)]
struct GpuMaterial {
	base_color: Vec4,
	kind_pad: Vec4,
}

#[derive(Resource, Clone, Default)]
struct ExtractedPackedGroves {
	batches: Vec<ExtractedBatch>,
	view: GpuView,
}

fn extract_packed_view(
	mut extracted: ResMut<ExtractedPackedGroves>,
	selection: Extract<Res<PackedGroveSelection>>,
	kits: Extract<Res<PackedKitMeshes>>,
	cameras: Extract<Query<(&GlobalTransform, &Camera), With<Camera3d>>>,
	lights: Extract<Query<&GlobalTransform, With<DirectionalLight>>>,
	time: Extract<Res<Time>>,
) {
	extracted.batches.clear();
	let Some((xf, camera)) = cameras.iter().next() else {
		return;
	};
	let view_from_world = xf.to_matrix().inverse();
	let sun = lights
		.iter()
		.next()
		.map(|light| -*light.forward())
		.unwrap_or(Vec3::new(0.35, 0.85, 0.35).normalize());
	extracted.view = GpuView {
		clip_from_world: camera.clip_from_view() * view_from_world,
		camera_position: xf.translation().extend(1.0),
		sun_direction: sun.extend(0.0),
		time_day: Vec4::new(time.elapsed_secs(), 1.0, 0.0, 0.0),
	};
	for tile in &selection.tiles {
		push_extracted_tile(&mut extracted.batches, tile, &kits);
	}
}

fn push_extracted_tile(
	out: &mut Vec<ExtractedBatch>,
	tile: &SelectedTile,
	kits: &PackedKitMeshes,
) {
	for batch in &tile.batches {
		let Some(mesh) = kits.mesh(batch.key.kit) else {
			continue;
		};
		out.push(ExtractedBatch {
			id: tile.id,
			level: tile.level,
			kit: batch.key.kit,
			material_hash: batch.key.material_hash,
			material_kind: batch.key.material_kind,
			base_color: batch.base_color,
			mesh: mesh.id(),
			instances: batch
				.instances
				.iter()
				.map(|instance| GpuInstance {
					world_from_local: instance.world_from_local,
					plant_origin: instance.plant_origin.extend(1.0),
				})
				.collect(),
			upload: tile.upload,
		});
	}
}

#[derive(Resource)]
struct PackedGrovePipeline {
	shader: Handle<Shader>,
	view_layout: BindGroupLayout,
	instance_layout: BindGroupLayout,
	material_layout: BindGroupLayout,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct PackedGrovePipelineKey {
	msaa: u32,
	format: TextureFormat,
}

impl PackedGrovePipeline {
	fn view_entries() -> [BindGroupLayoutEntry; 1] {
		[BindGroupLayoutEntry {
			binding: 0,
			visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
			ty: BindingType::Buffer {
				ty: BufferBindingType::Uniform,
				has_dynamic_offset: false,
				min_binding_size: Some(GpuView::min_size()),
			},
			count: None,
		}]
	}

	fn instance_entries() -> [BindGroupLayoutEntry; 1] {
		[BindGroupLayoutEntry {
			binding: 0,
			visibility: ShaderStages::VERTEX,
			ty: BindingType::Buffer {
				ty: BufferBindingType::Storage { read_only: true },
				has_dynamic_offset: false,
				min_binding_size: Some(GpuInstance::min_size()),
			},
			count: None,
		}]
	}

	fn material_entries() -> [BindGroupLayoutEntry; 1] {
		[BindGroupLayoutEntry {
			binding: 0,
			visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
			ty: BindingType::Buffer {
				ty: BufferBindingType::Uniform,
				has_dynamic_offset: false,
				min_binding_size: Some(GpuMaterial::min_size()),
			},
			count: None,
		}]
	}
}

impl FromWorld for PackedGrovePipeline {
	fn from_world(world: &mut World) -> Self {
		let device = world.resource::<RenderDevice>();
		let view_layout = device.create_bind_group_layout(Some("packed_grove_view"), &Self::view_entries());
		let instance_layout =
			device.create_bind_group_layout(Some("packed_grove_instances"), &Self::instance_entries());
		let material_layout =
			device.create_bind_group_layout(Some("packed_grove_material"), &Self::material_entries());
		let shader = world.resource::<AssetServer>().load(SHADER_PATH);
		Self { shader, view_layout, instance_layout, material_layout }
	}
}

fn init_packed_pipeline(mut commands: Commands) {
	commands.init_resource::<PackedGrovePipeline>();
}

impl SpecializedRenderPipeline for PackedGrovePipeline {
	type Key = PackedGrovePipelineKey;

	fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
		RenderPipelineDescriptor {
			label: Some("packed_grove".into()),
			layout: vec![
				BindGroupLayoutDescriptor::new("packed_grove_view", &Self::view_entries()),
				BindGroupLayoutDescriptor::new("packed_grove_instances", &Self::instance_entries()),
				BindGroupLayoutDescriptor::new("packed_grove_material", &Self::material_entries()),
			],
			immediate_size: 0,
			vertex: VertexState {
				shader: self.shader.clone(),
				shader_defs: Vec::new(),
				entry_point: Some("vertex".into()),
				buffers: vec![VertexBufferLayout::from_vertex_formats(
					VertexStepMode::Vertex,
					[bevy::mesh::VertexFormat::Float32x3, bevy::mesh::VertexFormat::Float32x3],
				)],
			},
			fragment: Some(FragmentState {
				shader: self.shader.clone(),
				shader_defs: Vec::new(),
				entry_point: Some("fragment".into()),
				targets: vec![Some(ColorTargetState {
					format: key.format,
					blend: None,
					write_mask: ColorWrites::ALL,
				})],
			}),
			primitive: PrimitiveState {
				topology: PrimitiveTopology::TriangleList,
				strip_index_format: None,
				front_face: FrontFace::Ccw,
				cull_mode: None::<Face>,
				unclipped_depth: false,
				polygon_mode: PolygonMode::Fill,
				conservative: false,
			},
			depth_stencil: Some(DepthStencilState {
				format: TextureFormat::Depth32Float,
				depth_write_enabled: Some(true),
				depth_compare: Some(CompareFunction::GreaterEqual),
				stencil: default(),
				bias: default(),
			}),
			multisample: MultisampleState {
				count: key.msaa.max(1),
				mask: !0,
				alpha_to_coverage_enabled: false,
			},
			zero_initialize_workgroup_memory: false,
		}
	}
}

struct GpuBatch {
	/// Owned so the bind group stays valid after upload.
	_instance_buffer: Buffer,
	instance_bind: BindGroup,
	material_bind: BindGroup,
	count: u32,
	mesh: AssetId<Mesh>,
	revision: u64,
}

#[derive(Resource, Default)]
struct PackedGroveGpu {
	view_buffer: Option<Buffer>,
	view_bind: Option<BindGroup>,
	batches: HashMap<u64, GpuBatch>,
	uploaded_bytes: u64,
}

fn batch_key(batch: &ExtractedBatch) -> u64 {
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	batch.id.hash(&mut hasher);
	batch.level.hash(&mut hasher);
	batch.kit.hash(&mut hasher);
	batch.material_hash.hash(&mut hasher);
	hasher.finish()
}

fn prepare_packed_gpu(
	extracted: Res<ExtractedPackedGroves>,
	pipeline: Option<Res<PackedGrovePipeline>>,
	device: Res<RenderDevice>,
	queue: Res<RenderQueue>,
	mut gpu: ResMut<PackedGroveGpu>,
) {
	let Some(pipeline) = pipeline else {
		return;
	};
	let _span = info_span!("packed_grove_upload").entered();
	gpu.uploaded_bytes = 0;
	let view_bytes = uniform_bytes(&extracted.view);
	let view_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
		label: Some("packed_grove_view"),
		contents: &view_bytes,
		usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
	});
	gpu.view_bind = Some(device.create_bind_group(
		Some("packed_grove_view_bg"),
		&pipeline.view_layout,
		&[BindGroupEntry { binding: 0, resource: view_buffer.as_entire_binding() }],
	));
	gpu.view_buffer = Some(view_buffer);

	let live: std::collections::HashSet<u64> = extracted.batches.iter().map(batch_key).collect();
	gpu.batches.retain(|key, _| live.contains(key));

	for batch in &extracted.batches {
		if batch.instances.is_empty() {
			continue;
		}
		let key = batch_key(batch);
		let revision = hash_instances(&batch.instances);
		if let Some(existing) = gpu.batches.get(&key) {
			if existing.revision == revision && !batch.upload {
				continue;
			}
		}
		let instance_bytes = storage_bytes(&batch.instances);
		let instance_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
			label: Some("packed_grove_instances"),
			contents: &instance_bytes,
			usage: BufferUsages::STORAGE,
		});
		let instance_bind = device.create_bind_group(
			Some("packed_grove_instances_bg"),
			&pipeline.instance_layout,
			&[BindGroupEntry { binding: 0, resource: instance_buffer.as_entire_binding() }],
		);
		let kind = match batch.material_kind {
			VisualMaterialKind::Leaf => 0.0,
			VisualMaterialKind::Stick => 1.0,
			VisualMaterialKind::Frond => 2.0,
		};
		let material = GpuMaterial { base_color: batch.base_color, kind_pad: Vec4::new(kind, 0.0, 0.0, 0.0) };
		let material_bytes = uniform_bytes(&material);
		let material_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
			label: Some("packed_grove_material"),
			contents: &material_bytes,
			usage: BufferUsages::UNIFORM,
		});
		let material_bind = device.create_bind_group(
			Some("packed_grove_material_bg"),
			&pipeline.material_layout,
			&[BindGroupEntry { binding: 0, resource: material_buffer.as_entire_binding() }],
		);
		gpu.uploaded_bytes += instance_bytes.len() as u64;
		gpu.batches.insert(
			key,
			GpuBatch {
				_instance_buffer: instance_buffer,
				instance_bind,
				material_bind,
				count: batch.instances.len() as u32,
				mesh: batch.mesh,
				revision,
			},
		);
		let _ = queue;
	}
}

fn packed_grove_pass(
	extracted: Res<ExtractedPackedGroves>,
	gpu: Res<PackedGroveGpu>,
	pipeline: Option<Res<PackedGrovePipeline>>,
	mut pipelines: ResMut<SpecializedRenderPipelines<PackedGrovePipeline>>,
	cache: Res<PipelineCache>,
	meshes: Res<RenderAssets<RenderMesh>>,
	allocator: Res<MeshAllocator>,
	view: ViewQuery<(&ExtractedView, &ViewTarget, &ViewDepthTexture, &Msaa)>,
	mut ctx: RenderContext,
) {
	let Some(pipeline_res) = pipeline else {
		return;
	};
	let Some(view_bind) = gpu.view_bind.as_ref() else {
		return;
	};
	if extracted.batches.is_empty() {
		return;
	}
	let (extracted_view, target, depth, msaa) = view.into_inner();
	let pipeline_id = pipelines.specialize(
		&cache,
		pipeline_res.as_ref(),
		PackedGrovePipelineKey { msaa: msaa.samples(), format: extracted_view.target_format },
	);
	let Some(hw_pipeline) = cache.get_render_pipeline(pipeline_id) else {
		return;
	};
	let color_attachments = [Some(target.get_color_attachment())];
	let depth_stencil_attachment = Some(depth.get_attachment(bevy::render::render_resource::StoreOp::Store));
	let mut pass = ctx.begin_tracked_render_pass(bevy::render::render_resource::RenderPassDescriptor {
		label: Some("packed_grove_pass"),
		color_attachments: &color_attachments,
		depth_stencil_attachment,
		timestamp_writes: None,
		occlusion_query_set: None,
		multiview_mask: None,
	});
	pass.set_render_pipeline(hw_pipeline);
	pass.set_bind_group(0, view_bind, &[]);
	for batch in &extracted.batches {
		let Some(gpu_batch) = gpu.batches.get(&batch_key(batch)) else {
			continue;
		};
		let Some(render_mesh) = meshes.get(gpu_batch.mesh) else {
			continue;
		};
		let Some(vertex) = allocator.mesh_vertex_slice(&gpu_batch.mesh) else {
			continue;
		};
		pass.set_bind_group(1, &gpu_batch.instance_bind, &[]);
		pass.set_bind_group(2, &gpu_batch.material_bind, &[]);
		pass.set_vertex_buffer(0, vertex.buffer.slice(..));
		if let Some(index) = allocator.mesh_index_slice(&gpu_batch.mesh) {
			let format = render_mesh.index_format().unwrap_or(IndexFormat::Uint32);
			pass.set_index_buffer(index.buffer.slice(..), format);
			pass.draw_indexed(index.range.clone(), vertex.range.start as i32, 0..gpu_batch.count);
		} else {
			pass.draw(vertex.range.clone(), 0..gpu_batch.count);
		}
	}
}

fn uniform_bytes<T: ShaderType + ShaderSize + WriteInto>(value: &T) -> Vec<u8> {
	let mut buffer = UniformBuffer::new(Vec::new());
	let _ = buffer.write(value);
	buffer.into_inner()
}

fn storage_bytes<T: ShaderType + ShaderSize + WriteInto>(values: &[T]) -> Vec<u8> {
	let mut buffer = StorageBuffer::new(Vec::new());
	let _ = buffer.write(values);
	buffer.into_inner()
}

fn hash_instances(instances: &[GpuInstance]) -> u64 {
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	instances.len().hash(&mut hasher);
	if let Some(first) = instances.first() {
		first.world_from_local.x_axis.x.to_bits().hash(&mut hasher);
		first.plant_origin.x.to_bits().hash(&mut hasher);
	}
	if let Some(last) = instances.last() {
		last.world_from_local.w_axis.x.to_bits().hash(&mut hasher);
	}
	hasher.finish()
}
