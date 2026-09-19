//! Opaque Cosimo cosmos behind the blue dome.
//!
//! Sun and moon are uploaded so the clock can keep posing them, but the
//! field shader does not draw them — the inner dome owns the disks.

use bevy::{
	asset::embedded_asset,
	light::NotShadowCaster,
	mesh::MeshVertexBufferLayoutRef,
	pbr::{MaterialPipeline, MaterialPipelineKey},
	prelude::*,
	reflect::TypePath,
	render::render_resource::{
		AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
	},
	shader::ShaderRef,
};

use crate::clock::SkyMood;

pub const FIELD_RADIUS_FACTOR: f32 = 1.06;

/// Camera-parented cosmos. Atmosphere lives on the inner blue dome.
#[derive(Component, Debug, Clone, Copy)]
pub struct SkyField;

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct SkyFieldParams {
	pub zenith: Vec4,
	pub horizon: Vec4,
	pub nadir: Vec4,
	/// `x` day weight, `y` swirl gain, `z` star gain, `w` phase.
	pub style: Vec4,
	pub sun_dir: Vec4,
	pub moon_dir: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkyFieldMaterial {
	#[uniform(0)]
	pub params: SkyFieldParams,
}

impl SkyFieldMaterial {
	pub fn from_mood(mood: SkyMood) -> Self {
		Self {
			params: SkyFieldParams {
				zenith: color_vec4(mood.zenith),
				horizon: color_vec4(mood.horizon),
				nadir: color_vec4(mood.nadir),
				style: Vec4::new(mood.day_weight, mood.swirl_gain, mood.star_gain, mood.phase),
				sun_dir: mood.sun_disk_dir().extend(0.0),
				moon_dir: mood.moon_dir().extend(0.0),
			},
		}
	}

	pub fn apply_mood(&mut self, mood: SkyMood) {
		*self = Self::from_mood(mood);
	}
}

impl Default for SkyFieldMaterial {
	fn default() -> Self {
		Self::from_mood(crate::SkyClock::golden().sample())
	}
}

impl Material for SkyFieldMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "field.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "field.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
	}

	fn enable_prepass() -> bool {
		false
	}

	fn enable_shadows() -> bool {
		false
	}

	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = None;
		Ok(())
	}
}

pub struct SkyFieldMaterialPlugin;

impl Plugin for SkyFieldMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "field.wgsl");
		app.init_asset::<SkyFieldMaterial>();
		if app.is_plugin_added::<bevy::render::RenderPlugin>() {
			app.add_plugins(MaterialPlugin::<SkyFieldMaterial>::default());
		}
	}
}

pub(crate) fn spawn_sky_field(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<SkyFieldMaterial>>,
	settings: Res<crate::dome::DomeSettings>,
	clock: Res<crate::SkyClock>,
	dome: Query<Entity, With<crate::SkyDome>>,
) {
	let Ok(parent) = dome.single() else {
		return;
	};
	let radius = settings.sphere_radius_m * FIELD_RADIUS_FACTOR;
	let material = materials.add(SkyFieldMaterial::from_mood(clock.sample()));
	commands.spawn((
		Name::new("sky-field"),
		SkyField,
		Mesh3d(meshes.add(settings.shell_mesh(radius))),
		MeshMaterial3d(material),
		Transform::IDENTITY,
		Visibility::Inherited,
		NotShadowCaster,
		ChildOf(parent),
	));
}

fn color_vec4(color: Color) -> Vec4 {
	let c = color.to_linear();
	Vec4::new(c.red, c.green, c.blue, 1.0)
}
