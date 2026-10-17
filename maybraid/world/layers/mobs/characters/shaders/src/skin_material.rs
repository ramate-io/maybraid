//! Skin [`Material`] — hide shading plus a UV0 tattoo emission mask.
//!
//! Recipes [`RECIPE_SKIN_TATTOO_BODY`] / [`RECIPE_SKIN_TATTOO_HEAD`] are claimed
//! by the character [`material_ref::MaterialLib`]. Color, intensity, and pulse
//! live on [`SkinTattoo`] (open [#1128](https://github.com/ramate-io/maybraid/issues/1128)
//! knobs). Masks are placeholder PNGs; see
//! `maybraid/assets/characters/tattoos/README.md`.

use bevy::{
	asset::{embedded_asset, RenderAssetUsages},
	image::{CompressedImageFormats, ImageSampler, ImageType},
	mesh::MeshVertexBufferLayoutRef,
	pbr::{MaterialPipeline, MaterialPipelineKey},
	prelude::*,
	reflect::TypePath,
	render::render_resource::{
		AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
		TextureDimension, TextureFormat,
	},
	shader::ShaderRef,
};
use material_ref::{
	MaterialId, MaterialRasters, MaterialRef, MATERIAL_PALETTE_SLOTS, MATERIAL_RASTER_CHANNELS,
	MATERIAL_RASTER_WIDTH, MATERIAL_SCALAR_FLOATS,
};

/// Body hide + tattoo mask (provisional unwrap until `epiphant.glb` gains UV0).
pub const RECIPE_SKIN_TATTOO_BODY: &str = "skin_tattoo_body";
/// Head hide + tattoo mask on existing meerkat UV0.
pub const RECIPE_SKIN_TATTOO_HEAD: &str = "skin_tattoo_head";
/// Ear hide + tattoo mask (planar XY unwrap; ear GLB has no UV0).
pub const RECIPE_SKIN_TATTOO_EAR: &str = "skin_tattoo_ear";
/// Trunk hide + tattoo mask (cylindrical about Y; trunk GLB has no UV0).
pub const RECIPE_SKIN_TATTOO_TRUNK: &str = "skin_tattoo_trunk";

pub const KIND_BODY: u32 = 0;
pub const KIND_HEAD: u32 = 1;
pub const KIND_EAR: u32 = 2;
pub const KIND_TRUNK: u32 = 3;
/// `MAYBRAID_TATTOO_UV_DEBUG=1` — checker + mask overlay (investigation only).
pub const FLAG_UV_DEBUG: u32 = 1;

pub const SKIN_PALETTE_BASE: usize = 0;
pub const SKIN_PALETTE_GLOW: usize = 1;
pub const SKIN_SCALAR_INTENSITY: usize = 0;
pub const SKIN_SCALAR_PULSE_RATE: usize = 1;
pub const SKIN_SCALAR_PULSE_AMP: usize = 2;

/// Placeholder gold / amber (God Beam family). Open: final glow color.
pub const DEFAULT_TATTOO_GLOW: Color = Color::srgb(1.0, 0.72, 0.28);
/// Placeholder intensity. Open: daylight vs night balance.
pub const DEFAULT_TATTOO_INTENSITY: f32 = 4.0;
/// `0` = no pulse. Open: idle vs Bray-synced.
pub const DEFAULT_TATTOO_PULSE_RATE: f32 = 0.0;
pub const DEFAULT_TATTOO_PULSE_AMP: f32 = 0.25;

const SCALAR_VEC4S: usize = MATERIAL_SCALAR_FLOATS / 4;
const DEFAULT_SKIN_COLOR: Vec4 = Vec4::new(0.42, 0.44, 0.48, 1.0);
const DEFAULT_GLOW_COLOR: Vec4 = Vec4::new(1.0, 0.72, 0.28, 1.0);

