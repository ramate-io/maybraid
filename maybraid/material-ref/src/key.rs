//! Bit-stable cache keys for [`MaterialRef`](crate::MaterialRef).

use std::hash::{Hash, Hasher};

use bevy::asset::Asset;
use bevy::color::{Color, ColorToComponents};
use bevy::platform::collections::HashMap;
use bevy::prelude::{Handle, Resource};
use procedural_common::{NoiseParams, NoiseType};

use crate::material_ref::{MaterialId, MaterialRef, MATERIAL_RASTER_SAMPLES};

/// Hashable / equality key for memoizing resolved material handles.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaterialRefKey {
	pub name: MaterialId,
	pub palette: Vec<[u32; 4]>,
	pub noise: NoiseParamsKey,
	pub rasters: Vec<[u32; MATERIAL_RASTER_SAMPLES]>,
	pub scalars: Vec<u32>,
}

impl From<&MaterialRef> for MaterialRefKey {
	fn from(r: &MaterialRef) -> Self {
		Self {
			name: r.name.clone(),
			palette: r.palette.iter().map(color_bits).collect(),
			noise: NoiseParamsKey::from(&r.noise),
			rasters: r
				.rasters
				.iter()
				.map(|(_, samples)| {
					let mut bits = [0u32; MATERIAL_RASTER_SAMPLES];
					for (slot, value) in bits.iter_mut().zip(samples) {
						*slot = value.to_bits();
					}
					bits
				})
				.collect(),
			scalars: r.scalars.as_slice().iter().map(|value| value.to_bits()).collect(),
		}
	}
}

/// Bit-stable [`NoiseParams`] for cache keys (`NoiseParams` is not [`Hash`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoiseParamsKey {
	pub seed: i32,
	pub frequency_bits: u32,
	pub amplitude_bits: u32,
	pub octaves: u32,
	pub noise_type: u8,
}

impl From<&NoiseParams> for NoiseParamsKey {
	fn from(n: &NoiseParams) -> Self {
		Self {
			seed: n.seed,
			frequency_bits: n.frequency.to_bits(),
			amplitude_bits: n.amplitude.to_bits(),
			octaves: n.octaves,
			noise_type: noise_type_tag(n.noise_type),
		}
	}
}

fn noise_type_tag(t: NoiseType) -> u8 {
	match t {
		NoiseType::OpenSimplex2 => 0,
		NoiseType::OpenSimplex2S => 1,
		NoiseType::Cellular => 2,
		NoiseType::Perlin => 3,
		NoiseType::ValueCubic => 4,
		NoiseType::Value => 5,
	}
}

fn color_bits(c: &Color) -> [u32; 4] {
	let linear = c.to_linear();
	let [r, g, b, a] = linear.to_f32_array();
	[r.to_bits(), g.to_bits(), b.to_bits(), a.to_bits()]
}

/// Shared handle cache keyed by [`MaterialRefKey`].
#[derive(Resource, Debug)]
pub struct MaterialRefCache<M: Asset> {
	map: HashMap<MaterialRefKey, Handle<M>>,
}

impl<M: Asset> Default for MaterialRefCache<M> {
	fn default() -> Self {
		Self { map: HashMap::default() }
	}
}

impl<M: Asset> MaterialRefCache<M> {
	pub fn get(&self, key: &MaterialRefKey) -> Option<Handle<M>> {
		self.map.get(key).cloned()
	}

	pub fn insert(&mut self, key: MaterialRefKey, handle: Handle<M>) {
		self.map.insert(key, handle);
	}

	pub fn len(&self) -> usize {
		self.map.len()
	}

	pub fn is_empty(&self) -> bool {
		self.map.is_empty()
	}
}

/// Stable hash helper for tests / callers that want a fingerprint without owning a map.
pub fn hash_material_ref(r: &MaterialRef) -> u64 {
	use std::collections::hash_map::DefaultHasher;
	let mut h = DefaultHasher::new();
	MaterialRefKey::from(r).hash(&mut h);
	h.finish()
}
