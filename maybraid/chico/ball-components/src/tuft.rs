//! Tuft components ([RFC-183 §3.1.2.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/06-tufts/README.md)).
//!
//! Several specialized tuft kinds share placement helpers and a low-poly prismatic mesh builder:
//!
//! - [`SucculentTuft`] — thick upward spears (dry conifers, succulents)
//! - [`BladeTuft`] — thin flat grass-like blades (sketch)
//! - [`WeepingTuft`] — downward drooping strands (sketch)

mod blade;
mod directions;
mod prism;
mod spawn;
mod succulent;
mod weeping;

pub mod render_item_plugin;

pub use blade::{BladeTuft, BladeTuftShape, BladeTuftStd};
pub use succulent::{SucculentTuft, SucculentTuftShape, SucculentTuftStd};
pub use weeping::{WeepingTuft, WeepingTuftShape, WeepingTuftStd};

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy::mesh::VertexAttributeValues;
	use bevy::prelude::*;
	use spawn::TuftSpawnTransform;

	fn test_succulent(seed: i32, amplitude: f32) -> SucculentTuft<StandardMaterial, MeshMaterial3d<StandardMaterial>> {
		SucculentTuft {
			shape: SucculentTuftShape {
				seed,
				noise_amplitude: amplitude,
				noise_frequency: 4.0,
				element_count: 6,
				..SucculentTuftShape::default()
			},
			..SucculentTuft::default()
		}
	}

	fn max_triangle_edge(pos: &[[f32; 3]], indices: &[u32]) -> f32 {
		let mut max_len = 0.0_f32;
		for tri in indices.chunks_exact(3) {
			for i in 0..3 {
				let a = Vec3::from_array(pos[tri[i] as usize]);
				let b = Vec3::from_array(pos[tri[(i + 1) % 3] as usize]);
				max_len = max_len.max(a.distance(b));
			}
		}
		max_len
	}

	#[test]
	fn upward_directions_are_unit_and_mostly_up() -> Result<()> {
		let tuft =
			SucculentTuft::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::default();
		let dirs = directions::CapDirections::upward(
			tuft.shape.element_count,
			tuft.shape.seed,
			tuft.shape.max_tilt_radians,
		);
		assert_eq!(dirs.len(), 8);
		for d in dirs {
			assert!((d.length() - 1.0).abs() < 1e-4);
			assert!(d.y > 0.5, "spears should bias upward: {d:?}");
		}
		Ok(())
	}

	#[test]
	fn same_seed_same_upward_directions() -> Result<()> {
		let a = directions::CapDirections::upward(6, 7, 0.35);
		let b = directions::CapDirections::upward(6, 7, 0.35);
		for (da, db) in a.iter().zip(b.iter()) {
			assert_eq!(da, db);
		}
		Ok(())
	}

	#[test]
	fn succulent_mesh_is_low_poly() -> Result<()> {
		let mesh = test_succulent(1, 0.08).build_mesh(1.0);
		let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
			anyhow::bail!("expected float positions");
		};
		let Some(bevy::mesh::Indices::U32(indices)) = mesh.indices() else {
			anyhow::bail!("expected u32 indices");
		};
		assert!(pos.len() <= 6 * 5 * 3);
		assert!(indices.len() <= 6 * 4 * 3 * 6);
		Ok(())
	}

	#[test]
	fn succulent_noise_bends_vertices() -> Result<()> {
		let flat = test_succulent(9, 0.0).build_mesh(1.0);
		let wavy = test_succulent(9, 0.12).build_mesh(1.0);
		let Some(VertexAttributeValues::Float32x3(flat_p)) = flat.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected flat positions");
		};
		let Some(VertexAttributeValues::Float32x3(wavy_p)) = wavy.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected wavy positions");
		};
		assert_eq!(flat_p.len(), wavy_p.len());
		let max_delta: f32 = flat_p
			.iter()
			.zip(wavy_p.iter())
			.map(|(a, b)| (a[0] - b[0]).abs() + (a[2] - b[2]).abs())
			.fold(0.0_f32, f32::max);
		assert!(max_delta > 1e-3);
		Ok(())
	}

	#[test]
	fn no_degenerate_long_edges_for_many_seeds() -> Result<()> {
		for seed in 0..512_i32 {
			let mesh = test_succulent(seed, 0.08).build_mesh(0.6);
			let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
			else {
				anyhow::bail!("expected float positions");
			};
			for p in pos {
				assert!(p[0].is_finite() && p[1].is_finite() && p[2].is_finite(), "seed {seed}");
			}
			let Some(bevy::mesh::Indices::U32(indices)) = mesh.indices() else {
				anyhow::bail!("expected u32 indices");
			};
			if pos.is_empty() {
				continue;
			}
			let max_edge = max_triangle_edge(pos, indices);
			assert!(max_edge < 1.5, "seed {seed} produced long edge {max_edge}");
		}
		Ok(())
	}

	#[test]
	fn spawn_transform_strips_non_uniform_scale() -> Result<()> {
		let (t, u) = Transform {
			translation: Vec3::new(1.0, 2.0, 3.0),
			rotation: Quat::from_rotation_y(0.5),
			scale: Vec3::new(0.5, 2.0, 0.25),
		}
		.tuft_spawn_uniform();
		assert_eq!(t.scale, Vec3::ONE);
		assert!((u - 2.0).abs() < 1e-5);
		Ok(())
	}
}