const BODY_MASK_PNG: &[u8] = include_bytes!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/../../../../../assets/characters/tattoos/epiphant_body_uv0_placeholder.png"
));
const HEAD_MASK_PNG: &[u8] = include_bytes!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/../../../../../assets/characters/tattoos/epiphant_head_uv0_placeholder.png"
));
const EAR_MASK_PNG: &[u8] = include_bytes!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/../../../../../assets/characters/tattoos/epiphant_ear_uv0_placeholder.png"
));
const TRUNK_MASK_PNG: &[u8] = include_bytes!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/../../../../../assets/characters/tattoos/epiphant_trunk_uv0_placeholder.png"
));

/// Which UV0 mask a part host should stamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkinTattooMask {
	Body,
	Head,
	Ear,
	Trunk,
}

impl SkinTattooMask {
	pub const fn recipe_name(self) -> &'static str {
		match self {
			Self::Body => RECIPE_SKIN_TATTOO_BODY,
			Self::Head => RECIPE_SKIN_TATTOO_HEAD,
			Self::Ear => RECIPE_SKIN_TATTOO_EAR,
			Self::Trunk => RECIPE_SKIN_TATTOO_TRUNK,
		}
	}

	pub fn from_recipe_name(name: &str) -> Self {
		match name {
			RECIPE_SKIN_TATTOO_HEAD => Self::Head,
			RECIPE_SKIN_TATTOO_EAR => Self::Ear,
			RECIPE_SKIN_TATTOO_TRUNK => Self::Trunk,
			_ => Self::Body,
		}
	}

	pub fn is_skin_tattoo_recipe(name: &str) -> bool {
		matches!(
			name,
			RECIPE_SKIN_TATTOO_BODY
				| RECIPE_SKIN_TATTOO_HEAD
				| RECIPE_SKIN_TATTOO_EAR
				| RECIPE_SKIN_TATTOO_TRUNK
		)
	}
}

/// Per-character tattoo knobs. Ward / Enraged reuse this and recolor `glow`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkinTattoo {
	pub glow: Color,
	pub intensity: f32,
	pub pulse_rate: f32,
	pub pulse_amp: f32,
}

impl Default for SkinTattoo {
	fn default() -> Self {
		Self {
			glow: DEFAULT_TATTOO_GLOW,
			intensity: DEFAULT_TATTOO_INTENSITY,
			pulse_rate: DEFAULT_TATTOO_PULSE_RATE,
			pulse_amp: DEFAULT_TATTOO_PULSE_AMP,
		}
	}
}

impl SkinTattoo {
	pub fn material_ref(self, mask: SkinTattooMask, skin: Color) -> MaterialRef {
		MaterialRef::named(mask.recipe_name())
			.with_palette([skin, self.glow])
			.with_scalars([self.intensity, self.pulse_rate, self.pulse_amp])
	}
}

/// Decoded placeholder masks used by [`SkinShaderMaterial`].
#[derive(Resource, Clone, Default)]
pub struct SkinTattooMasks {
	pub body: Handle<Image>,
	pub head: Handle<Image>,
	pub ear: Handle<Image>,
	pub trunk: Handle<Image>,
}

impl SkinTattooMasks {
	pub fn handle_for(&self, mask: SkinTattooMask) -> Handle<Image> {
		match mask {
			SkinTattooMask::Body => self.body.clone(),
			SkinTattooMask::Head => self.head.clone(),
			SkinTattooMask::Ear => self.ear.clone(),
			SkinTattooMask::Trunk => self.trunk.clone(),
		}
	}

	pub fn handle_for_recipe(&self, name: &str) -> Handle<Image> {
		self.handle_for(SkinTattooMask::from_recipe_name(name))
	}
}

/// Registers embedded **`skin_material.wgsl`**, placeholder masks, and [`MaterialPlugin`].
pub struct SkinShaderMaterialPlugin;

impl Plugin for SkinShaderMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "skin_material.wgsl");
		app.add_plugins(MaterialPlugin::<SkinShaderMaterial>::default());
		install_skin_tattoo_masks(app);
	}
}

