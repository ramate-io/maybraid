//! **Weeping tuft** — upward curving bush clump (sketch; [#217](https://github.com/ramate-io/maybraid/issues/217)).
//!
//! **Note:** Current implementation still uses downward draping geometry. Intended semantics
//! (palm-bush style upward curving tufts) are tracked in
//! `issues/chico-tufts/weeping-semantics/` — defer aesthetic work until that redesign lands.

use std::marker::PhantomData;

use bevy::prelude::*;
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use super::directions::CapDirections;
use super::prism::{PrismaticCluster, PrismaticElement};
use super::spawn::MergedTuft;

const HEIGHT_SEGMENTS: u32 = 5;
const SIDE_COUNT: u32 = 3;

/// [`StandardMaterial`] weeping tuft (common default).
pub type WeepingTuftStd = WeepingTuft<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// CLI / noise-driven shape parameters for [`WeepingTuft`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct WeepingTuftShape {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 7))]
	pub strand_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.25))]
	pub strand_length: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.04))]
	pub strand_radius: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.18))]
	pub tip_radius_fraction: f32,
	/// How far strands point below horizontal (radians).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.55))]
	pub downward_tilt_radians: f32,
	/// Outward fan angle from the vertical (radians).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.45))]
	pub outward_spread_radians: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.12))]
	pub noise_amplitude: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 3.5))]
	pub noise_frequency: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
}

impl Default for WeepingTuftShape {
	fn default() -> Self {
		Self {
			strand_count: 7,
			strand_length: 1.25,
			strand_radius: 0.04,
			tip_radius_fraction: 0.18,
			downward_tilt_radians: 0.55,
			outward_spread_radians: 0.45,
			noise_amplitude: 0.12,
			noise_frequency: 3.5,
			seed: 0,
		}
	}
}

/// Downward drooping strand cluster (sketch implementation).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct WeepingTuft<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub shape: WeepingTuftShape,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for WeepingTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self { shape: WeepingTuftShape::default(), material: S::default(), __marker: PhantomData }
	}
}

impl<M: Material, S> FromScalarNoise for WeepingTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn from_scalar(noise: NoiseParams) -> Self {
		Self {
			shape: WeepingTuftShape {
				seed: noise.seed,
				noise_frequency: noise.frequency,
				noise_amplitude: noise.amplitude,
				..WeepingTuftShape::default()
			},
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> WeepingTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub fn from_shape(shape: WeepingTuftShape, material: S) -> Self {
		Self { shape, material, __marker: PhantomData }
	}

	fn strand_directions(&self) -> Vec<Vec3> {
		CapDirections::weeping(
			self.shape.strand_count,
			self.shape.seed,
			self.shape.downward_tilt_radians,
			self.shape.outward_spread_radians,
		)
	}

	fn strand_length_at(&self, index: u32, min: f32, max: f32, scale: f32) -> f32 {
		(self.shape.strand_length
			* CapDirections::length_scale(index, self.shape.seed, min, max)
			* scale)
			.max(1e-4)
	}

	pub fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		let scale = world_uniform_scale.max(1e-8);
		let base_radius = (self.shape.strand_radius * scale).max(1e-6);
		let tip_radius = (base_radius * self.shape.tip_radius_fraction).max(0.0);
		let noise_amplitude = self.shape.noise_amplitude * scale;

		let elements: Vec<PrismaticElement> = self
			.strand_directions()
			.into_iter()
			.enumerate()
			.map(|(i, direction)| PrismaticElement {
				direction,
				length: self.strand_length_at(i as u32, 0.85, 1.15, scale),
				base_radius,
				tip_radius,
				seed: self.shape.seed.wrapping_add(i as i32),
				base_offset: Vec3::ZERO,
			})
			.collect();

		PrismaticCluster::new_draping(
			elements,
			HEIGHT_SEGMENTS,
			SIDE_COUNT,
			self.shape.noise_frequency,
			noise_amplitude,
		)
		.into_mesh()
	}
}

impl<M: Material, S> MergedTuft for WeepingTuft<M, S>
where
	M: Send + Sync + 'static,
	S: Clone + Into<MeshMaterial3d<M>> + Send + Sync + 'static,
{
	type Mat = M;
	type MatSlot = S;

	fn material_slot(&self) -> Self::MatSlot {
		self.material.clone()
	}

	fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		Self::build_mesh(self, world_uniform_scale)
	}
}

impl<M: Material, S> RenderItem for WeepingTuft<M, S>
where
	M: Send + Sync + 'static,
	S: Clone + Into<MeshMaterial3d<M>> + Send + Sync + 'static,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.spawn_render_entities(commands, cascade_chunk, transform)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn weeping_directions_point_downward() -> Result<()> {
		let tuft = WeepingTuft::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::default();
		for d in tuft.strand_directions() {
			assert!(d.y < -0.2, "strands should droop: {d:?}");
		}
		Ok(())
	}

	#[test]
	fn weeping_rotation_maps_local_y_to_direction() -> Result<()> {
		use super::super::prism::PrismaticElement;

		let tuft = WeepingTuft::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::default();
		for (i, direction) in tuft.strand_directions().into_iter().enumerate() {
			let element = PrismaticElement {
				direction,
				length: 1.25,
				base_radius: 0.04,
				tip_radius: 0.01,
				seed: i as i32,
				base_offset: Vec3::ZERO,
			};
			let rotation = element.draping_rotation();
			let mapped = rotation * (-Vec3::Y);
			assert!(
				(mapped - direction).length() < 1e-4,
				"rotation should map −Y to direction; got {mapped:?} want {direction:?}"
			);
			let tip = rotation * Vec3::new(0.0, -element.length, 0.0);
			assert!(tip.y < -0.2, "tip should hang below anchor: {tip:?}");
		}
		Ok(())
	}

	#[test]
	fn weeping_mesh_extends_below_anchor() -> Result<()> {
		use bevy::mesh::VertexAttributeValues;

		let tuft = WeepingTuft::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::default();
		let mesh = tuft.build_mesh(1.0);
		let Some(VertexAttributeValues::Float32x3(positions)) =
			mesh.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("missing positions");
		};
		let max_y = positions.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
		let min_y = positions.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
		assert!(
			max_y < 0.15,
			"anchor should stay near origin; mesh max_y={max_y}"
		);
		assert!(
			min_y < -0.35,
			"strands should hang below anchor; mesh min_y={min_y}"
		);
		Ok(())
	}
}
