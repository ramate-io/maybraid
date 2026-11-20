//! Two ordered compute passes: rest-density composition, then upstream-key lighting.
use bevy::render::diagnostic::RecordDiagnostics;
use bevy::{
	asset::RenderAssetUsages,
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
#[derive(Resource, Default, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GpuReadiness(pub std::sync::Arc<std::sync::Mutex<std::collections::HashSet<Entity>>>);
impl GpuReadiness {
	pub fn contains(&self, id: Entity) -> bool {
		self.0.lock().is_ok_and(|s| s.contains(&id))
	}
}
#[derive(Clone)]
pub struct Input {
	pub id: Entity,
	pub textures: [Handle<Image>; 16],
	pub density: Handle<Image>,
	pub light: Handle<Image>,
	pub payload: Handle<ShaderBuffer>,
	pub generation: u64,
	pub dimensions: UVec3,
}
#[derive(Resource, Default, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct Inputs(pub Vec<Input>);
#[derive(Resource)]
struct Pipelines {
	compose_layout: BindGroupLayoutDescriptor,
	light_layout: BindGroupLayoutDescriptor,
	compose: CachedComputePipelineId,
	light: CachedComputePipelineId,
}
struct Job {
	compose: BindGroup,
	light: BindGroup,
	generation: u64,
	completed: u64,
	dimensions: UVec3,
}
#[derive(Resource, Default)]
struct Jobs(std::collections::HashMap<Entity, Job>);
pub fn image(format: TextureFormat) -> Image {
	let mut im = Image::new_uninit(
		Extent3d { width: 256, height: 256, depth_or_array_layers: 256 },
		TextureDimension::D3,
		format,
		RenderAssetUsages::RENDER_WORLD,
	);
	im.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
		| TextureUsages::STORAGE_BINDING
		| TextureUsages::COPY_SRC
		| TextureUsages::COPY_DST;
	im
}
pub fn plugin(app: &mut App) {
	app.init_resource::<GpuReadiness>()
		.add_plugins(ExtractResourcePlugin::<GpuReadiness>::default())
		.init_resource::<Inputs>()
		.add_plugins(ExtractResourcePlugin::<Inputs>::default());
	if let Some(r) = app.get_sub_app_mut(RenderApp) {
		r.init_resource::<Jobs>()
			.add_systems(RenderStartup, initialize)
			.add_systems(Render, prepare.in_set(RenderSystems::PrepareBindGroups))
			.add_systems(RenderGraph, dispatch.before(camera_driver));
	}
}
fn initialize(
	mut c: Commands,
	s: Res<AssetServer>,
	cache: Res<PipelineCache>,
	device: Res<RenderDevice>,
) {
	let limits = device.limits();
	assert!(
		limits.max_sampled_textures_per_shader_stage >= 16,
		"CombinedExplosion requires sixteen sampled compute textures"
	);
	assert!(
		limits.max_storage_textures_per_shader_stage >= 1,
		"CombinedExplosion requires a storage texture"
	);
	let tex = texture_3d(TextureSampleType::Float { filterable: true });
	let compose_layout = BindGroupLayoutDescriptor::new(
		"Explosion rest density inputs",
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
				tex,
				tex,
				tex,
				tex,
				tex,
				tex,
				tex,
				tex,
				tex,
				sampler(SamplerBindingType::Filtering),
				storage_buffer_read_only::<Vec<Vec4>>(false),
				texture_storage_3d(TextureFormat::R16Float, StorageTextureAccess::WriteOnly),
			),
		),
	);
	let light_layout = BindGroupLayoutDescriptor::new(
		"Explosion upstream density lighting",
		&BindGroupLayoutEntries::sequential(
			ShaderStages::COMPUTE,
			(
				tex,
				sampler(SamplerBindingType::Filtering),
				storage_buffer_read_only::<Vec<Vec4>>(false),
				texture_storage_3d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
			),
		),
	);
	let shader = |name: &str| {
		s.load(format!("embedded://{}/parts/smoke_explosion/{name}.wesl", env!("CARGO_CRATE_NAME")))
	};
	let compose = cache.queue_compute_pipeline(ComputePipelineDescriptor {
		label: Some("Explosion exact appearance composition".into()),
		layout: vec![compose_layout.clone()],
		shader: shader("compose"),
		entry_point: Some("compose".into()),
		..default()
	});
	let light = cache.queue_compute_pipeline(ComputePipelineDescriptor {
		label: Some("Explosion upstream key integration".into()),
		layout: vec![light_layout.clone()],
		shader: shader("light"),
		entry_point: Some("light".into()),
		..default()
	});
	c.insert_resource(Pipelines { compose_layout, light_layout, compose, light });
}
fn prepare(
	input: Res<Inputs>,
	mut jobs: ResMut<Jobs>,
	pipelines: Res<Pipelines>,
	cache: Res<PipelineCache>,
	images: Res<RenderAssets<GpuImage>>,
	buffers: Res<RenderAssets<GpuShaderBuffer>>,
	device: Res<RenderDevice>,
) {
	jobs.0.retain(|id, _| input.0.iter().any(|i| i.id == *id));
	for i in &input.0 {
		if jobs.0.get(&i.id).is_some_and(|j| j.generation == i.generation) {
			continue;
		}
		let Some(source) = i.textures.iter().map(|h| images.get(h)).collect::<Option<Vec<_>>>()
		else {
			continue;
		};
		let (Some(d), Some(l), Some(b)) =
			(images.get(&i.density), images.get(&i.light), buffers.get(&i.payload))
		else {
			continue;
		};
		let compose = device.create_bind_group(
			Some("Explosion density"),
			&cache.get_bind_group_layout(&pipelines.compose_layout),
			&BindGroupEntries::sequential((
				&source[0].texture_view,
				&source[1].texture_view,
				&source[2].texture_view,
				&source[3].texture_view,
				&source[4].texture_view,
				&source[5].texture_view,
				&source[6].texture_view,
				&source[7].texture_view,
				&source[8].texture_view,
				&source[9].texture_view,
				&source[10].texture_view,
				&source[11].texture_view,
				&source[12].texture_view,
				&source[13].texture_view,
				&source[14].texture_view,
				&source[15].texture_view,
				&source[0].sampler,
				b.buffer.as_entire_buffer_binding(),
				&d.texture_view,
			)),
		);
		let light = device.create_bind_group(
			Some("Explosion lighting"),
			&cache.get_bind_group_layout(&pipelines.light_layout),
			&BindGroupEntries::sequential((
				&d.texture_view,
				&source[0].sampler,
				b.buffer.as_entire_buffer_binding(),
				&l.texture_view,
			)),
		);
		jobs.0.insert(
			i.id,
			Job {
				compose,
				light,
				generation: i.generation,
				completed: 0,
				dimensions: i.dimensions,
			},
		);
	}
}
fn dispatch(
	mut context: RenderContext,
	mut jobs: ResMut<Jobs>,
	pipelines: Res<Pipelines>,
	cache: Res<PipelineCache>,
	ready: Res<GpuReadiness>,
) {
	let (Some(compose), Some(light)) = (
		cache.get_compute_pipeline(pipelines.compose),
		cache.get_compute_pipeline(pipelines.light),
	) else {
		return;
	};
	let diagnostics = context.diagnostic_recorder();
	if let Ok(mut set) = ready.0.lock() {
		set.retain(|id| jobs.0.contains_key(id));
	}
	for (id, j) in jobs.0.iter_mut() {
		if j.generation == j.completed {
			continue;
		}
		{
			let span = diagnostics
				.as_ref()
				.map(|d| d.time_span(context.command_encoder(), "explosion_density"));
			let mut pass = context.command_encoder().begin_compute_pass(&ComputePassDescriptor {
				label: Some("Explosion density"),
				..default()
			});
			pass.set_pipeline(compose);
			pass.set_bind_group(0, &j.compose, &[]);
			pass.dispatch_workgroups(
				j.dimensions.x.div_ceil(4),
				j.dimensions.y.div_ceil(4),
				j.dimensions.z.div_ceil(4),
			);
			drop(pass);
			if let Some(span) = span {
				span.end(context.command_encoder());
			}
		}
		{
			let span = diagnostics
				.as_ref()
				.map(|d| d.time_span(context.command_encoder(), "explosion_light"));
			let mut pass = context.command_encoder().begin_compute_pass(&ComputePassDescriptor {
				label: Some("Explosion upstream lighting"),
				..default()
			});
			pass.set_pipeline(light);
			pass.set_bind_group(0, &j.light, &[]);
			pass.dispatch_workgroups(
				j.dimensions.x.div_ceil(4),
				j.dimensions.y.div_ceil(4),
				j.dimensions.z.div_ceil(4),
			);
			drop(pass);
			if let Some(span) = span {
				span.end(context.command_encoder());
			}
		}
		j.completed = j.generation;
		if let Ok(mut set) = ready.0.lock() {
			set.insert(*id);
		}
	}
}