fn install_skin_tattoo_masks(app: &mut App) {
	if !app.world().contains_resource::<Assets<Image>>() {
		app.init_resource::<SkinTattooMasks>();
		return;
	}
	let masks = {
		let mut images = app.world_mut().resource_mut::<Assets<Image>>();
		let body = decode_mask_png(BODY_MASK_PNG).unwrap_or_else(fallback_mask);
		let head = decode_mask_png(HEAD_MASK_PNG).unwrap_or_else(fallback_mask);
		let ear = decode_mask_png(EAR_MASK_PNG).unwrap_or_else(fallback_mask);
		let trunk = decode_mask_png(TRUNK_MASK_PNG).unwrap_or_else(fallback_mask);
		SkinTattooMasks {
			body: images.add(body),
			head: images.add(head),
			ear: images.add(ear),
			trunk: images.add(trunk),
		}
	};
	app.insert_resource(masks);
}

fn decode_mask_png(bytes: &[u8]) -> Option<Image> {
	Image::from_buffer(
		bytes,
		ImageType::Extension("png"),
		CompressedImageFormats::NONE,
		true,
		ImageSampler::linear(),
		RenderAssetUsages::default(),
	)
	.ok()
}

fn fallback_mask() -> Image {
	Image::new_fill(
		Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
		TextureDimension::D2,
		&[255, 255, 255, 255],
		TextureFormat::Rgba8UnormSrgb,
		RenderAssetUsages::default(),
	)
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct SkinMaterialUniform {
	pub colors: [Vec4; MATERIAL_PALETTE_SLOTS],
	pub noise: Vec4,
	pub scalars: [Vec4; SCALAR_VEC4S],
	pub rasters: [[Vec4; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS],
	pub kind: u32,
	pub flags: u32,
	pub _pad: UVec2,
}

impl SkinMaterialUniform {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let kind = match &material_ref.name {
			MaterialId::Name(name) => SkinTattooMask::from_recipe_name(name),
			MaterialId::Default => SkinTattooMask::Body,
		};

		let mut colors = [Vec4::ZERO; MATERIAL_PALETTE_SLOTS];
		if material_ref.palette.is_empty() {
			colors[SKIN_PALETTE_BASE] = DEFAULT_SKIN_COLOR;
			colors[SKIN_PALETTE_GLOW] = DEFAULT_GLOW_COLOR;
		} else {
			for (slot, color) in colors.iter_mut().zip(&material_ref.palette) {
				*slot = linear_vec4(*color);
			}
			if material_ref.palette.len() < 2 {
				colors[SKIN_PALETTE_GLOW] = DEFAULT_GLOW_COLOR;
			}
		}

		let mut scalars = [Vec4::ZERO; SCALAR_VEC4S];
		let values = material_ref.scalar_values();
		for (i, slot) in scalars.iter_mut().enumerate() {
			let base = i * 4;
			*slot = Vec4::new(
				values.get(base).copied().unwrap_or(0.0),
				values.get(base + 1).copied().unwrap_or(0.0),
				values.get(base + 2).copied().unwrap_or(0.0),
				values.get(base + 3).copied().unwrap_or(0.0),
			);
		}
		if scalars[0][SKIN_SCALAR_INTENSITY] <= 1e-4 {
			scalars[0][SKIN_SCALAR_INTENSITY] = DEFAULT_TATTOO_INTENSITY;
		}

		let mut rasters = [[Vec4::ZERO; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS];
		for (channel, raster) in rasters.iter_mut().enumerate() {
			let samples = material_ref.rasters.get_or(channel, 0.0);
			let rows = MaterialRasters::packed_rows(samples);
			*raster = rows.map(Vec4::from_array);
		}

		Self {
			colors,
			noise: Vec4::new(
				material_ref.noise.frequency.max(1e-6),
				material_ref.noise.amplitude,
				material_ref.noise.seed as f32,
				material_ref.noise.octaves as f32,
			),
			scalars,
			rasters,
			kind: match kind {
				SkinTattooMask::Body => KIND_BODY,
				SkinTattooMask::Head => KIND_HEAD,
				SkinTattooMask::Ear => KIND_EAR,
				SkinTattooMask::Trunk => KIND_TRUNK,
			},
			flags: if std::env::var_os("MAYBRAID_TATTOO_UV_DEBUG").is_some() {
				FLAG_UV_DEBUG
			} else {
				0
			},
			_pad: UVec2::ZERO,
		}
	}
}

fn linear_vec4(color: Color) -> Vec4 {
	let linear = LinearRgba::from(color);
	Vec4::new(linear.red, linear.green, linear.blue, linear.alpha)
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkinShaderMaterial {
	#[uniform(0)]
	pub params: SkinMaterialUniform,
	#[texture(1)]
	#[sampler(2)]
	pub tattoo_mask: Handle<Image>,
}

impl SkinShaderMaterial {
	pub fn from_material_ref(material_ref: &MaterialRef, tattoo_mask: Handle<Image>) -> Self {
		Self { params: SkinMaterialUniform::from_material_ref(material_ref), tattoo_mask }
	}
}

impl Default for SkinShaderMaterial {
	fn default() -> Self {
		Self::from_material_ref(&MaterialRef::named(RECIPE_SKIN_TATTOO_BODY), Handle::default())
	}
}

impl Material for SkinShaderMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "skin_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "skin_material.wgsl").into()
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

/// Bind-pose cylindrical UV used when `epiphant.glb` has no TEXCOORD_0.
pub fn body_provisional_uv(local: Vec3) -> Vec2 {
	const Z_MIN: f32 = -0.9804133;
	const Z_SPAN: f32 = 2.0035005;
	let u = local.x.atan2(local.y) * std::f32::consts::FRAC_1_PI * 0.5 + 0.5;
	let v = ((local.z - Z_MIN) / Z_SPAN).clamp(0.0, 1.0);
	Vec2::new(u, v)
}

/// Planar XY UV used when `epiphant_ear_left.glb` has no TEXCOORD_0.
pub fn ear_provisional_uv(local: Vec3) -> Vec2 {
	const X_MIN: f32 = -0.295_222_76;
	const X_SPAN: f32 = 2.330_160_9;
	const Y_MIN: f32 = -1.652_735_1;
	const Y_SPAN: f32 = 2.981_944_3;
	let u = ((local.x - X_MIN) / X_SPAN).clamp(0.0, 1.0);
	let v = ((local.y - Y_MIN) / Y_SPAN).clamp(0.0, 1.0);
	Vec2::new(u, v)
}

/// Cylindrical-about-Y UV used when `trunkish_nose.glb` has no TEXCOORD_0.
pub fn trunk_provisional_uv(local: Vec3) -> Vec2 {
	const Y_MIN: f32 = -1.932_509;
	const Y_SPAN: f32 = 2.469_754_7;
	let u = local.x.atan2(local.z) * std::f32::consts::FRAC_1_PI * 0.5 + 0.5;
	let v = ((local.y - Y_MIN) / Y_SPAN).clamp(0.0, 1.0);
	Vec2::new(u, v)
}

/// Cylindrical-about-Y UV for the meerkat head. Authored `TEXCOORD_0` collapses
/// most verts to `(0, 1)`; this unwrap puts the face at `u = 0.5`.
pub fn head_provisional_uv(local: Vec3) -> Vec2 {
	const Y_MIN: f32 = -0.979_147_3;
	const Y_SPAN: f32 = 2.041_864_7;
	let u = local.x.atan2(local.z) * std::f32::consts::FRAC_1_PI * 0.5 + 0.5;
	let v = ((local.y - Y_MIN) / Y_SPAN).clamp(0.0, 1.0);
	Vec2::new(u, v)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_material_ref_packs_palette_and_scalars() {
		let material_ref = MaterialRef::named(RECIPE_SKIN_TATTOO_HEAD)
			.with_palette([Color::linear_rgb(0.2, 0.3, 0.4), Color::linear_rgb(1.0, 0.5, 0.1)])
			.with_scalars([2.5, 0.5, 0.2]);
		let material = SkinShaderMaterial::from_material_ref(&material_ref, Handle::default());
		assert_eq!(material.params.kind, KIND_HEAD);
		assert!((material.params.colors[SKIN_PALETTE_BASE].x - 0.2).abs() < 0.02);
		assert!((material.params.scalars[0][SKIN_SCALAR_INTENSITY] - 2.5).abs() < 1e-5);
		assert!((material.params.scalars[0][SKIN_SCALAR_PULSE_RATE] - 0.5).abs() < 1e-5);
		assert!((material.params.scalars[0][SKIN_SCALAR_PULSE_AMP] - 0.2).abs() < 1e-5);
	}

	#[test]
	fn default_intensity_fills_when_scalars_omitted() {
		let material = SkinShaderMaterial::from_material_ref(
			&MaterialRef::named(RECIPE_SKIN_TATTOO_BODY),
			Handle::default(),
		);
		assert_eq!(material.params.kind, KIND_BODY);
		assert!(
			(material.params.scalars[0][SKIN_SCALAR_INTENSITY] - DEFAULT_TATTOO_INTENSITY).abs()
				< 1e-5
		);
		assert!((material.params.colors[SKIN_PALETTE_GLOW].x - DEFAULT_GLOW_COLOR.x).abs() < 1e-5);
	}

	#[test]
	fn tattoo_recipe_names() {
		assert!(SkinTattooMask::is_skin_tattoo_recipe(RECIPE_SKIN_TATTOO_BODY));
		assert!(SkinTattooMask::is_skin_tattoo_recipe(RECIPE_SKIN_TATTOO_HEAD));
		assert!(SkinTattooMask::is_skin_tattoo_recipe(RECIPE_SKIN_TATTOO_EAR));
		assert!(SkinTattooMask::is_skin_tattoo_recipe(RECIPE_SKIN_TATTOO_TRUNK));
		assert!(!SkinTattooMask::is_skin_tattoo_recipe("face_eye"));
	}

	#[test]
	fn skin_tattoo_builds_part_host_ref() {
		let tattoo =
			SkinTattoo { glow: Color::srgb(0.1, 0.8, 1.0), intensity: 1.5, ..Default::default() };
		let material = tattoo.material_ref(SkinTattooMask::Body, Color::srgb(0.3, 0.3, 0.3));
		assert_eq!(material.name, MaterialId::named(RECIPE_SKIN_TATTOO_BODY));
		assert_eq!(material.palette.len(), 2);
		assert!((material.scalar_values()[SKIN_SCALAR_INTENSITY] - 1.5).abs() < 1e-5);
	}

	#[test]
	fn body_provisional_uv_puts_spine_at_u_half() {
		let uv = body_provisional_uv(Vec3::new(0.0, 0.4, 0.0));
		assert!((uv.x - 0.5).abs() < 1e-4, "spine u={}", uv.x);
		assert!((uv.y - ((0.0 + 0.9804133) / 2.0035005)).abs() < 1e-4);
	}

	#[test]
	fn placeholder_pngs_decode() {
		assert!(decode_mask_png(BODY_MASK_PNG).is_some());
		assert!(decode_mask_png(HEAD_MASK_PNG).is_some());
		assert!(decode_mask_png(EAR_MASK_PNG).is_some());
		assert!(decode_mask_png(TRUNK_MASK_PNG).is_some());
	}

	#[test]
	fn ear_provisional_uv_uses_planar_xy() {
		let uv = ear_provisional_uv(Vec3::new(-0.295_222_76, -1.652_735_1, 0.0));
		assert!(uv.x.abs() < 1e-4 && uv.y.abs() < 1e-4);
	}

	#[test]
	fn trunk_provisional_uv_puts_forward_at_u_half() {
		let uv = trunk_provisional_uv(Vec3::new(0.0, 0.0, 0.4));
		assert!((uv.x - 0.5).abs() < 1e-4, "forward u={}", uv.x);
	}

	#[test]
	fn head_provisional_uv_puts_face_at_u_half() {
		let uv = head_provisional_uv(Vec3::new(0.0, 0.0, 0.8));
		assert!((uv.x - 0.5).abs() < 1e-4, "face u={}", uv.x);
		assert!((uv.y - ((0.0 + 0.979_147_3) / 2.041_864_7)).abs() < 1e-4);
	}

	#[test]
	fn uv_debug_flag_is_off_by_default() {
		let material = SkinShaderMaterial::from_material_ref(
			&MaterialRef::named(RECIPE_SKIN_TATTOO_HEAD),
			Handle::default(),
		);
		assert_eq!(material.params.flags & FLAG_UV_DEBUG, 0);
	}
}
