//! Face [`Material`] — painted iris / lid wrap blink, and millimetre mouth idle.
//!
//! Phase is `globals.time` + instance seed. No mailbox. No blendshapes.
//! `face_eye` palette: iris, pupil, sclera, catchlight, limbus, lid.

use bevy::{
	asset::embedded_asset,
	mesh::MeshVertexBufferLayoutRef,
	pbr::{MaterialPipeline, MaterialPipelineKey},
	prelude::*,
	reflect::TypePath,
	render::render_resource::{
		AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
	},
	shader::ShaderRef,
};
use material_ref::{
	MaterialId, MaterialRasters, MaterialRef, MATERIAL_PALETTE_SLOTS, MATERIAL_RASTER_CHANNELS,
	MATERIAL_RASTER_WIDTH, MATERIAL_SCALAR_FLOATS,
};

pub const RECIPE_FACE_EYE: &str = "face_eye";
pub const RECIPE_FACE_MOUTH: &str = "face_mouth";

pub const KIND_EYE: u32 = 0;
pub const KIND_MOUTH: u32 = 1;

/// `face_eye` palette: iris, pupil, sclera, catchlight, limbus, lid.
pub const EYE_PALETTE_IRIS: usize = 0;
pub const EYE_PALETTE_PUPIL: usize = 1;
pub const EYE_PALETTE_SCLERA: usize = 2;
pub const EYE_PALETTE_HIGHLIGHT: usize = 3;
pub const EYE_PALETTE_LIMBUS: usize = 4;
pub const EYE_PALETTE_LID: usize = 5;
const EYE_PALETTE_DETAIL_SLOTS: usize = 6;

const SCALAR_VEC4S: usize = MATERIAL_SCALAR_FLOATS / 4;
const DEFAULT_EYE_COLOR: Vec4 = Vec4::new(0.22, 0.16, 0.12, 1.0);
const DEFAULT_MOUTH_COLOR: Vec4 = Vec4::new(0.62, 0.32, 0.30, 1.0);

/// Species eye color as iris, plus derived pupil / sclera / highlight / limbus / lid.
pub fn eye_palette(iris: Color) -> [Color; EYE_PALETTE_DETAIL_SLOTS] {
	let mut colors = derived_eye_palette(linear_vec4(iris))
		.map(|color| Color::linear_rgba(color.x, color.y, color.z, color.w));
	colors[EYE_PALETTE_IRIS] = iris;
	colors
}

fn linear_vec4(color: Color) -> Vec4 {
	let linear = LinearRgba::from(color);
	Vec4::new(linear.red, linear.green, linear.blue, linear.alpha)
}

fn derived_eye_palette(iris: Vec4) -> [Vec4; EYE_PALETTE_DETAIL_SLOTS] {
	let rgb = iris.truncate();
	[
		iris,
		(rgb * Vec3::new(0.07, 0.05, 0.04)).extend(1.0),
		(Vec3::new(0.93, 0.91, 0.88) + rgb * 0.045).extend(1.0),
		Vec4::new(0.98, 0.99, 0.97, 1.0),
		(rgb * Vec3::new(0.38, 0.32, 0.28)).extend(1.0),
		// Warm skin, not an iris stain — lids should not pick up eye color.
		Vec4::new(0.72, 0.52, 0.44, 1.0),
	]
}

/// Registers embedded **`face_material.wgsl`** and [`MaterialPlugin`].
pub struct FaceShaderMaterialPlugin;

impl Plugin for FaceShaderMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "face_material.wgsl");
		app.add_plugins(MaterialPlugin::<FaceShaderMaterial>::default());
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceShaderKind {
	Eye,
	Mouth,
}

impl FaceShaderKind {
	pub const fn as_u32(self) -> u32 {
		match self {
			Self::Eye => KIND_EYE,
			Self::Mouth => KIND_MOUTH,
		}
	}

	pub fn from_recipe_name(name: &str) -> Self {
		if name == RECIPE_FACE_MOUTH {
			Self::Mouth
		} else {
			Self::Eye
		}
	}

