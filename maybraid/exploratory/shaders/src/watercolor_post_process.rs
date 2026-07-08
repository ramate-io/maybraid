//! Screen-space watercolor blur post-process for cameras carrying [`WatercolorPostProcess`].

use bevy::{
	asset::embedded_asset,
	camera::{Camera, Camera3d},
	core_pipeline::{schedule::Core3d, Core3dSystems, FullscreenShader},
	prelude::*,
	render::{
		extract_component::{
			ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
			UniformComponentPlugin,
		},
		render_resource::{
			binding_types::{
				sampler, texture_2d, texture_depth_2d, texture_depth_2d_multisampled,
				uniform_buffer,
			},
			*,
		},
		renderer::{RenderContext, RenderDevice, ViewQuery},
		sync_component::SyncComponent,
		view::{prepare_view_targets, Msaa, ViewDepthTexture, ViewTarget},
		Render, RenderApp, RenderStartup, RenderSystems,
	},
};

/// Edge-aware screen-space blur that softens low-poly facets without muddying silhouettes.
///
/// Attach to a [`Camera3d`] alongside the camera bundle:
///
/// ```ignore
/// commands.spawn((
///     Camera3d::default(),
///     WatercolorPostProcess::default(),
/// ));
/// ```
#[derive(Component, Clone, Copy, Debug, Reflect, ShaderType)]
#[reflect(Component, Default)]
pub struct WatercolorPostProcess {
	/// Blend between the sharp scene color and the blurred result (`0.2`–`0.45` is a good range).
	pub blur_amount: f32,
	/// Pixel-radius multiplier for the 3×3 kernel (`1.0`–`2.0` is a good range).
	pub blur_radius: f32,
	/// Edge preservation strength when [`Self::edge_aware`] is enabled.
	pub depth_edge_sharpness: f32,
	/// `1.0` = depth-aware bilateral blur; `0.0` = uniform blur.
	pub edge_aware: f32,
}

impl Default for WatercolorPostProcess {
	fn default() -> Self {
		Self { blur_amount: 0.4, blur_radius: 10.5, depth_edge_sharpness: 0.2, edge_aware: 0.1 }
	}
}

impl WatercolorPostProcess {
	pub fn new(blur_amount: f32, blur_radius: f32) -> Self {
		Self { blur_amount, blur_radius, depth_edge_sharpness: 80.0, edge_aware: 1.0, ..default() }
	}

	#[inline]
	pub fn with_blur_amount(mut self, blur_amount: f32) -> Self {
		self.blur_amount = blur_amount;
		self
	}

	#[inline]
	pub fn with_blur_radius(mut self, blur_radius: f32) -> Self {
		self.blur_radius = blur_radius;
		self
	}

	#[inline]
	pub fn with_depth_edge_sharpness(mut self, depth_edge_sharpness: f32) -> Self {
		self.depth_edge_sharpness = depth_edge_sharpness;
		self
	}

	#[inline]
	pub fn with_edge_aware(mut self, edge_aware: bool) -> Self {
		self.edge_aware = if edge_aware { 1.0 } else { 0.0 };
		self
	}
}

impl SyncComponent for WatercolorPostProcess {
	type Target = Self;
}

impl ExtractComponent for WatercolorPostProcess {
	type QueryData = &'static Self;
	type QueryFilter = With<Camera>;
	type Out = Self;

	fn extract_component(
		item: bevy::ecs::query::QueryItem<'_, '_, Self::QueryData>,
	) -> Option<Self::Out> {
		if item.blur_amount > 1e-4 {
			Some(*item)
		} else {
			None
		}
	}
}

/// Registers embedded **`watercolor_post_process.wgsl`** and the render pass for [`WatercolorPostProcess`].
pub struct WatercolorPostProcessPlugin;

