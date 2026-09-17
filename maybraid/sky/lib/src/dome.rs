//! Inner blue / haze dome. Fragment alpha opens onto the cosmos field.

use bevy::asset::{embedded_asset, RenderAssetUsages};
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
	AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use std::f32::consts::PI;

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
	/// `x` day weight, `y` peak alpha, `z` unused, `w` phase.
	pub style: Vec4,
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
				style: Vec4::new(mood.day_weight, peak_alpha, 0.0, mood.phase),
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
		let rings = 48u32;
		let segs = 64u32;

		let mut positions = Vec::new();
		let mut normals = Vec::new();
		let mut indices = Vec::new();

		for ring in 0..=rings {
			let v = ring as f32 / rings as f32;
			let theta = v * PI;
			let y = radius * theta.cos();
			let ring_r = radius * theta.sin();
			for seg in 0..=segs {
				let u = seg as f32 / segs as f32;
				let phi = u * 2.0 * PI;
				let x = ring_r * phi.cos();
				let z = ring_r * phi.sin();
				positions.push([x, y, z]);
				let len = (x * x + y * y + z * z).sqrt().max(1e-5);
				normals.push([-x / len, -y / len, -z / len]);
			}
		}

		let verts_per_ring = segs + 1;
		for ring in 0..rings {
			for seg in 0..segs {
				let a = ring * verts_per_ring + seg;
				let b = a + verts_per_ring;
				indices.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
			}
		}

		let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
		mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
		mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
		mesh.insert_indices(Indices::U32(indices));
		mesh
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
