//! Inner blue / haze dome. Fragment alpha opens onto the cosmos field.

use bevy::asset::embedded_asset;
use bevy::light::NotShadowCaster;
use bevy::mesh::primitives::MeshBuilder;
use bevy::mesh::{MeshVertexBufferLayoutRef, SphereKind, SphereMeshBuilder};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
	AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use crate::clock::SkyMood;
use crate::{SKY_HORIZON, SKY_NADIR, SKY_ZENITH};

#[derive(Resource, Clone, Copy)]
pub(crate) struct DomeSettings {
	#[allow(dead_code)]
	pub inner_fade_m: f32,
	#[allow(dead_code)]
	pub outer_fade_m: f32,
	pub sphere_radius_m: f32,
	pub max_alpha: f32,
	#[allow(dead_code)]
	pub horizon: Color,
	#[allow(dead_code)]
	pub zenith: Color,
	#[allow(dead_code)]
	pub nadir: Color,
}

impl Default for DomeSettings {
	fn default() -> Self {
		Self {
			inner_fade_m: crate::DEFAULT_INNER_FADE_M,
			outer_fade_m: crate::DEFAULT_OUTER_FADE_M,
			sphere_radius_m: crate::DEFAULT_SPHERE_RADIUS_M,
			max_alpha: crate::DEFAULT_MAX_ALPHA,
			horizon: SKY_HORIZON,
			zenith: SKY_ZENITH,
			nadir: SKY_NADIR,
		}
	}
}

/// Inner atmosphere shell. Cosmos shows through its alpha holes.
#[derive(Component, Debug, Clone, Copy)]
pub struct SkyWash;

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct SkyDomeParams {
	pub zenith: Vec4,
	pub horizon: Vec4,
	pub nadir: Vec4,
	/// `x` day weight, `y` peak alpha, `z` star gain, `w` phase.
	pub style: Vec4,
	pub sun_dir: Vec4,
	pub moon_dir: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkyDomeMaterial {
	#[uniform(0)]
	pub params: SkyDomeParams,
}

impl SkyDomeMaterial {
	pub fn from_mood(mood: SkyMood, peak_alpha: f32) -> Self {
		Self {
			params: SkyDomeParams {
				zenith: color_vec4(mood.zenith),
				horizon: color_vec4(mood.horizon),
				nadir: color_vec4(mood.nadir),
				style: Vec4::new(mood.day_weight, peak_alpha, mood.star_gain, mood.phase),
				sun_dir: mood.sun_disk_dir().extend(0.0),
				moon_dir: mood.moon_dir().extend(0.0),
			},
		}
	}

	pub fn apply_mood(&mut self, mood: SkyMood, peak_alpha: f32) {
		*self = Self::from_mood(mood, peak_alpha);
	}
}

impl Default for SkyDomeMaterial {
	fn default() -> Self {
		Self::from_mood(crate::SkyClock::golden().sample(), crate::DEFAULT_MAX_ALPHA)
	}
}

impl Material for SkyDomeMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "dome.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "dome.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Blend
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

pub struct SkyDomeMaterialPlugin;

impl Plugin for SkyDomeMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "dome.wgsl");
		app.init_asset::<SkyDomeMaterial>();
		if app.is_plugin_added::<bevy::render::RenderPlugin>() {
			app.add_plugins(MaterialPlugin::<SkyDomeMaterial>::default());
		}
	}
}

impl DomeSettings {
	/// Inverted shell. Both atmosphere and cosmos paint from view direction.
	pub(crate) fn shell_mesh(self, radius: f32) -> Mesh {
		self.sphere_mesh(radius)
	}

	fn sphere_mesh(self, radius: f32) -> Mesh {
		// Ico has no UV-sphere pole. Noon looks straight up; a UV pole was a hole.
		SphereMeshBuilder::new(radius, SphereKind::Ico { subdivisions: 4 }).build()
	}
}

pub(crate) fn spawn_sky_wash(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<SkyDomeMaterial>>,
	settings: Res<DomeSettings>,
	clock: Res<crate::SkyClock>,
	dome: Query<Entity, With<crate::SkyDome>>,
) {
	let Ok(parent) = dome.single() else {
		return;
	};
	let material = materials.add(SkyDomeMaterial::from_mood(clock.sample(), settings.max_alpha));
	commands.spawn((
		Name::new("sky-blue-dome"),
		SkyWash,
		Mesh3d(meshes.add(settings.shell_mesh(settings.sphere_radius_m))),
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn blue_dome_blends_over_cosmos() {
		let material = SkyDomeMaterial::default();
		assert_eq!(material.alpha_mode(), AlphaMode::Blend);
		assert!(material.params.style.y > 0.5, "peak alpha should be a real atmosphere");
	}

	#[test]
	fn horizon_stays_warmer_than_zenith() {
		let horizon = SKY_HORIZON.to_linear();
		let zenith = SKY_ZENITH.to_linear();
		assert!(horizon.red > zenith.red, "horizon stays warmer than zenith");
		assert!(zenith.blue > horizon.blue, "zenith stays cooler than horizon");
	}
}
