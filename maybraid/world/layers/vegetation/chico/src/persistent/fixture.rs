//! Explicit, shared one-tile benchmark content; renderer choice never selects trees.

use bevy::prelude::*;
use bevy::shape::Aabb3d;
use lod::gen::{Id, OriginalId};
use vegetation_groves::GroveExtent;

use crate::{grove_id, ForestGroveKind, ForestGroveRecipe, ForestLayer};

pub const ENV_TILE: &str = "MAYBRAID_VEGETATION_INSTANCE_FIXTURE_TILE";
pub const ENV_MODE: &str = "MAYBRAID_VEGETATION_INSTANCE_MODE";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FixtureMode {
	#[default]
	Ordinary,
	Persistent,
}

/// Immutable startup configuration, seeded into the same HCSG session as selection.
#[derive(Clone, Copy, Resource, Default)]
pub struct VegetationInstanceFixture {
	pub tile_min: Option<Vec2>,
	pub mode: FixtureMode,
	pub canopy_shadows: vegetation_components::CanopyShadowMode,
}

impl VegetationInstanceFixture {
	pub fn parse(tile: Option<&str>, mode: Option<&str>) -> Result<Self, String> {
		let mode = match mode.unwrap_or("ordinary") {
			"ordinary" => FixtureMode::Ordinary,
			"persistent" => FixtureMode::Persistent,
			other => return Err(format!("{ENV_MODE}: invalid mode {other:?}")),
		};
		let Some(tile) = tile else {
			if mode == FixtureMode::Persistent {
				return Err(format!("{ENV_MODE}=persistent requires {ENV_TILE}"));
			}
			return Ok(Self::default());
		};
		let values = tile
			.split(',')
			.map(str::trim)
			.map(str::parse::<f32>)
			.collect::<Result<Vec<_>, _>>()
			.map_err(|error| format!("{ENV_TILE}: {error}"))?;
		if values.len() != 2
			|| values.iter().any(|value| {
				!value.is_finite()
					|| value.rem_euclid(100.0) != 0.0
					|| (*value + 100.0) - *value != 100.0
			}) {
			return Err(format!("{ENV_TILE}: expected finite 100m-aligned X,Z tile minimum"));
		}
		Ok(Self {
			tile_min: Some(Vec2::new(values[0], values[1])),
			mode,
			canopy_shadows: vegetation_components::CanopyShadowMode::Proxy,
		})
	}

	pub fn from_env() -> Self {
		let mut fixture = Self::parse(
			std::env::var(ENV_TILE).ok().as_deref(),
			std::env::var(ENV_MODE).ok().as_deref(),
		)
		.unwrap_or_else(|error| {
			eprintln!("{error}");
			std::process::exit(2)
		});
		fixture.canopy_shadows = vegetation_components::CanopyShadowMode::from_env();
		fixture
	}

	pub fn extent(self) -> Option<GroveExtent> {
		let min = self.tile_min?;
		Some(GroveExtent::new(
			Vec3::new(min.x, 0.0, min.y),
			Vec3::new(min.x + 100.0, 1.0, min.y + 100.0),
		))
	}

	pub fn selects(self, extent: GroveExtent, layer: ForestLayer) -> bool {
		layer == ForestLayer::UpperCanopy
			&& self.extent().is_some_and(|selected| {
				selected.min() == extent.min() && selected.max() == extent.max()
			})
	}

	pub fn recipe(self, extent: GroveExtent, layer: ForestLayer) -> Option<ForestGroveRecipe> {
		self.selects(extent, layer)
			.then(|| ForestGroveRecipe::uniform(ForestGroveKind::Orchard, extent))
	}

	pub fn include_id(self, ids: &mut Vec<OriginalId>, region: Aabb3d) {
		let Some(extent) = self.extent() else { return };
		if extent.min().x >= region.max.x
			|| extent.max().x <= region.min.x
			|| extent.min().z >= region.max.z
			|| extent.max().z <= region.min.z
		{
			return;
		}
		let id = OriginalId(grove_id(extent, ForestLayer::UpperCanopy));
		if !ids.contains(&id) {
			ids.push(id);
		}
	}

	pub fn id(self) -> Option<Id> {
		self.extent().map(|extent| grove_id(extent, ForestLayer::UpperCanopy))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn selector_is_explicit_finite_aligned_and_renderer_independent() -> anyhow::Result<()> {
		assert!(VegetationInstanceFixture::parse(None, Some("persistent")).is_err());
		for invalid in ["0", "1,0", "NaN,0", "inf,0", "0,0,0"] {
			assert!(VegetationInstanceFixture::parse(Some(invalid), None).is_err());
		}
		let ordinary = VegetationInstanceFixture::parse(Some("0,-100"), Some("ordinary"))
			.map_err(anyhow::Error::msg)?;
		let persistent = VegetationInstanceFixture::parse(Some("0,-100"), Some("persistent"))
			.map_err(anyhow::Error::msg)?;
		assert_eq!(ordinary.id(), persistent.id());
		let extent = ordinary.extent().ok_or_else(|| anyhow::anyhow!("fixture absent"))?;
		assert_eq!(
			ordinary.recipe(extent, ForestLayer::UpperCanopy).map(|recipe| recipe.kind),
			Some(ForestGroveKind::Orchard)
		);
		assert!(ordinary.recipe(extent, ForestLayer::LowerCanopy).is_none());
		let region = Aabb3d::from_min_max(Vec3::splat(-200.0), Vec3::splat(200.0));
		let mut ids = Vec::new();
		ordinary.include_id(&mut ids, region);
		ordinary.include_id(&mut ids, region);
		assert_eq!(ids.len(), 1);
		let mut excluded = Vec::new();
		ordinary.include_id(
			&mut excluded,
			Aabb3d::from_min_max(Vec3::splat(300.0), Vec3::splat(400.0)),
		);
		assert!(excluded.is_empty());
		Ok(())
	}
}
