//! Face-winding policy for posed triangles.

use bevy::math::Vec3;

/// How to treat triangle winding after posing (including mirrored parts).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindingPolicy {
	/// Leave authored indices unchanged.
	Keep,
	/// Flip a triangle when its geometric normal disagrees with vertex normals.
	///
	/// Mesh-export uses this so mirrored L/R parts stay front-facing for
	/// Blender / Procreate (they cull clockwise).
	#[default]
	AlignWithVertexNormals,
}

impl WindingPolicy {
	pub fn apply(self, positions: &[Vec3], normals: &[Vec3], faces: &mut [[u32; 3]]) {
		match self {
			Self::Keep => {}
			Self::AlignWithVertexNormals => align_with_vertex_normals(positions, normals, faces),
		}
	}
}

fn align_with_vertex_normals(positions: &[Vec3], normals: &[Vec3], faces: &mut [[u32; 3]]) {
	if normals.len() != positions.len() {
		return;
	}
	for face in faces.iter_mut() {
		let [ia, ib, ic] = *face;
		let (Some(&pa), Some(&pb), Some(&pc)) =
			(positions.get(ia as usize), positions.get(ib as usize), positions.get(ic as usize))
		else {
			continue;
		};
		let (Some(&na), Some(&nb), Some(&nc)) =
			(normals.get(ia as usize), normals.get(ib as usize), normals.get(ic as usize))
		else {
			continue;
		};
		let geo = (pb - pa).cross(pc - pa);
		if geo.dot(na + nb + nc) < 0.0 {
			*face = [ia, ic, ib];
		}
	}
}