impl Plugin for WatercolorPostProcessPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "watercolor_post_process.wgsl");
		app.add_plugins((
			ExtractComponentPlugin::<WatercolorPostProcess>::default(),
			UniformComponentPlugin::<WatercolorPostProcess>::default(),
		))
		.register_type::<WatercolorPostProcess>()
		.add_systems(PreUpdate, configure_watercolor_post_process_depth_textures);

		let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
			return;
		};

		render_app
			.add_systems(RenderStartup, init_watercolor_post_process_pipelines)
			.add_systems(
				Render,
				configure_watercolor_post_process_depth_textures
					.in_set(RenderSystems::PrepareViews)
					.after(prepare_view_targets),
			)
			.add_systems(Core3d, watercolor_post_process_system.in_set(Core3dSystems::PostProcess));
	}
}

/// Bevy's default depth textures are render-target only; edge-aware blur must sample them.
///
/// Mirrors [`bevy_post_process::dof::configure_depth_of_field_view_targets`].
fn configure_watercolor_post_process_depth_textures(
	mut cameras: Query<&mut Camera3d, With<WatercolorPostProcess>>,
) {
	for mut camera in &mut cameras {
		camera.depth_texture_usages.0 |= TextureUsages::TEXTURE_BINDING.bits();
	}
}

#[derive(Resource)]
struct WatercolorPostProcessPipelines {
	layouts: WatercolorPostProcessLayouts,
	sampler: Sampler,
	non_multisampled: CachedRenderPipelineId,
	multisampled: CachedRenderPipelineId,
}

#[derive(Clone)]
struct WatercolorPostProcessLayouts {
	non_multisampled: BindGroupLayoutDescriptor,
	multisampled: BindGroupLayoutDescriptor,
}

#[derive(Default)]
struct WatercolorPostProcessBindGroupCache {
	cached: Option<(TextureViewId, bool, BindGroup)>,
}

fn watercolor_post_process_system(
	view: ViewQuery<(
		&ViewTarget,
		&WatercolorPostProcess,
		&DynamicUniformIndex<WatercolorPostProcess>,
		&ViewDepthTexture,
		&Msaa,
	)>,
	post_process_pipelines: Option<Res<WatercolorPostProcessPipelines>>,
	pipeline_cache: Res<PipelineCache>,
	settings_uniforms: Res<ComponentUniforms<WatercolorPostProcess>>,
	mut cache: Local<WatercolorPostProcessBindGroupCache>,
	mut ctx: RenderContext,
) {
	let Some(post_process_pipelines) = post_process_pipelines else {
		return;
	};

	let (view_target, _settings, settings_index, depth_texture, msaa) = view.into_inner();
	let multisampled_depth = msaa.samples() > 1;

	let pipeline_id = if multisampled_depth {
		post_process_pipelines.multisampled
	} else {
		post_process_pipelines.non_multisampled
	};

	let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id) else {
		return;
	};

	let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
		return;
	};

	let post_process = view_target.post_process_write();
	let layout = if multisampled_depth {
		&post_process_pipelines.layouts.multisampled
	} else {
		&post_process_pipelines.layouts.non_multisampled
	};

	let bind_group = match &mut cache.cached {
		Some((texture_id, cached_multisampled, bind_group))
			if post_process.source.id() == *texture_id
				&& *cached_multisampled == multisampled_depth =>
		{
			bind_group
		}
		cached => {
			let bind_group = ctx.render_device().create_bind_group(
				"watercolor_post_process_bind_group",
				&pipeline_cache.get_bind_group_layout(layout),
				&BindGroupEntries::sequential((
					post_process.source,
					&post_process_pipelines.sampler,
					depth_texture.view(),
					settings_binding.clone(),
				)),
			);

			let (_, _, bind_group) =
				cached.insert((post_process.source.id(), multisampled_depth, bind_group));
			bind_group
		}
	};

	let mut render_pass = ctx.command_encoder().begin_render_pass(&RenderPassDescriptor {
		label: Some("watercolor_post_process_pass"),
		color_attachments: &[Some(RenderPassColorAttachment {
			view: post_process.destination,
			depth_slice: None,
			resolve_target: None,
			ops: Operations::default(),
		})],
		depth_stencil_attachment: None,
		timestamp_writes: None,
		occlusion_query_set: None,
		multiview_mask: None,
	});

	render_pass.set_pipeline(pipeline);
	render_pass.set_bind_group(0, bind_group, &[settings_index.index()]);
	render_pass.draw(0..3, 0..1);
}