	pub fn is_face_recipe(name: &str) -> bool {
		matches!(name, RECIPE_FACE_EYE | RECIPE_FACE_MOUTH)
	}
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct FaceMaterialUniform {
	pub colors: [Vec4; MATERIAL_PALETTE_SLOTS],
	/// `x` frequency, `y` amplitude, `z` seed, `w` octaves (unused by blink).
	pub noise: Vec4,
	pub scalars: [Vec4; SCALAR_VEC4S],
	pub rasters: [[Vec4; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS],
	pub kind: u32,
	pub flags: u32,
	pub _pad: UVec2,
}

impl FaceMaterialUniform {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let kind = match &material_ref.name {
			MaterialId::Name(name) => FaceShaderKind::from_recipe_name(name),
			MaterialId::Default => FaceShaderKind::Eye,
		};
		let fallback = match kind {
			FaceShaderKind::Eye => DEFAULT_EYE_COLOR,
			FaceShaderKind::Mouth => DEFAULT_MOUTH_COLOR,
		};

		let mut colors = [Vec4::ZERO; MATERIAL_PALETTE_SLOTS];
		let provided = if material_ref.palette.is_empty() {
			colors[0] = fallback;
			1
		} else {
			for (slot, color) in colors.iter_mut().zip(&material_ref.palette) {
				*slot = linear_vec4(*color);
			}
			material_ref.palette.len()
		};
		if kind == FaceShaderKind::Eye {
			let derived = derived_eye_palette(colors[EYE_PALETTE_IRIS]);
			for (slot, color) in colors.iter_mut().take(EYE_PALETTE_DETAIL_SLOTS).zip(derived).skip(provided) {
				*slot = color;
			}
		} else {
			let first = colors[0];
			for color in colors.iter_mut().skip(provided) {
				*color = first;
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

		let mut rasters = [[Vec4::ZERO; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS];
		for channel in 0..MATERIAL_RASTER_CHANNELS {
			let samples = material_ref.rasters.get_or(channel, 0.0);
			let rows = MaterialRasters::packed_rows(samples);
			rasters[channel] = rows.map(Vec4::from_array);
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
			kind: kind.as_u32(),
			flags: 0,
			_pad: UVec2::ZERO,
		}
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FaceShaderMaterial {
	#[uniform(0)]
	pub params: FaceMaterialUniform,
}

impl FaceShaderMaterial {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		Self { params: FaceMaterialUniform::from_material_ref(material_ref) }
	}
}

impl Default for FaceShaderMaterial {
	fn default() -> Self {
		Self::from_material_ref(&MaterialRef::named(RECIPE_FACE_EYE))
	}
}

impl Material for FaceShaderMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "face_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "face_material.wgsl").into()
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

/// `0` = open, `1` = closed. Fast close, slower open, occasional double.
///
/// Peak depth varies per cycle so most blinks are slighter than a full slit.
/// `seed` is `[0, 1)`. Must stay a designed 1D envelope — not raw 4D noise.
pub fn blink_envelope(time: f32, seed: f32) -> f32 {
	let seed = seed.rem_euclid(1.0);
	let period = blink_period(seed);
	let phase_time = time + seed * 17.0;
	let t = phase_time.rem_euclid(period) / period.max(1e-4);
	let cycle = (phase_time / period.max(1e-4)).floor();
	blink_shape(t, seed) * blink_depth(cycle, seed)
}

fn blink_period(seed: f32) -> f32 {
	3.4 + seed * 1.8
}

fn blink_shape(t: f32, seed: f32) -> f32 {
	let close = 0.016;
	let hold = 0.008;
	let open = 0.048;
	let first = blink_pulse(t, 0.0, close, hold, open);
	if seed > 0.62 {
		let second_start = close + hold + open + 0.018;
		first.max(blink_pulse(t, second_start, close, hold, open))
	} else {
		first
	}
}

fn blink_depth(cycle: f32, seed: f32) -> f32 {
	let h = hash11(cycle * 1.73 + seed * 9.1 + 2.4);
	if h < 0.58 {
		0.24 + h * 0.38
	} else if h < 0.86 {
		0.52 + (h - 0.58)
	} else {
		1.0
	}
}

fn hash11(n: f32) -> f32 {
	let x = (n * 0.1031).fract();
	(x * (x + 33.33)).fract()
}

fn blink_pulse(t: f32, start: f32, close: f32, hold: f32, open: f32) -> f32 {
	let u = t - start;
	if u <= 0.0 || u >= close + hold + open {
		return 0.0;
	}
	if u < close {
		return smoothstep(u / close);
	}
	if u < close + hold {
		return 1.0;
	}
	1.0 - smoothstep((u - close - hold) / open)
}

fn smoothstep(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

/// `0` = globe, `1` = lid. Almond aperture; blink grows lids toward the midline.
pub fn lid_wrap(xy: Vec2, blink: f32) -> f32 {
	let blink = blink.clamp(0.0, 1.0);
	let taper = (1.0 - (xy.x / 0.52).powi(2)).max(0.0).sqrt();
	let upper_edge = (0.15 * (1.0 - blink) - 0.06 * blink) * taper;
	let lower_edge = (0.30 * (1.0 - blink) - 0.06 * blink) * taper;
	let upper = smoothstep((xy.y - upper_edge) / 0.04);
	let lower = smoothstep((-xy.y - lower_edge) / 0.04);
	upper.max(lower)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_material_ref_packs_kind_and_palette() {
		let eye = FaceShaderMaterial::from_material_ref(
			&MaterialRef::named(RECIPE_FACE_EYE).with_palette([Color::srgb(0.0, 1.0, 0.0)]),
		);
		assert_eq!(eye.params.kind, KIND_EYE);
		assert!((eye.params.colors[EYE_PALETTE_IRIS].y - 1.0).abs() < 1e-5);
		let pupil = eye.params.colors[EYE_PALETTE_PUPIL];
		let iris = eye.params.colors[EYE_PALETTE_IRIS];
		assert!(pupil.x + pupil.y + pupil.z < (iris.x + iris.y + iris.z) * 0.2);
		assert!(eye.params.colors[EYE_PALETTE_SCLERA].x > 0.85);
		assert!(eye.params.colors[EYE_PALETTE_LID].x > 0.5);
		assert!(
			(eye.params.colors[EYE_PALETTE_LID].x - eye.params.colors[EYE_PALETTE_IRIS].x).abs()
				> 0.2
		);

		let explicit_pupil = FaceShaderMaterial::from_material_ref(
			&MaterialRef::named(RECIPE_FACE_EYE)
				.with_palette([Color::srgb(0.0, 1.0, 0.0), Color::srgb(1.0, 0.0, 0.0)]),
		);
		assert!((explicit_pupil.params.colors[EYE_PALETTE_PUPIL].x - 1.0).abs() < 1e-5);

		let mouth = FaceShaderMaterial::from_material_ref(&MaterialRef::named(RECIPE_FACE_MOUTH));
		assert_eq!(mouth.params.kind, KIND_MOUTH);
	}

	#[test]
	fn face_recipe_names() {
		assert!(FaceShaderKind::is_face_recipe(RECIPE_FACE_EYE));
		assert!(FaceShaderKind::is_face_recipe(RECIPE_FACE_MOUTH));
		assert!(!FaceShaderKind::is_face_recipe("clothing_cloth"));
	}

	#[test]
	fn blink_is_open_for_most_of_the_period() {
		let seed = 0.2;
		let mut closed = 0;
		let samples = 200;
		for i in 0..samples {
			let t = i as f32 * 0.025;
			if blink_envelope(t, seed) > 0.2 {
				closed += 1;
			}
		}
		assert!(closed < samples / 6, "blink should be a short pulse, closed={closed}");
	}

	fn time_at_phase(period: f32, seed: f32, phase: f32) -> f32 {
		(phase * period - seed * 17.0).rem_euclid(period)
	}

	#[test]
	fn blink_closes_faster_than_it_opens() {
		let seed = 0.1;
		let period = blink_period(seed);
		assert!(blink_shape(0.008, seed) > 0.2);
		assert!(blink_shape(0.016 + 0.008 + 0.04, seed) < 0.85);
		assert!(blink_envelope(time_at_phase(period, seed, 0.2), seed) == 0.0);
	}

	#[test]
	fn blink_double_only_for_high_seeds() {
		let quiet = 0.1;
		assert_eq!(blink_shape(0.2, quiet), 0.0);
		let seed = 0.8;
		let second = 0.016 + 0.008 + 0.048 + 0.018 + 0.008;
		assert!(blink_shape(second, seed) > 0.5);
	}

	#[test]
	fn blink_mixes_light_and_full_closes() {
		let mut light = 0;
		let mut full = 0;
		for i in 0..40 {
			let depth = blink_depth(i as f32, 0.3);
			if depth < 0.5 {
				light += 1;
			}
			if depth > 0.95 {
				full += 1;
			}
		}
		assert!(light > 10, "expected mixed light blinks, light={light}");
		assert!(full > 0, "expected occasional full blinks");
		assert!(full < light, "full blinks should be the minority");
	}

	#[test]
	fn lid_wrap_hoods_rest_and_covers_on_full_blink() {
		assert!(lid_wrap(Vec2::new(0.0, 0.0), 0.0) < 0.1, "pupil stays open at rest");
		assert!(lid_wrap(Vec2::new(0.0, 0.42), 0.0) > 0.9, "upper sclera is lid at rest");
		assert!(lid_wrap(Vec2::new(0.0, 0.0), 1.0) > 0.9, "full blink covers the globe");
	}
}
