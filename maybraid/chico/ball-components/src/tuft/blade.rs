//! **Blade tuft** — thin, flat, grass-like blades (sketch; [#217](https://github.com/ramate-io/maybraid/issues/217)).

use std::marker::PhantomData;

use bevy::prelude::*;
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use super::directions::CapDirections;
use super::prism::{PrismaticCluster, PrismaticElement};
use super::spawn::MergedTuft;

const SIDE_COUNT: u32 = 2;

/// [`StandardMaterial`] blade tuft (common default).
pub type BladeTuftStd = BladeTuft<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// CLI / noise-driven shape parameters for [`BladeTuft`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct BladeTuftShape {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 12))]
	pub blade_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.15))]
	pub blade_length: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.025))]
	pub blade_width: f32,
	/// Max polar angle from +Y (radians); keep small for columnar grass clumps.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.22))]
	pub max_tilt_radians: f32,
	/// Max radius (m) blade bases scatter from the anchor; `0` roots every blade at one
	/// point (the classic radiating cone), larger values read as a loose mound.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub base_spread: f32,
	/// Along-strand segment count (`1` = one straight section base→tip; higher = more kinks).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 2))]
	pub bend_segments: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.10))]
	pub noise_amplitude: f32,
	/// Sway cycles **per bend segment**; near `1.0` each segment kinks independently, lower
	/// keeps neighbouring segments correlated (smoother bow).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub noise_frequency: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
}

impl Default for BladeTuftShape {
	fn default() -> Self {
		Self {
			blade_count: 12,
			blade_length: 1.15,
			blade_width: 0.025,
			max_tilt_radians: 0.22,
			base_spread: 0.0,
			bend_segments: 2,
			noise_amplitude: 0.10,
			noise_frequency: 1.0,
			seed: 0,
		}
	}
}

/// Thin flat blades radiating from a shared anchor (sketch implementation via ribbon prisms).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BladeTuft<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub shape: BladeTuftShape,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for BladeTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self { shape: BladeTuftShape::default(), material: S::default(), __marker: PhantomData }
	}
}

impl<M: Material, S> FromScalarNoise for BladeTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn from_scalar(noise: NoiseParams) -> Self {
		Self {
			shape: BladeTuftShape {
				seed: noise.seed,
				noise_frequency: noise.frequency,
				noise_amplitude: noise.amplitude,
				..BladeTuftShape::default()
			},
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> BladeTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub fn from_shape(shape: BladeTuftShape, material: S) -> Self {
		Self { shape, material, __marker: PhantomData }
	}

	fn blade_directions(&self) -> Vec<Vec3> {
		CapDirections::upward(self.shape.blade_count, self.shape.seed, self.shape.max_tilt_radians)
	}

	fn blade_length_at(&self, index: u32, min: f32, max: f32, scale: f32) -> f32 {
		(self.shape.blade_length
			* CapDirections::length_scale(index, self.shape.seed, min, max)
			* scale)
			.max(1e-4)
	}

	/// Base offset for blade `index`: outward along the blade's own lean so spread blades
	/// root where they point, scattered up to `base_spread` from the anchor.
	fn blade_base_offset(&self, index: u32, direction: Vec3, scale: f32) -> Vec3 {
		let spread = self.shape.base_spread * scale;
		if spread <= 0.0 {
			return Vec3::ZERO;
		}
		let outward = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
		let radius = CapDirections::length_scale(index, self.shape.seed.wrapping_add(17), 0.2, 1.0);
		outward * (spread * radius)
	}

	pub fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		let scale = world_uniform_scale.max(1e-8);
		let half_width = (self.shape.blade_width * scale * 0.5).max(1e-6);
		let tip_width = half_width * 0.15;
		let noise_amplitude = self.shape.noise_amplitude * scale;

		let elements: Vec<PrismaticElement> = self
			.blade_directions()
			.into_iter()
			.enumerate()
			.map(|(i, direction)| PrismaticElement {
				direction,
				length: self.blade_length_at(i as u32, 0.78, 1.0, scale),
				base_radius: half_width,
				tip_radius: tip_width,
				seed: self.shape.seed.wrapping_add(i as i32),
				base_offset: self.blade_base_offset(i as u32, direction, scale),
			})
			.collect();

		PrismaticCluster::new(
			elements,
			self.shape.bend_segments,
			SIDE_COUNT,
			self.shape.noise_frequency,
			noise_amplitude,
		)
		.into_mesh()
	}
}

impl<M: Material, S> MergedTuft for BladeTuft<M, S>
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

impl<M: Material, S> RenderItem for BladeTuft<M, S>
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
	use bevy::mesh::VertexAttributeValues;

	/// Max horizontal distance of near-ground vertices from the anchor.
	fn base_root_radius(shape: BladeTuftShape) -> Result<f32> {
		let tuft = BladeTuft::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::from_shape(
			shape,
			MeshMaterial3d(Handle::default()),
		);
		let mesh = tuft.build_mesh(1.0);
		let Some(VertexAttributeValues::Float32x3(positions)) =
			mesh.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected f32x3 positions");
		};
		Ok(positions
			.iter()
			.filter(|p| p[1].abs() < 0.05)
			.map(|p| (p[0] * p[0] + p[2] * p[2]).sqrt())
			.fold(0.0, f32::max))
	}

	#[test]
	fn base_spread_scatters_blade_roots() -> Result<()> {
		let shape = BladeTuftShape { noise_amplitude: 0.0, ..BladeTuftShape::default() };
		let anchored = base_root_radius(shape.clone())?;
		let spread = base_root_radius(BladeTuftShape { base_spread: 0.3, ..shape })?;
		assert!(anchored < 0.05, "zero spread should root at the anchor, got {anchored}");
		assert!(spread > 0.05, "spread blades should root away from the anchor, got {spread}");
		Ok(())
	}
}
