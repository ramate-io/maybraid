//! [`MaterialRef`] identity: named recipe + palette + noise + numeric parameter blocks.

use std::collections::BTreeMap;

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

/// Schema-stable named numeric parameter blocks for a material recipe.
///
/// A sorted map makes equality and cache identity independent of insertion order. Domain material
/// libraries own each block's meaning and GPU layout.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MaterialParameters {
	blocks: BTreeMap<String, Vec<f32>>,
}

impl MaterialParameters {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn insert(
		&mut self,
		name: impl Into<String>,
		values: impl IntoIterator<Item = f32>,
	) -> Option<Vec<f32>> {
		self.blocks.insert(name.into(), values.into_iter().collect())
	}

	pub fn with(mut self, name: impl Into<String>, values: impl IntoIterator<Item = f32>) -> Self {
		self.insert(name, values);
		self
	}

	pub fn get(&self, name: &str) -> Option<&[f32]> {
		self.blocks.get(name).map(Vec::as_slice)
	}

	pub fn iter(&self) -> impl Iterator<Item = (&str, &[f32])> {
		self.blocks.iter().map(|(name, values)| (name.as_str(), values.as_slice()))
	}

	pub fn is_empty(&self) -> bool {
		self.blocks.is_empty()
	}

	pub fn len(&self) -> usize {
		self.blocks.len()
	}
}

/// Deferred material identity: recipe name, optional palette, noise, and numeric parameters.
///
/// Resolved by a [`crate::MaterialLib`] into a concrete Bevy [`bevy::prelude::Material`]
/// handle and inserted (typically as [`bevy::prelude::MeshMaterial3d`]).
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct MaterialRef {
	pub name: MaterialId,
	pub palette: Vec<Color>,
	pub noise: NoiseParams,
	pub parameters: MaterialParameters,
}

impl MaterialRef {
	pub fn new(name: MaterialId) -> Self {
		Self {
			name,
			palette: Vec::new(),
			noise: NoiseParams::default(),
			parameters: MaterialParameters::default(),
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

	pub fn with_parameter(
		mut self,
		name: impl Into<String>,
		values: impl IntoIterator<Item = f32>,
	) -> Self {
		self.parameters.insert(name, values);
		self
	}

	pub fn with_parameters(mut self, parameters: MaterialParameters) -> Self {
		self.parameters = parameters;
		self
	}

	pub fn parameter(&self, name: &str) -> Option<&[f32]> {
		self.parameters.get(name)
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
