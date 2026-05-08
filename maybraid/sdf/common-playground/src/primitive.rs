//! Playground-only union of [`sdf_common`] mesh primitives for marching-cubes + [`RenderItem`].

use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use render_item::{
	mesh::{handle::MeshHandle, IdentifiedMesh, MeshDispatch, MeshId},
	NormalizeChunk, RenderItem,
};
use sdf::{Bounds, Sdf};
use sdf_common::{
	Ball, CrookCylinder, NoisyBall, NoisyCylinder, NoisyCrookCylinder, NoisySurface,
	TaperedCylinder, UnitCylinderNoiseParams,
};

/// Concrete SDF variants previewed in this playground.
#[derive(Clone)]
pub enum PlaygroundPrimitive {
	TaperedCylinder(TaperedCylinder),
	NoisyCylinder(NoisyCylinder),
	CrookCylinder(CrookCylinder),
	NoisyCrookCylinder(NoisyCrookCylinder),
	Ball(Ball),
	NoisyBall(NoisyBall),
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

	/// Visible bend for Tab cycling / key `3` (same radii as default taper).
	pub fn crook_cylinder_default() -> Self {
		Self::CrookCylinder(CrookCylinder {
			bend_x: 0.12,
			bend_z: 0.08,
			..CrookCylinder::unit_segment(0.5, 0.4)
		})
	}

	/// Same crook as [`Self::crook_cylinder_default`] with [`UnitCylinderNoiseParams`] surface noise.
	pub fn noisy_crook_cylinder_default() -> Self {
		let crook = CrookCylinder {
			bend_x: 0.12,
			bend_z: 0.08,
			..CrookCylinder::unit_segment(0.5, 0.4)
		};
		Self::NoisyCrookCylinder(NoisySurface::from_params(
			crook,
			UnitCylinderNoiseParams.into(),
		))
	}

	pub fn ball_default() -> Self {
		Self::Ball(Ball::unit_sphere())
	}

	/// [`Ball::unit_sphere`] with Perlin noise (seed **42**, freq **5**, amp **0.05**) like [`Self::noisy_cylinder_default`].
	pub fn noisy_ball_default() -> Self {
		Self::NoisyBall(NoisySurface::new_perlin(Ball::unit_sphere(), 42, 5.0, 0.05))
	}

	pub fn variant_key(&self) -> &'static str {
		match self {
			Self::TaperedCylinder(_) => "tapered-cylinder",
			Self::NoisyCylinder(_) => "noisy-cylinder",
			Self::CrookCylinder(_) => "crook-cylinder",
			Self::NoisyCrookCylinder(_) => "noisy-crook-cylinder",
			Self::Ball(_) => "ball",
			Self::NoisyBall(_) => "noisy-ball",
		}
	}

	pub fn all_variant_keys() -> &'static [&'static str] {
		&[
			"tapered-cylinder",
			"noisy-cylinder",
			"crook-cylinder",
			"noisy-crook-cylinder",
			"ball",
			"noisy-ball",
		]
	}

	pub fn from_name(name: &str) -> Option<Self> {
		match name.trim().to_ascii_lowercase().replace('_', "-").as_str() {
			"tapered-cylinder" | "cylinder" | "tapered" | "tc" | "1" => {
				Some(Self::tapered_cylinder_default())
			}
			"noisy-cylinder" | "noisy" | "nc" | "2" => Some(Self::noisy_cylinder_default()),
			"crook-cylinder" | "crook" | "cc" | "3" => Some(Self::crook_cylinder_default()),
			"noisy-crook-cylinder" | "noisy-crook" | "ncc" | "4" => {
				Some(Self::noisy_crook_cylinder_default())
			}
			"ball" | "sphere" | "b" | "5" => Some(Self::ball_default()),
			"noisy-ball" | "noisy-sphere" | "nb" | "6" => Some(Self::noisy_ball_default()),
			_ => None,
		}
	}
}

