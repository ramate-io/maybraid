//! Opaque authored surfaces, indexed by MeshTag through Bevy GPU instancing.
use bevy::{
	asset::embedded_asset,
	prelude::*,
	render::render_resource::{AsBindGroup, ShaderType},
	shader::ShaderRef,
};
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct Surface {
	pub centroid: Vec4,
	pub opening: Vec4,
	pub controls: Vec4,
	pub effect_from_world: Mat4,
	pub tint: Vec4,
}
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct CartoonMaterial {
	#[storage(0, read_only)]
	pub data: Handle<bevy::render::storage::ShaderBuffer>,
}
impl Material for CartoonMaterial {
	// Mask classification requests the material depth fragment; retained fragments are opaque.
	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Mask(0.5)
	}
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/parts/smoke_explosion/cartoon.wesl")
			.into()
	}
	fn fragment_shader() -> ShaderRef {
		Self::vertex_shader()
	}
	fn prepass_vertex_shader() -> ShaderRef {
		Self::prepass_fragment_shader()
	}
	fn prepass_fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/parts/smoke_explosion/depth.wesl").into()
	}
	fn enable_shadows() -> bool {
		false
	}
}
pub fn plugin(app: &mut App) {
	embedded_asset!(app, "cartoon.wesl");
	embedded_asset!(app, "depth.wesl");
	app.add_plugins(MaterialPlugin::<CartoonMaterial>::default())
		.init_resource::<Pool>()
		.add_systems(Startup, setup);
}

impl Surface {
	pub fn rows(self) -> [Vec4; 8] {
		[
			self.centroid,
			self.opening,
			self.controls,
			self.effect_from_world.x_axis,
			self.effect_from_world.y_axis,
			self.effect_from_world.z_axis,
			self.effect_from_world.w_axis,
			self.tint,
		]
	}
}
#[derive(Resource, Default)]
pub struct Pool {
	pub material: Handle<CartoonMaterial>,
	pub buffer: Handle<bevy::render::storage::ShaderBuffer>,
	pub values: Vec<Vec4>,
}
fn setup(
	mut pool: ResMut<Pool>,
	mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
	mut materials: ResMut<Assets<CartoonMaterial>>,
) {
	pool.values = vec![Vec4::ZERO; (8 * 349 + 3 * 96) * 8];
	pool.buffer = buffers.add(bevy::render::storage::ShaderBuffer::new(
		pool.values.clone(),
		bevy::asset::RenderAssetUsages::all(),
	));
	pool.material = materials.add(CartoonMaterial { data: pool.buffer.clone() });
}
