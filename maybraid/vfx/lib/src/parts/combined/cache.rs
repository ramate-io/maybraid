//! GPU composition of the current live state. This is never a recorded animation.
use bevy::{
	asset::{embedded_asset, RenderAssetUsages},
	core_pipeline::schedule::camera_driver,
	prelude::*,
	render::{
		extract_resource::{ExtractResource, ExtractResourcePlugin},
		render_asset::RenderAssets,
		render_resource::{binding_types::*, *},
		renderer::{RenderContext, RenderDevice, RenderGraph},
		storage::{GpuShaderBuffer, ShaderBuffer},
		texture::GpuImage,
		Render, RenderApp, RenderStartup, RenderSystems,
	},
};
pub const MAX_SIZE: UVec3 = UVec3::new(192, 96, 160);
pub const LOW_SIZE: UVec3 = UVec3::new(128, 64, 96);
pub const BYTES: u64 = 192 * 96 * 160 * 8;
#[derive(Clone)]
pub struct Input {
	pub textures: [Handle<Image>; 7],
	pub field: Handle<Image>,
	pub payload: Handle<ShaderBuffer>,
	pub generation: u64,
	pub dimensions: UVec3,
}
#[derive(Resource, Default, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct CacheInput(pub Option<Input>, pub Vec<Input>);
#[derive(Resource)]
struct Pipeline {
	layout: BindGroupLayoutDescriptor,
	id: CachedComputePipelineId,
}
#[derive(Default)]
struct Work {
	group: Option<BindGroup>,
	queued: u64,
	completed: u64,
	field: Option<AssetId<Image>>,
	dimensions: UVec3,
}
#[derive(Resource, Default)]
struct Works(Vec<Work>);
pub fn plugin(app: &mut App) {
	embedded_asset!(app, "compose.wesl");
	app.init_resource::<CacheInput>()
		.add_plugins(ExtractResourcePlugin::<CacheInput>::default());
	let Some(render) = app.get_sub_app_mut(RenderApp) else {
		return;
	};
	render
		.init_resource::<Works>()
		.add_systems(RenderStartup, initialize)
		.add_systems(Render, prepare.in_set(RenderSystems::PrepareBindGroups))
		.add_systems(RenderGraph, compose.before(camera_driver));
}
pub fn image() -> Image {
	let mut image = Image::new_uninit(
		Extent3d { width: MAX_SIZE.x, height: MAX_SIZE.y, depth_or_array_layers: MAX_SIZE.z },
		TextureDimension::D3,
		TextureFormat::Rgba16Float,
		RenderAssetUsages::RENDER_WORLD,
	);
	image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
		| TextureUsages::STORAGE_BINDING
		| TextureUsages::COPY_DST
		| TextureUsages::COPY_SRC;
	image
}
fn initialize(mut commands: Commands, server: Res<AssetServer>, cache: Res<PipelineCache>) {
	let tex = texture_3d(TextureSampleType::Float { filterable: true });
	let layout = BindGroupLayoutDescriptor::new(
		"Combined live density composition",
		&BindGroupLayoutEntries::sequential(
			ShaderStages::COMPUTE,
			(
				tex,
				tex,
				tex,
				tex,
				tex,
				tex,
				tex,
				sampler(SamplerBindingType::Filtering),
				storage_buffer_read_only::<Vec<Vec4>>(false),
				texture_storage_3d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
			),
		),
	);
	let id = cache.queue_compute_pipeline(ComputePipelineDescriptor {
		label: Some("Combined rest-density composition".into()),
		layout: vec![layout.clone()],
		shader: server.load(concat!(
			"embedded://",
			env!("CARGO_CRATE_NAME"),
			"/parts/combined/compose.wesl"
		)),
		entry_point: Some("compose".into()),
		..default()
	});
	commands.insert_resource(Pipeline { layout, id });
}
fn prepare(
	input: Res<CacheInput>,
	mut works: ResMut<Works>,
	pipeline: Res<Pipeline>,
	cache: Res<PipelineCache>,
	images: Res<RenderAssets<GpuImage>>,
	buffers: Res<RenderAssets<GpuShaderBuffer>>,
	device: Res<RenderDevice>,
) {
	let inputs = input.0.iter().chain(input.1.iter()).collect::<Vec<_>>();
	works.0.resize_with(inputs.len(), Work::default);
	for (work, input) in works.0.iter_mut().zip(inputs) {
		if work.queued == input.generation
			&& work.field == Some(input.field.id())
			&& work.group.is_some()
		{
			continue;
		}
		let Some(source) = input.textures.iter().map(|h| images.get(h)).collect::<Option<Vec<_>>>()
		else {
			continue;
		};
		let Some(field) = images.get(&input.field) else {
			continue;
		};
		let Some(buffer) = buffers.get(&input.payload) else {
			continue;
		};
		let group = device.create_bind_group(
			Some("Combined density inputs"),
			&cache.get_bind_group_layout(&pipeline.layout),
			&BindGroupEntries::sequential((
				&source[0].texture_view,
				&source[1].texture_view,
				&source[2].texture_view,
				&source[3].texture_view,
				&source[4].texture_view,
				&source[5].texture_view,
				&source[6].texture_view,
				&source[0].sampler,
				buffer.buffer.as_entire_buffer_binding(),
				&field.texture_view,
			)),
		);
		if work.field != Some(input.field.id()) {
			work.completed = u64::MAX;
		}
		work.field = Some(input.field.id());
		work.group = Some(group);
		work.queued = input.generation;
		work.dimensions = input.dimensions;
	}
}
fn compose(
	mut context: RenderContext,
	mut works: ResMut<Works>,
	pipeline: Res<Pipeline>,
	cache: Res<PipelineCache>,
) {
	for work in &mut works.0 {
		if work.queued == work.completed {
			continue;
		}
		let Some(group) = &work.group else {
			continue;
		};
		let Some(pipeline) = cache.get_compute_pipeline(pipeline.id) else {
			continue;
		};
		let mut pass = context.command_encoder().begin_compute_pass(&ComputePassDescriptor {
			label: Some("Combined live composed field"),
			..default()
		});
		pass.set_pipeline(pipeline);
		pass.set_bind_group(0, group, &[]);
		let n = work.dimensions;
		pass.dispatch_workgroups(n.x.div_ceil(4), n.y.div_ceil(4), n.z.div_ceil(4));
		drop(pass);
		work.completed = work.queued;
	}
}