impl std::fmt::Debug for PlaygroundPrimitive {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::TaperedCylinder(c) => f.debug_tuple("TaperedCylinder").field(c).finish(),
			Self::NoisyCylinder(_) => f.write_str("NoisyCylinder(...)"),
			Self::CrookCylinder(c) => f.debug_tuple("CrookCylinder").field(c).finish(),
			Self::NoisyCrookCylinder(_) => f.write_str("NoisyCrookCylinder(...)"),
			Self::Ball(b) => f.debug_tuple("Ball").field(b).finish(),
			Self::NoisyBall(_) => f.write_str("NoisyBall(...)"),
		}
	}
}

impl std::fmt::Display for PlaygroundPrimitive {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::TaperedCylinder(_) => write!(f, "TaperedCylinder"),
			Self::NoisyCylinder(_) => write!(f, "NoisyCylinder"),
			Self::CrookCylinder(_) => write!(f, "CrookCylinder"),
			Self::NoisyCrookCylinder(_) => write!(f, "NoisyCrookCylinder"),
			Self::Ball(_) => write!(f, "Ball"),
			Self::NoisyBall(_) => write!(f, "NoisyBall"),
		}
	}
}

impl Sdf for PlaygroundPrimitive {
	fn distance(&self, p: Vec3) -> f32 {
		match self {
			Self::TaperedCylinder(c) => c.distance(p),
			Self::NoisyCylinder(n) => n.distance(p),
			Self::CrookCylinder(c) => c.distance(p),
			Self::NoisyCrookCylinder(n) => n.distance(p),
			Self::Ball(b) => b.distance(p),
			Self::NoisyBall(n) => n.distance(p),
		}
	}

	fn bounds(&self) -> Bounds {
		match self {
			Self::TaperedCylinder(c) => c.bounds(),
			Self::NoisyCylinder(n) => n.bounds(),
			Self::CrookCylinder(c) => c.bounds(),
			Self::NoisyCrookCylinder(n) => n.bounds(),
			Self::Ball(b) => b.bounds(),
			Self::NoisyBall(n) => n.bounds(),
		}
	}
}

impl NormalizeChunk for PlaygroundPrimitive {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		match self {
			Self::TaperedCylinder(c) => c.normalize_chunk(cascade_chunk),
			Self::NoisyCylinder(n) => n.normalize_chunk(cascade_chunk),
			Self::CrookCylinder(c) => c.normalize_chunk(cascade_chunk),
			Self::NoisyCrookCylinder(n) => n.normalize_chunk(cascade_chunk),
			Self::Ball(b) => b.normalize_chunk(cascade_chunk),
			Self::NoisyBall(n) => n.normalize_chunk(cascade_chunk),
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
			Self::CrookCylinder(c) => MeshId::new(format!(
				"playground.CrookCylinder:{}:{}:{}:{}:{}:{}:{}:{}:{}",
				c.base_radius.to_bits(),
				c.top_radius.to_bits(),
				c.y_min.to_bits(),
				c.height.to_bits(),
				c.bounds_margin.to_bits(),
				c.bend_x.to_bits(),
				c.bend_z.to_bits(),
				c.phase_x.to_bits(),
				c.phase_z.to_bits(),
			)),
			Self::NoisyCrookCylinder(n) => {
				let c = &n.inner;
				let p = &n.noise;
				MeshId::new(format!(
					"playground.NoisyCrookCylinder:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{:?}",
					c.base_radius.to_bits(),
					c.top_radius.to_bits(),
					c.y_min.to_bits(),
					c.height.to_bits(),
					c.bounds_margin.to_bits(),
					c.bend_x.to_bits(),
					c.bend_z.to_bits(),
					c.phase_x.to_bits(),
					c.phase_z.to_bits(),
					p.seed,
					p.frequency.to_bits(),
					p.amplitude.to_bits(),
					p.octaves,
					p.noise_type,
				))
			}
			Self::Ball(b) => MeshId::new(format!(
				"playground.Ball:{}:{}",
				b.radius.to_bits(),
				b.bounds_margin.to_bits(),
			)),
			Self::NoisyBall(n) => {
				let b = &n.inner;
				let p = &n.noise;
				MeshId::new(format!(
					"playground.NoisyBall:{}:{}:{}:{}:{}:{}:{:?}",
					b.radius.to_bits(),
					b.bounds_margin.to_bits(),
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
