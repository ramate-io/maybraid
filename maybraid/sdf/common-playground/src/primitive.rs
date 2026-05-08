//! Playground-only union of [`sdf_common`] mesh primitives for marching-cubes + [`RenderItem`].

use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use render_item::{
	mesh::{handle::MeshHandle, IdentifiedMesh, MeshDispatch, MeshId},
	NormalizeChunk, RenderItem,
};
use sdf::{Bounds, Sdf};
use sdf_common::{NoisyCylinder, NoisySurface, TaperedCylinder};

/// Concrete SDF variants previewed in this playground.
#[derive(Clone)]
pub enum PlaygroundPrimitive {
	TaperedCylinder(TaperedCylinder),
	NoisyCylinder(NoisyCylinder),
}

impl PlaygroundPrimitive {
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
			Self::TaperedCylinder(_) => "tapered-cylinder",
			Self::NoisyCylinder(_) => "noisy-cylinder",
		}
	}

	pub fn all_variant_keys() -> &'static [&'static str] {
		&["tapered-cylinder", "noisy-cylinder"]
	}

	pub fn from_name(name: &str) -> Option<Self> {
		match name.trim().to_ascii_lowercase().replace('_', "-").as_str() {
			"tapered-cylinder" | "cylinder" | "tapered" | "tc" | "1" => {
				Some(Self::tapered_cylinder_default())
			}
			"noisy-cylinder" | "noisy" | "nc" | "2" => Some(Self::noisy_cylinder_default()),
			_ => None,
		}
	}
}

impl std::fmt::Debug for PlaygroundPrimitive {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::TaperedCylinder(c) => f.debug_tuple("TaperedCylinder").field(c).finish(),
			Self::NoisyCylinder(_) => f.write_str("NoisyCylinder(...)"),
		}
	}
}

impl std::fmt::Display for PlaygroundPrimitive {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::TaperedCylinder(_) => write!(f, "TaperedCylinder"),
			Self::NoisyCylinder(_) => write!(f, "NoisyCylinder"),
		}
	}
}

impl Sdf for PlaygroundPrimitive {
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

impl NormalizeChunk for PlaygroundPrimitive {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		match self {
			Self::TaperedCylinder(c) => c.normalize_chunk(cascade_chunk),
			Self::NoisyCylinder(n) => n.normalize_chunk(cascade_chunk),
		}
	}
}

impl IdentifiedMesh for PlaygroundPrimitive {
	fn id(&self) -> MeshId {
		match self {
			Self::TaperedCylinder(c) => MeshId::new(format!(
				"playground.TaperedCylinder:{}:{}:{}:{}:{}",
				c.base_radius.to_bits(),
				c.top_radius.to_bits(),
				c.y_min.to_bits(),
				c.height.to_bits(),
				c.bounds_margin.to_bits(),
			)),
			Self::NoisyCylinder(n) => {
				let p = &n.noise;
				MeshId::new(format!(
					"playground.NoisyCylinder:{}:{}:{}:{}:{:?}",
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

#[derive(Clone)]
pub struct PlaygroundRenderItem<M: Material> {
	pub primitive: PlaygroundPrimitive,
	pub material: MeshMaterial3d<M>,
}

impl<M: Material + Clone> PlaygroundRenderItem<M> {
	pub fn new(primitive: PlaygroundPrimitive, material: MeshMaterial3d<M>) -> Self {
		Self { primitive, material }
	}
}

impl<M: Material + Clone> RenderItem for PlaygroundRenderItem<M>
where
	(CascadeChunk, MeshDispatch<MeshHandle<PlaygroundPrimitive>>, Transform, MeshMaterial3d<M>): Bundle,
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
