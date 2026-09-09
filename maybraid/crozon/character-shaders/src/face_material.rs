//! Face [`Material`] — blink squash and a few millimetres of mouth idle.
//!
//! Phase is `globals.time` + instance seed. No mailbox. No blendshapes.

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

const SCALAR_VEC4S: usize = MATERIAL_SCALAR_FLOATS / 4;
const DEFAULT_EYE_COLOR: Vec4 = Vec4::new(0.22, 0.16, 0.12, 1.0);
const DEFAULT_MOUTH_COLOR: Vec4 = Vec4::new(0.62, 0.32, 0.30, 1.0);

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
		if material_ref.palette.is_empty() {
			colors[0] = fallback;
		} else {
			for (slot, color) in colors.iter_mut().zip(&material_ref.palette) {
				let linear = LinearRgba::from(*color);
				*slot = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
			}
			let first = colors[0];
			for color in colors.iter_mut().skip(material_ref.palette.len()) {
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

/// `0` = open, `1` = closed. Fast close, slower open, long hold, occasional double.
///
/// `seed` is `[0, 1)`. Must stay a designed 1D envelope — not raw 4D noise.
pub fn blink_envelope(time: f32, seed: f32) -> f32 {
	let seed = seed.rem_euclid(1.0);
	let period = 3.4 + seed * 1.8;
	let t = (time + seed * 17.0).rem_euclid(period) / period.max(1e-4);
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_material_ref_packs_kind_and_palette() {
		let eye = FaceShaderMaterial::from_material_ref(
			&MaterialRef::named(RECIPE_FACE_EYE).with_palette([Color::srgb(0.0, 1.0, 0.0)]),
		);
		assert_eq!(eye.params.kind, KIND_EYE);
		assert!((eye.params.colors[0].y - 1.0).abs() < 1e-5);

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
		let period = 3.4 + seed * 1.8;
		assert!(blink_envelope(time_at_phase(period, seed, 0.008), seed) > 0.2);
		assert!(blink_envelope(time_at_phase(period, seed, 0.016 + 0.008 + 0.04), seed) < 0.85);
	}

	#[test]
	fn blink_double_only_for_high_seeds() {
		let quiet = 0.1;
		let quiet_period = 3.4 + quiet * 1.8;
		assert_eq!(blink_envelope(time_at_phase(quiet_period, quiet, 0.2), quiet), 0.0);
		let seed = 0.8;
		let period = 3.4 + seed * 1.8;
		let second = 0.016 + 0.008 + 0.048 + 0.018 + 0.008;
		assert!(blink_envelope(time_at_phase(period, seed, second), seed) > 0.5);
	}
}
