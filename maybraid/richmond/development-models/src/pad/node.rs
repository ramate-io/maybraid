//! One building-skirt leaf: primitive + ease params.

use bevy::math::Vec2;
use procedural_common::Bounds2;

use super::elevation::PadElevation;
use super::footprint::{PadFootprint, PadReach, PadRect};
use super::{PadParams, PadPrimitive};

/// One authored pad, discovered by footprint AABB ⊕ ease extent.
#[derive(Debug, Clone)]
pub struct PadNode {
	pub primitive: PadPrimitive,
	pub params: PadParams,
	/// Max distance beyond flatten support at which the ease may need this node.
	pub max_correction_extent: f32,
}

impl PadNode {
	pub fn new(primitive: PadPrimitive, params: PadParams, max_correction_extent: f32) -> Self {
		Self { primitive, params, max_correction_extent: max_correction_extent.max(0.0) }
	}

	/// Flatten terrace under a yawed building rectangle, plus berm and ease skirt.
	pub fn rectangular_flatten(
		center: Vec2,
		building_half_extents: Vec2,
		yaw: f32,
		height: f32,
		params: PadParams,
	) -> Self {
		let berm = params.berm.max(0.0);
		let ease = params.ease.max(0.0);
		let round = params.round.max(0.0);
		let half = building_half_extents.max(Vec2::splat(1e-3)) + Vec2::splat(berm);
		Self::new(
			PadPrimitive {
				footprint: PadFootprint::Rect(PadRect { center, half_extents: half, yaw, round }),
				elevation: PadElevation::Flatten { height },
				influence_pad: ease,
			},
			params,
			ease,
		)
	}

	pub fn rectangular_flatten_default(
		center: Vec2,
		building_half_extents: Vec2,
		yaw: f32,
		height: f32,
	) -> Self {
		Self::rectangular_flatten(center, building_half_extents, yaw, height, PadParams::default())
	}

	pub fn index_pad(&self) -> f32 {
		self.max_correction_extent
			.max(self.primitive.influence_pad)
			.max(self.params.ease)
	}

	pub fn phi(&self, p: Vec2) -> f32 {
		self.primitive.phi(p)
	}

	pub fn flatten_height(&self) -> f32 {
		self.primitive.elevation.representative_height()
	}

	pub fn height_at(&self, p: Vec2) -> f32 {
		self.primitive.height_at(p)
	}

	/// Graded capsule between two pad sites (hydro thalweg analog).
	pub fn graded_reach(
		a: Vec2,
		b: Vec2,
		half_width: f32,
		height_a: f32,
		height_b: f32,
		params: PadParams,
	) -> Self {
		let berm = params.berm.max(0.0);
		let ease = params.ease.max(0.0);
		Self::new(
			PadPrimitive {
				footprint: PadFootprint::Reach(PadReach {
					a,
					b,
					half_width: (half_width + berm).max(1e-3),
				}),
				elevation: PadElevation::Grade { height_a, height_b },
				influence_pad: ease,
			},
			params,
			ease,
		)
	}

	pub fn correction_index_bounds(&self) -> Bounds2 {
		let (mn, mx) = self.primitive.aabb();
		let pad = self.index_pad();
		Bounds2 { min: mn - Vec2::splat(pad), max: mx + Vec2::splat(pad) }
	}

	pub fn correction_intersects(&self, bounds: Bounds2) -> bool {
		let support = self.correction_index_bounds();
		support.min.x <= bounds.max.x
			&& bounds.min.x <= support.max.x
			&& support.min.y <= bounds.max.y
			&& bounds.min.y <= support.max.y
	}

	pub fn contains_index_point(&self, p: Vec2) -> bool {
		let b = self.correction_index_bounds();
		p.x >= b.min.x && p.x <= b.max.x && p.y >= b.min.y && p.y <= b.max.y
	}

	/// Flatten or grade inside the support; ease in the outer skirt; else none.
	pub fn classification(&self, p: Vec2) -> Option<PadStage> {
		let d = self.phi(p);
		let ease = self.params.ease.max(0.0);
		if d <= 0.0 {
			Some(match self.primitive.elevation {
				PadElevation::Flatten { .. } => PadStage::Flatten,
				PadElevation::Grade { .. } => PadStage::Grade,
			})
		} else if d < ease {
			Some(PadStage::Ease)
		} else {
			None
		}
	}

	/// Smoothstep from authored height at \(\phi = 0\) to incoming elevation at the ease outer.
	pub fn ease_candidate(&self, elevation: f32, p: Vec2) -> f32 {
		let d = self.phi(p);
		let ease = self.params.ease.max(1e-3);
		let u = (d / ease).clamp(0.0, 1.0);
		let fade = u * u * (3.0 - 2.0 * u);
		let h = self.height_at(p);
		elevation * fade + h * (1.0 - fade)
	}

	/// Flatten terraces win over graded paths, which win over ease skirts.
	pub fn elevation_blend(nodes: &[&Self], elevation: f32, p: Vec2) -> f32 {
		let mut flatten: Option<(&Self, f32)> = None;
		let mut grade: Option<(&Self, f32)> = None;
		let mut ease: Option<(&Self, f32)> = None;
		for node in nodes {
			let phi = node.phi(p);
			match node.classification(p) {
				Some(PadStage::Flatten) => {
					// Tightest containing terrace (largest φ ≤ 0), not the deepest SDF.
					if flatten.is_none_or(|(_, d)| phi > d) {
						flatten = Some((node, phi));
					}
				}
				Some(PadStage::Grade) => {
					if grade.is_none_or(|(_, d)| phi > d) {
						grade = Some((node, phi));
					}
				}
				Some(PadStage::Ease) => {
					if ease.is_none_or(|(_, d)| phi < d) {
						ease = Some((node, phi));
					}
				}
				None => {}
			}
		}
		if let Some((node, _)) = flatten {
			return node.height_at(p);
		}
		if let Some((node, _)) = grade {
			return node.height_at(p);
		}
		if let Some((node, _)) = ease {
			return node.ease_candidate(elevation, p);
		}
		elevation
	}
}

/// Hard bands for one pad (terrace vs connecting grade vs ease skirt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadStage {
	Flatten,
	Grade,
	Ease,
}
