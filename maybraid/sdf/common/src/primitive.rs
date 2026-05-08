//! Unified mesh/render surface for every [`crate::TaperedCylinder`] / [`crate::NoisyCylinder`] primitive.

use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use render_item::{
	mesh::{handle::MeshHandle, IdentifiedMesh, MeshDispatch, MeshId},
	NormalizeChunk, RenderItem,
};
use sdf::{Bounds, Sdf};

use crate::{NoisyCylinder, NoisySurface, TaperedCylinder};

/// Every concrete SDF primitive exposed by **`sdf-common`**, for authoring / playgrounds / dispatch.
#[derive(Clone)]
pub enum SdfCommonPrimitive {
	TaperedCylinder(TaperedCylinder),
	NoisyCylinder(NoisyCylinder),
}

impl SdfCommonPrimitive {
	pub fn tapered_cylinder_default() -> Self {
		Self::TaperedCylinder(TaperedCylinder::default())
	}

	pub fn noisy_cylinder_default() -> Self {
		Self::NoisyCylinder(NoisySurface::new_perlin(
			TaperedCylinder::default(),
			42,
			5.0,
			0.05,
		))
	}

	pub fn variant_key(&self) -> &'static str {
		match self {
			Self::TaperedCylinder(_) => "tapered_cylinder",
			Self::NoisyCylinder(_) => "noisy_cylinder",
		}
	}

	pub fn all_variant_keys() -> &'static [&'static str] {
		&["tapered_cylinder", "noisy_cylinder"]
	}

	/// Resolve a user-typed or UI-selected label (case-insensitive).
	pub fn from_name(name: &str) -> Option<Self> {
		match name.trim().to_ascii_lowercase().as_str() {
			"tapered_cylinder" | "cylinder" | "tapered" | "tc" | "1" => {
				Some(Self::tapered_cylinder_default())
			}
			"noisy_cylinder" | "noisy" | "nc" | "2" => Some(Self::noisy_cylinder_default()),
			_ => None,
		}
	}
}

impl std::fmt::Debug for SdfCommonPrimitive {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::TaperedCylinder(c) => f.debug_tuple("TaperedCylinder").field(c).finish(),
			Self::NoisyCylinder(_) => f.write_str("NoisyCylinder(...)"),
		}
	}
}

impl std::fmt::Display for SdfCommonPrimitive {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::TaperedCylinder(_) => write!(f, "TaperedCylinder"),
			Self::NoisyCylinder(_) => write!(f, "NoisyCylinder"),
		}
	}
}

impl Sdf for SdfCommonPrimitive {
	fn distance(&self, p: Vec3) -> f32 {
		match self {
			Self::TaperedCylinder(c) => c.distance(p),
			Self::NoisyCylinder(n) => n.distance(p),
		}
	}

	fn bounds(&self) -> Bounds {
		match self {
			Self::TaperedCylinder(c) => c.bounds(),
			Self::NoisyCylinder(n) => n.bounds(),
		}
	}
}

impl NormalizeChunk for SdfCommonPrimitive {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		match self {
			Self::TaperedCylinder(c) => c.normalize_chunk(cascade_chunk),
			Self::NoisyCylinder(n) => n.normalize_chunk(cascade_chunk),
		}
	}
}

impl IdentifiedMesh for SdfCommonPrimitive {
	fn id(&self) -> MeshId {
		match self {
			Self::TaperedCylinder(c) => MeshId::new(format!(
				"sdf_common.TaperedCylinder:{}:{}:{}:{}:{}",
				c.base_radius.to_bits(),
				c.top_radius.to_bits(),
				c.y_min.to_bits(),
				c.height.to_bits(),
				c.bounds_margin.to_bits(),
			)),
			Self::NoisyCylinder(n) => {
				let p = n.noise.params();
				MeshId::new(format!(
					"sdf_common.NoisyCylinder:{}:{}:{}:{}:{:?}",
					p.seed,
					p.frequency.to_bits(),
					p.amplitude.to_bits(),
					p.octaves,
					p.noise_type,
				))
			}
		}
	}
}

/// Standard [`RenderItem`] for marching-cubes meshing of [`SdfCommonPrimitive`] with a Bevy material.
#[derive(Clone)]
pub struct SdfCommonRenderItem<M: Material> {
	pub primitive: SdfCommonPrimitive,
	pub material: MeshMaterial3d<M>,
}

impl<M: Material + Clone> SdfCommonRenderItem<M> {
	pub fn new(primitive: SdfCommonPrimitive, material: MeshMaterial3d<M>) -> Self {
		Self { primitive, material }
	}
}

impl<M: Material + Clone> RenderItem for SdfCommonRenderItem<M>
where
	(CascadeChunk, MeshDispatch<MeshHandle<SdfCommonPrimitive>>, Transform, MeshMaterial3d<M>): Bundle,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let mesh_handle = MeshHandle::new(self.primitive.clone());
		commands.spawn((
			*cascade_chunk,
			MeshDispatch::new(mesh_handle),
			transform,
			self.material.clone(),
		));
		vec![]
	}
}
