//! **Chico stick**: noisy tapered cylinder along +Y for ball-stick **segment** meshes.
//!
//! # Role in Sope's Banyan ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252))
//!
//! Sticks are the mesh primitive for each graph edge between parent and child nodes once a `BallStickChain` (and render helpers in `chico-sbs-geometry`) supply segment transforms. Each stick carries a material source `S` convertible via [`Into`] into [`MeshMaterial3d`]` `<`[`Material`]`>` so tree assemblies (`chico-sbs-trees`) can embed bark-facing handles alongside procedural noise.

pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sdf::{NoisySurface, TaperedCylinder};
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{mesh::handle::Cached, CascadeChunk, RenderItem};

/// [`StandardMaterial`] stick using explicit mesh materials (common default).
pub type ChicoStickStd = ChicoStick<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// First-pass stick marker plus embedded render material (via [`Into`]).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ChicoStick<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub seed_scalar: f32,
	pub frequency: f32,
	pub amplitude: f32,
	pub octaves: u32,
	/// Converts into [`MeshMaterial3d`] at spawn (see [`Into`]).
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for ChicoStick<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self {
			seed_scalar: 0.0,
			frequency: 1.0,
			amplitude: 1.0,
			octaves: 1,
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> FromScalarNoise for ChicoStick<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn from_scalar(seed_scalar: f32, frequency: f32, amplitude: f32, octaves: u32) -> Self {
		Self {
			seed_scalar,
			frequency,
			amplitude,
			octaves,
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> ChicoStick<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	/// Unit-height tapered segment with surface noise from [`FromScalarNoise`] fields (RFC stick / trunk convention).
	pub fn noisy_cylinder(&self) -> chico_sdf::NoisyCylinder {
		NoisySurface::from_params(
			TaperedCylinder::unit_segment(0.5, 0.4),
			NoiseParams::from_scalar(
				self.seed_scalar,
				self.frequency,
				self.amplitude,
				self.octaves,
			),
		)
	}
}

impl<M: Material, S> RenderItem for ChicoStick<M, S>
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
		// noisy_cylinder is authored from lower-left:
		// local x: [0, scale.x]
		// local y: [0, scale.y]
		// local z: [0, scale.z]
		//
		// We want the provided transform.translation to refer to the xz centroid
		// at the stick's lower anchor, then offset to the mesh's authored origin.
		let local_offset =
			Vec3::new(-0.5 * transform.scale.x, -0.5 * transform.scale.y, -0.5 * transform.scale.z);

		let centroid_offset = transform.rotation * local_offset;
		let translation = transform.translation + centroid_offset;

		let mesh_material: MeshMaterial3d<M> = self.material.clone().into();

		vec![commands
			.spawn((
				self.clone(),
				Cached::new(self.noisy_cylinder()),
				cascade_chunk.clone(),
				transform.with_translation(translation),
				mesh_material,
			))
			.id()]
	}
}
