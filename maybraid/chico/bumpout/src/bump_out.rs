use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::NoAutoAabb;
use bevy::math::Vec3A;
use bevy::prelude::*;
use material_ref::{MaterialRef, MaterialRefRoot};
use procedural_common::NoiseParams;
use terrain_chunk_ref::TerrainChunkRef;

use crate::{BumpOutNeighborhood, BumpOutStyle, DENSITY_PARAMETER, HEIGHT_PARAMETER};

/// A visual terrain overlay with conservative vertical displacement bounds.
#[derive(Component, Debug, Clone)]
pub struct BumpOut {
	pub material: MaterialRef,
	pub min_vertical_displacement: f32,
	pub max_vertical_displacement: f32,
}

impl BumpOut {
	pub fn new(
		material: MaterialRef,
		min_vertical_displacement: f32,
		max_vertical_displacement: f32,
	) -> Self {
		Self {
			material,
			min_vertical_displacement: min_vertical_displacement.min(max_vertical_displacement),
			max_vertical_displacement: max_vertical_displacement.max(min_vertical_displacement),
		}
	}

	pub fn from_neighborhood(
		neighborhood: BumpOutNeighborhood,
		palette: impl IntoIterator<Item = Color>,
		noise: NoiseParams,
	) -> Self {
		let amplitude = noise.amplitude.abs();
		let min_height = neighborhood.min_height() - amplitude;
		let max_height = neighborhood.max_height() + amplitude;
		Self::new(neighborhood.material_ref(palette, noise), min_height, max_height)
	}

	pub fn with_style(mut self, style: BumpOutStyle) -> Self {
		self.set_style(style);
		self
	}

	pub fn neighborhood(&self) -> BumpOutNeighborhood {
		BumpOutNeighborhood::from_material_ref(&self.material)
	}

	pub fn style(&self) -> BumpOutStyle {
		BumpOutStyle::from_material_ref(&self.material)
	}

	pub fn set_neighborhood(&mut self, neighborhood: BumpOutNeighborhood) {
		self.material.parameters.insert(DENSITY_PARAMETER, neighborhood.densities);
		self.material.parameters.insert(HEIGHT_PARAMETER, neighborhood.heights);
		let amplitude = self.material.noise.amplitude.abs();
		self.min_vertical_displacement = neighborhood.min_height() - amplitude;
		self.max_vertical_displacement = neighborhood.max_height() + amplitude;
	}

	pub fn set_style(&mut self, style: BumpOutStyle) {
		self.material = style.apply_to(self.material.clone());
	}

	pub fn aabb<T>(&self, terrain_ref: &TerrainChunkRef<T>) -> Aabb {
		let extent = terrain_ref.chunk.extent();
		let local_min = Vec3::new(0.0, self.min_vertical_displacement.min(0.0), 0.0);
		let local_max = extent + Vec3::new(0.0, self.max_vertical_displacement.max(0.0), 0.0);
		Aabb {
			center: Vec3A::from((local_min + local_max) * 0.5),
			half_extents: Vec3A::from((local_max - local_min) * 0.5),
		}
	}

	/// Spawn an independently materialized presenter that lazily resolves `terrain_ref`.
	pub fn spawn<T>(self, commands: &mut Commands, terrain_ref: TerrainChunkRef<T>) -> Entity
	where
		T: Send + Sync + 'static,
	{
		let aabb = self.aabb(&terrain_ref);
		let transform = terrain_ref.transform();
		let material = self.material.clone();

		commands
			.spawn((
				self,
				terrain_ref,
				MaterialRefRoot(material),
				transform,
				Visibility::default(),
				aabb,
				NoAutoAabb,
			))
			.id()
	}
}
