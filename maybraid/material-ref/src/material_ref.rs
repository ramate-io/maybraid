//! [`MaterialRef`] identity: named recipe + palette + noise + rasters + scalars.

use bevy::prelude::{Color, Component};
use procedural_common::NoiseParams;

/// Which material recipe a [`MaterialRef`] asks a [`crate::MaterialLib`] to build.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum MaterialId {
	/// Library default recipe.
	#[default]
	Default,
	/// Named recipe (e.g. `"tuft_leaf"`). Interpreted by the active [`crate::MaterialLib`].
	Name(String),
}

impl MaterialId {
	pub fn named(name: impl Into<String>) -> Self {
		Self::Name(name.into())
	}
}

/// Neighborhood raster width shared by every material recipe (3×3 cell profiles).
pub const MATERIAL_RASTER_WIDTH: usize = 3;
/// Flattened 3×3 sample count.
pub const MATERIAL_RASTER_SAMPLES: usize = MATERIAL_RASTER_WIDTH * MATERIAL_RASTER_WIDTH;
/// GPU channel count. Unused channels are zero. Shader / [`MaterialId`] own index meaning.
pub const MATERIAL_RASTER_CHANNELS: usize = 8;
/// Scalar pad packed as eight `vec4`s on the GPU.
pub const MATERIAL_SCALAR_FLOATS: usize = 32;
/// Palette slots packed into the GPU uniform (CPU palettes may be shorter).
pub const MATERIAL_PALETTE_SLOTS: usize = 8;

/// Named 3×3 neighborhood channels. Channel indices are a shader contract, not string keys.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MaterialRasters {
	channels: Vec<[f32; MATERIAL_RASTER_SAMPLES]>,
}

impl MaterialRasters {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn set(&mut self, index: usize, samples: [f32; MATERIAL_RASTER_SAMPLES]) {
		if index >= MATERIAL_RASTER_CHANNELS {
			return;
		}
		if self.channels.len() <= index {
			self.channels.resize(index + 1, [0.0; MATERIAL_RASTER_SAMPLES]);
		}
		self.channels[index] = samples;
	}

	pub fn with(mut self, index: usize, samples: [f32; MATERIAL_RASTER_SAMPLES]) -> Self {
		self.set(index, samples);
		self
	}

	pub fn get(&self, index: usize) -> Option<[f32; MATERIAL_RASTER_SAMPLES]> {
		self.channels.get(index).copied()
	}

	pub fn get_or(&self, index: usize, default_value: f32) -> [f32; MATERIAL_RASTER_SAMPLES] {
		self.get(index).unwrap_or([default_value; MATERIAL_RASTER_SAMPLES])
	}

	pub fn iter(&self) -> impl Iterator<Item = (usize, [f32; MATERIAL_RASTER_SAMPLES])> + '_ {
		self.channels.iter().copied().enumerate()
	}

	pub fn len(&self) -> usize {
		self.channels.len()
	}

	pub fn is_empty(&self) -> bool {
		self.channels.is_empty()
	}

	/// `vec4`-padded rows for one channel (`xyz` samples, `w` unused).
	pub fn packed_rows(
		samples: [f32; MATERIAL_RASTER_SAMPLES],
	) -> [[f32; 4]; MATERIAL_RASTER_WIDTH] {
		[
			[samples[0], samples[1], samples[2], 0.0],
			[samples[3], samples[4], samples[5], 0.0],
			[samples[6], samples[7], samples[8], 0.0],
		]
	}
}

/// Material-level scalars packed into a fixed GPU pad. Index meaning is per-shader.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MaterialScalars {
	values: Vec<f32>,
}

impl MaterialScalars {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn from_values(values: impl IntoIterator<Item = f32>) -> Self {
		let mut values: Vec<f32> = values.into_iter().collect();
		values.truncate(MATERIAL_SCALAR_FLOATS);
		Self { values }
	}

	pub fn as_slice(&self) -> &[f32] {
		&self.values
	}

	pub fn get(&self, index: usize) -> Option<f32> {
		self.values.get(index).copied()
	}

	pub fn is_empty(&self) -> bool {
		self.values.is_empty()
	}

	pub fn len(&self) -> usize {
		self.values.len()
	}
}

/// Deferred material identity: recipe name, palette, noise, neighborhood rasters, and scalars.
///
/// Resolved by a [`crate::MaterialLib`] into a concrete Bevy [`bevy::prelude::Material`]
/// handle and inserted (typically as [`bevy::prelude::MeshMaterial3d`]).
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct MaterialRef {
	pub name: MaterialId,
	pub palette: Vec<Color>,
	pub noise: NoiseParams,
	pub rasters: MaterialRasters,
	pub scalars: MaterialScalars,
}

impl MaterialRef {
	pub fn new(name: MaterialId) -> Self {
		Self {
			name,
			palette: Vec::new(),
			noise: NoiseParams::default(),
			rasters: MaterialRasters::default(),
			scalars: MaterialScalars::default(),
		}
	}

	pub fn default_material() -> Self {
		Self::new(MaterialId::Default)
	}

	pub fn named(name: impl Into<String>) -> Self {
		Self::new(MaterialId::named(name))
	}

	pub fn with_palette(mut self, palette: impl IntoIterator<Item = Color>) -> Self {
		self.palette = palette.into_iter().collect();
		self
	}

	pub fn with_noise(mut self, noise: NoiseParams) -> Self {
		self.noise = noise;
		self
	}

	pub fn with_raster(mut self, index: usize, samples: [f32; MATERIAL_RASTER_SAMPLES]) -> Self {
		self.rasters.set(index, samples);
		self
	}

	pub fn with_rasters(mut self, rasters: MaterialRasters) -> Self {
		self.rasters = rasters;
		self
	}

	pub fn with_scalars(mut self, values: impl IntoIterator<Item = f32>) -> Self {
		self.scalars = MaterialScalars::from_values(values);
		self
	}

	pub fn raster(&self, index: usize) -> Option<[f32; MATERIAL_RASTER_SAMPLES]> {
		self.rasters.get(index)
	}

	pub fn scalar_values(&self) -> &[f32] {
		self.scalars.as_slice()
	}
}

/// BSN / ECS root fulfilled by [`crate::MaterialRefPlugin`] via a [`crate::MaterialLib`].
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct MaterialRefRoot(pub MaterialRef);

/// Opt-in: apply this root’s [`MaterialRef`] to `Mesh3d` entities under it (and to self if
/// the root also has `Mesh3d`).
///
/// Without this marker, fulfill inserts the material only on the [`MaterialRefRoot`] entity.
/// Use for `WorldAsset` / GLB instances whose meshes spawn as descendants.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PropagateToDescendants;

/// Marker: [`MaterialRefRoot`] has been fulfilled (material component inserted), or a
/// propagating root has been registered / a descendant mesh has been fulfilled.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MaterialRefApplied;
