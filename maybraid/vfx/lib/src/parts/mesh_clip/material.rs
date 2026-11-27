use bevy::{
	asset::embedded_asset, prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef,
};
/// Snapshot of spawn appearance. Alpha is intentionally ignored by opaque surfaces.
#[derive(Clone, Copy, Debug)]
pub struct ClipAppearance {
	pub rgb_gain: Vec3,
}
impl ClipAppearance {
	pub fn from_spawn(spawn: &crate::VfxSpawn) -> Self {
		let tint = spawn.tint_or_white().to_linear();
		Self { rgb_gain: Vec3::new(tint.red, tint.green, tint.blue) * spawn.clamped_intensity() }
	}
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ClipMaterial {
	#[uniform(0)]
	pub cutoff: Vec4,
	#[uniform(1)]
	pub seed: UVec4,
	#[uniform(2)]
	pub light: Vec4,
	#[uniform(3)]
	pub dark: Vec4,
	#[uniform(4)]
	pub mid: Vec4,
	#[uniform(5)]
	pub bright: Vec4,
	#[uniform(6)]
	pub thresholds: Vec4,
}
impl ClipMaterial {
	pub fn new(seed: u32, style: &crate::parts::mesh_clip::asset::SurfaceStyle) -> Self {
		Self::with_appearance(seed, style, ClipAppearance { rgb_gain: Vec3::ONE })
	}
	pub fn with_appearance(
		seed: u32,
		style: &crate::parts::mesh_clip::asset::SurfaceStyle,
		appearance: ClipAppearance,
	) -> Self {
		let color = |c: [f32; 3]| (Vec3::from_array(c) * appearance.rgb_gain).extend(1.);
		Self {
			cutoff: Vec4::splat(-0.02),
			seed: UVec4::new(seed, 0, 0, 0),
			light: Vec3::from_array(style.light).extend(0.),
			dark: color(style.colors[0]),
			mid: color(style.colors[1]),
			bright: color(style.colors[2]),
			thresholds: Vec4::new(style.thresholds[0], style.thresholds[1], 0., 0.),
		}
	}
}
impl Material for ClipMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/parts/mesh_clip/surface.wesl").into()
	}
	fn fragment_shader() -> ShaderRef {
		Self::vertex_shader()
	}
	// MAY_DISCARD is required: Bevy omits depth-only fragment shaders for Opaque.
	// Mask uses the opaque depth-writing phase; all retained fragments still output alpha 1.
	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Mask(0.5)
	}
	fn prepass_vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/parts/mesh_clip/prepass.wesl").into()
	}
	fn prepass_fragment_shader() -> ShaderRef {
		Self::prepass_vertex_shader()
	}
	fn enable_prepass() -> bool {
		true
	}
	fn enable_shadows() -> bool {
		false
	}
}
pub struct ClipMaterialPlugin;
impl Plugin for ClipMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "surface.wesl");
		bevy::shader::load_shader_library!(app, "common.wesl");
		embedded_asset!(app, "prepass.wesl");
		app.add_plugins(MaterialPlugin::<ClipMaterial>::default());
	}
}