fn watercolor_post_process_layout(multisampled_depth: bool) -> BindGroupLayoutDescriptor {
	let depth_binding =
		if multisampled_depth { texture_depth_2d_multisampled() } else { texture_depth_2d() };

	BindGroupLayoutDescriptor::new(
		if multisampled_depth {
			"watercolor_post_process_bind_group_layout_msaa"
		} else {
			"watercolor_post_process_bind_group_layout"
		},
		&BindGroupLayoutEntries::sequential(
			ShaderStages::FRAGMENT,
			(
				texture_2d(TextureSampleType::Float { filterable: true }),
				sampler(SamplerBindingType::Filtering),
				depth_binding,
				uniform_buffer::<WatercolorPostProcess>(true),
			),
		),
	)
}

fn queue_watercolor_post_process_pipeline(
	pipeline_cache: &PipelineCache,
	fullscreen_shader: &FullscreenShader,
	shader: Handle<Shader>,
	layout: BindGroupLayoutDescriptor,
	multisampled_depth: bool,
) -> CachedRenderPipelineId {
	let mut fragment = FragmentState {
		shader,
		targets: vec![Some(ColorTargetState {
			format: TextureFormat::Rgba8UnormSrgb,
			blend: None,
			write_mask: ColorWrites::ALL,
		})],
		..default()
	};

	if multisampled_depth {
		fragment.shader_defs.push("MULTISAMPLED_DEPTH".into());
	}

	pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
		label: Some(if multisampled_depth {
			"watercolor_post_process_pipeline_msaa".into()
		} else {
			"watercolor_post_process_pipeline".into()
		}),
		layout: vec![layout],
		vertex: fullscreen_shader.to_vertex_state(),
		fragment: Some(fragment),
		..default()
	})
}

fn init_watercolor_post_process_pipelines(
	mut commands: Commands,
	render_device: Res<RenderDevice>,
	asset_server: Res<AssetServer>,
	fullscreen_shader: Res<FullscreenShader>,
	pipeline_cache: Res<PipelineCache>,
) {
	let sampler = render_device.create_sampler(&SamplerDescriptor::default());
	let shader = asset_server.load(concat!(
		"embedded://",
		env!("CARGO_CRATE_NAME"),
		"/watercolor_post_process.wgsl"
	));

	let layouts = WatercolorPostProcessLayouts {
		non_multisampled: watercolor_post_process_layout(false),
		multisampled: watercolor_post_process_layout(true),
	};

	let non_multisampled = queue_watercolor_post_process_pipeline(
		&pipeline_cache,
		&fullscreen_shader,
		shader.clone(),
		layouts.non_multisampled.clone(),
		false,
	);
	let multisampled = queue_watercolor_post_process_pipeline(
		&pipeline_cache,
		&fullscreen_shader,
		shader,
		layouts.multisampled.clone(),
		true,
	);

	commands.insert_resource(WatercolorPostProcessPipelines {
		layouts,
		sampler,
		non_multisampled,
		multisampled,
	});
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_post_process_settings() {
		let settings = WatercolorPostProcess::default();
		assert!((settings.blur_amount - 0.35).abs() < 1e-5);
		assert!((settings.blur_radius - 1.5).abs() < 1e-5);
		assert!((settings.depth_edge_sharpness - 80.0).abs() < 1e-5);
		assert!((settings.edge_aware - 1.0).abs() < 1e-5);
	}

	#[test]
	fn builder_methods() {
		let settings = WatercolorPostProcess::default()
			.with_blur_amount(0.25)
			.with_blur_radius(2.0)
			.with_edge_aware(false);
		assert!((settings.blur_amount - 0.25).abs() < 1e-5);
		assert!((settings.blur_radius - 2.0).abs() < 1e-5);
		assert!((settings.edge_aware).abs() < 1e-5);
	}
}
