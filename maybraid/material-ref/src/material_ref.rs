//! [`MaterialRef`] identity: named recipe + palette + noise.

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

/// Deferred material identity: recipe name, optional palette, and noise params.
///
/// Resolved by a [`crate::MaterialLib`] into a concrete Bevy [`bevy::prelude::Material`]
/// handle and inserted (typically as [`bevy::prelude::MeshMaterial3d`]).
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct MaterialRef {
	pub name: MaterialId,
	pub palette: Vec<Color>,
	pub noise: NoiseParams,
}

impl MaterialRef {
	pub fn new(name: MaterialId) -> Self {
		Self { name, palette: Vec::new(), noise: NoiseParams::default() }
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
}

/// BSN / ECS root fulfilled by [`crate::MaterialRefPlugin`] via a [`crate::MaterialLib`].
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct MaterialRefRoot(pub MaterialRef);

/// Marker: [`MaterialRefRoot`] has been fulfilled (material component inserted).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MaterialRefApplied;
