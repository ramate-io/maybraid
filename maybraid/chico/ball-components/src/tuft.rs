//! Tuft component ([RFC-183 §3.1.2.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/06-tufts/README.md)).
//!
//! A tuft is several **spear-like cones** sharing one anchor: each child mesh grows from the joint
//! origin and radiates outward (mostly upward). Orientation of the whole cluster is applied on the
//! root [`Transform`] only.

pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::mesh::primitives::{ConeAnchor, Meshable};
use bevy::prelude::*;
use procedural_common::FromScalarNoise;
use render_item::{CascadeChunk, RenderItem};

/// Golden-angle step for even azimuth spacing on the tuft hemisphere.
const GOLDEN_ANGLE: f32 = 2.399_963_229_728_653_32;

/// Unit directions for [`ChicoTuft::spear_count`] spears radiating from a shared origin (mostly +Y).
pub fn spear_directions(count: u32, seed: i32, max_tilt_radians: f32) -> Vec<Vec3> {
	let n = count.max(1);
	let phase = (seed as f32).mul_add(0.173, 0.0);
	(0..n)
		.map(|i| {
			let fi = i as f32;
			let azimuth = GOLDEN_ANGLE.mul_add(fi, phase);
			let tilt = max_tilt_radians * (0.55 + 0.45 * ((seed.wrapping_add(i as i32) as f32) * 0.31).sin().abs());
			Vec3::new(tilt.sin() * azimuth.cos(), tilt.cos(), tilt.sin() * azimuth.sin())
				.normalize_or_zero()
		})
		.collect()
}

/// Per-spear length multiplier in `[min, max]` (deterministic from seed).
pub fn spear_length_scale(index: u32, seed: i32, min: f32, max: f32) -> f32 {
	let t = ((seed.wrapping_add(index as i32) as f32) * 0.47).sin().abs();
	min + (max - min) * t
}

/// [`StandardMaterial`] tuft using explicit mesh materials (common default).
pub type ChicoTuftStd = ChicoTuft<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// Marker plus embedded material for one tuft cluster at a ball-stick joint.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ChicoTuft<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	/// Number of spear meshes sharing this anchor.
	pub spear_count: u32,
	/// Unit spear length before root [`Transform::scale`].
	pub spear_length: f32,
	/// Cone base radius in unit space (tip is a point).
	pub base_radius: f32,
	/// Max polar angle from world-up in the root's local space (radians).
	pub max_tilt_radians: f32,
	pub seed: i32,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for ChicoTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self {
			spear_count: 8,
			spear_length: 1.0,
			base_radius: 0.09,
			max_tilt_radians: 0.42,
			seed: 0,
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> FromScalarNoise for ChicoTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn from_scalar(seed_scalar: f32, _frequency: f32, _amplitude: f32, _octaves: u32) -> Self {
		Self { seed: seed_scalar as i32, ..Self::default() }
	}
}

impl<M: Material, S> RenderItem for ChicoTuft<M, S>
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
		let root = commands
			.spawn((
				self.clone(),
				cascade_chunk.clone(),
				transform,
				Visibility::default(),
			))
			.id();

		let tuft = self.clone();
		commands.queue(move |world: &mut World| {
			let spear_mesh = Cone {
				radius: tuft.base_radius,
				height: tuft.spear_length,
			}
			.mesh()
			.anchor(ConeAnchor::Base)
			.build();

			let mesh_handle = {
				let mut meshes = world.resource_mut::<Assets<Mesh>>();
				meshes.add(spear_mesh)
			};

			let material: MeshMaterial3d<M> = tuft.material.clone().into();
			let directions =
				spear_directions(tuft.spear_count, tuft.seed, tuft.max_tilt_radians);

			for (i, dir) in directions.into_iter().enumerate() {
				if dir.length_squared() < 1e-10 {
					continue;
				}
				let length_scale = spear_length_scale(i as u32, tuft.seed, 0.72, 1.0);
				let rotation = Quat::from_rotation_arc(Vec3::Y, dir);
				world.spawn((
					ChildOf(root),
					Mesh3d(mesh_handle.clone()),
					material.clone(),
					Transform::from_rotation(rotation).with_scale(Vec3::new(1.0, length_scale, 1.0)),
				));
			}
		});

		vec![root]
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn spear_directions_are_unit_and_mostly_up() -> Result<()> {
		let dirs = spear_directions(8, 42, 0.4);
		assert_eq!(dirs.len(), 8);
		for d in dirs {
			assert!((d.length() - 1.0).abs() < 1e-4);
			assert!(d.y > 0.5, "spears should bias upward: {d:?}");
		}
		Ok(())
	}

	#[test]
	fn same_seed_same_directions() -> Result<()> {
		let a = spear_directions(6, 7, 0.35);
		let b = spear_directions(6, 7, 0.35);
		for (da, db) in a.iter().zip(b.iter()) {
			assert_eq!(da, db);
		}
		Ok(())
	}
}
