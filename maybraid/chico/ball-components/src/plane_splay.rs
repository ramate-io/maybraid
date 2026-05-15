//! Plane splay canopy: icosphere core plus flat triangular plates ([RFC-183 §3.1.2.5](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/05-plane-splay/README.md)).
//!
//! Each plate lies in a **plane through the cluster origin** (so every triangle **contains** the centroid), with a **different outward-facing radial** per icosphere face so plates are offset around the volume instead of meeting in a single starburst tip.
//!
//! The core and merged plate shell are separate [`Mesh3d`] children under the root; [`ChildOf`] keeps despawn coherent.

use std::marker::PhantomData;

use bevy::mesh::primitives::{MeshBuilder, Meshable, SphereKind, SphereMeshBuilder};
use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::prelude::*;
use render_item::{CascadeChunk, RenderItem};

/// Canopy cluster: small **icosphere** core plus **flat triangular plates** (one merged mesh) in planes through the origin.
#[derive(Component, Clone, Debug)]
pub struct PlaneSplay<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	/// [`SphereKind::Ico`] subdivision count for the solid core (0 = icosahedron, 1 = 80 faces, …).
	pub icosphere_subdivisions: u32,
	/// Radius of the icosphere core in local units (before parent [`Transform`] scale).
	pub core_radius: f32,
	/// Circumradius of each equilateral plate in its plane through the origin (controls reach, not spike length along one axis).
	pub leaf_disc_radius: f32,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for PlaneSplay<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self {
			icosphere_subdivisions: 0,
			core_radius: 0.8,
			leaf_disc_radius: 0.9,
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> PlaneSplay<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	/// Approximate number of faces on the core icosphere (`20 · 4^n`).
	pub fn icosphere_face_count(&self) -> u32 {
		let n = self.icosphere_subdivisions.min(6);
		20u32.saturating_mul(4u32.saturating_pow(n))
	}
}

/// Right-handed orthonormal basis spanning the plane `u · x = 0` (plane through origin with normal `u`).
fn tangent_basis(u: Vec3) -> (Vec3, Vec3) {
	let up = if u.y.abs() < 0.92 { Vec3::Y } else { Vec3::Z };
	let mut e1 = up.cross(u);
	if e1.length_squared() < 1e-10 {
		e1 = Vec3::X.cross(u);
	}
	e1 = e1.normalize();
	let e2 = u.cross(e1).normalize();
	(e1, e2)
}

/// One triangle per icosphere face: equilateral triangle in the plane through the origin ⊥ `radial`,
/// rotated in-plane by `phi` so plates do not align globally.
fn plate_shell_mesh(core: &Mesh, leaf_disc_radius: f32) -> Option<Mesh> {
	let positions = core.attribute(Mesh::ATTRIBUTE_POSITION)?;
	let VertexAttributeValues::Float32x3(pos) = positions else {
		return None;
	};
	let indices = core.indices()?;
	let Indices::U32(idx) = indices else {
		return None;
	};

	let mut plates: Option<Mesh> = None;

	for (fi, tri) in idx.chunks_exact(3).enumerate() {
		let i0 = tri[0] as usize;
		let i1 = tri[1] as usize;
		let i2 = tri[2] as usize;
		let a = Vec3::from_array(pos[i0]);
		let b = Vec3::from_array(pos[i1]);
		let c = Vec3::from_array(pos[i2]);

		let centroid = (a + b + c) * (1.0 / 3.0);
		if centroid.length_squared() < 1e-12 {
			continue;
		}
		let radial = centroid.normalize();
		let (e1, e2) = tangent_basis(radial);

		// In-plane roll so neighboring faces do not share the same blade clocking.
		let phi = (fi as f32) * 0.754_877_666_246_693_7 * std::f32::consts::TAU;
		let cos_p = phi.cos();
		let sin_p = phi.sin();
		let e1r = cos_p * e1 + sin_p * e2;
		let e2r = -sin_p * e1 + cos_p * e2;

		let r = leaf_disc_radius;
		let v0 = r * e1r;
		let ang = std::f32::consts::TAU / 3.0;
		let v1 = r * (ang.cos() * e1r + ang.sin() * e2r);
		let v2 = r * ((2.0 * ang).cos() * e1r + (2.0 * ang).sin() * e2r);

		let piece = Triangle3d::new(v0, v1, v2).mesh().build();
		match &mut plates {
			None => plates = Some(piece),
			Some(acc) => {
				let _ = acc.merge(&piece);
			}
		}
	}

	plates
}

impl<M: Material, S> RenderItem for PlaneSplay<M, S>
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
			.spawn((self.clone(), cascade_chunk.clone(), transform, Visibility::default()))
			.id();

		let splay = self.clone();
		commands.queue(move |world: &mut World| {
			let subdiv = splay.icosphere_subdivisions.min(4);
			let core_mesh =
				SphereMeshBuilder::new(splay.core_radius, SphereKind::Ico { subdivisions: subdiv })
					.build();

			let plate_mesh = plate_shell_mesh(&core_mesh, splay.leaf_disc_radius);

			let (core_handle, plate_handle) = {
				let mut meshes = world.resource_mut::<Assets<Mesh>>();
				let core_handle = meshes.add(core_mesh);
				let plate_handle = plate_mesh.map(|p| meshes.add(p));
				(core_handle, plate_handle)
			};

			let material: MeshMaterial3d<M> = splay.material.clone().into();

			world.spawn((
				ChildOf(root),
				Mesh3d(core_handle),
				material.clone(),
				Transform::IDENTITY,
			));

			if let Some(ph) = plate_handle {
				world.spawn((ChildOf(root), Mesh3d(ph), material, Transform::IDENTITY));
			}
		});

		vec![root]
	}
}
