//! Persistent field: compact descriptors, not expanded meshes.

use bevy::prelude::*;
use vegetation_shaders::wesl::{BladeDescriptor, GrassPatchHeader, WindInputs};

use crate::bounds::GrassBounds;
use crate::patch::GrassPatch;

/// Tunables for GPU list capacities and wind. Authoring budgets live here, not in WESL.
#[derive(Resource, Clone, Debug)]
pub struct GrassFieldSettings {
	pub max_blades: u32,
	pub max_per_lod: u32,
	pub wind: WindInputs,
	pub diagnostics: bool,
	/// When set, select/draw use this time instead of `Time` (deterministic captures).
	pub freeze_time_s: Option<f32>,
}

impl GrassFieldSettings {
	pub fn from_env() -> Self {
		let mut settings = Self::default();
		settings.diagnostics = std::env::var_os("MAYBRAID_GRASS_DIAGNOSTICS").is_some();
		settings
	}

	pub fn keep_fraction(&self, total_blades: u32) -> f32 {
		if total_blades == 0 {
			return 1.0;
		}
		(self.max_per_lod as f32 / total_blades as f32).min(1.0)
	}
}

impl Default for GrassFieldSettings {
	fn default() -> Self {
		Self {
			max_blades: 131_072,
			max_per_lod: 65_536,
			wind: WindInputs::gentle_botw(0.0),
			diagnostics: false,
			freeze_time_s: None,
		}
	}
}

/// CPU mirror of the persistent GPU blade/patch buffers.
#[derive(Resource, Clone, Debug, Default)]
pub struct GrassField {
	pub blades: Vec<BladeDescriptor>,
	pub patches: Vec<GrassPatchHeader>,
	pub bounds: GrassBounds,
	pub generation: u64,
}

impl GrassField {
	pub fn rebuild<'a>(
		patches: impl IntoIterator<Item = &'a GrassPatch>,
		settings: &GrassFieldSettings,
	) -> Self {
		let mut blades = Vec::new();
		let mut headers = Vec::new();
		let mut bounds = GrassBounds::empty();
		for patch in patches {
			if blades.len() as u32 >= settings.max_blades {
				break;
			}
			let mut desc = patch.descriptors();
			let remaining = settings.max_blades as usize - blades.len();
			if desc.len() > remaining {
				desc.truncate(remaining);
			}
			let header = patch.header(blades.len() as u32, desc.len() as u32, settings);
			bounds = bounds.union(patch.bounds(settings));
			blades.extend(desc);
			headers.push(header);
		}
		Self { blades, patches: headers, bounds, generation: 0 }
	}

	pub fn with_generation(mut self, generation: u64) -> Self {
		self.generation = generation;
		self
	}

	pub fn is_empty(&self) -> bool {
		self.blades.is_empty()
	}
}
