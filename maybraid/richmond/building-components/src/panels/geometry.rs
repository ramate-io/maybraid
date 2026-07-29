//! Atomic panel geometry and tessellation policy.

use scene_ref::MirrorAxis;

use crate::placed::{Placed, Placement};

/// Suggested full tile width along local \(X\) (matches unscaled ground kit \(X \in [0, 1]\)).
pub const DEFAULT_TILE_WIDTH: f32 = 1.0;

/// How many tiles fit a length given a suggested width.
///
/// \(n = \mathrm{round}(\texttt{length}/\texttt{tile\_width})\), at least 1. Callers use
/// \(\texttt{length}/n\) as the actual tile size so tiles span the length exactly.
pub fn fitted_tile_count(length: f32, tile_width: f32) -> u32 {
	let tw = tile_width.max(1e-4);
	((length / tw).round() as i32).max(1) as u32
}

/// Controls how [`crate::panels::Quad::decompose`] fills the rectangular body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TessellatePolicy {
	/// When true, emit [`PanelGeom::Rectangle`] tiles for the body.
	/// When false, emit complementary [`PanelGeom::RightTriangle`] pairs (dual-triangle fill).
	pub prefer_rectangle: bool,
}

impl TessellatePolicy {
	pub const RECTANGLE: Self = Self {
		prefer_rectangle: true,
	};

	pub const DUAL_TRIANGLES: Self = Self {
		prefer_rectangle: false,
	};

	pub fn prefer_rectangle(prefer: bool) -> Self {
		Self {
			prefer_rectangle: prefer,
		}
	}
}

/// Axis-aligned rectangular panel tile in panel space.
///
/// Kit footprint: \(X \in [0, 1]\), \(Z \in [-1, 0]\) (lower-left at origin, depth toward \(-Z\)).
/// Placement scale maps that unit square to world width × depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rectangle;

/// Unit right-triangle panel atom.
///
/// Kit footprint: \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\).
/// When `mirror` is set, the GLB is rebuilt via [`scene_ref::SceneRef`] mirroring
/// (positive Transform scale) instead of a negative `scale.x`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RightTriangle {
	pub mirror: Option<MirrorAxis>,
}

impl Default for RightTriangle {
	fn default() -> Self {
		Self { mirror: None }
	}
}

impl RightTriangle {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn mirrored(mirror: MirrorAxis) -> Self {
		Self {
			mirror: Some(mirror),
		}
	}
}

/// Shared panel geometry IR (authoring + decompose intermediates).
#[derive(Debug, Clone, PartialEq)]
pub enum PanelGeom {
	Rectangle(Rectangle),
	RightTriangle(RightTriangle),
	Quad(crate::panels::Quad),
	Joint(crate::panels::Joint),
	QuadPolyline(crate::panels::QuadPolyline),
}

impl PanelGeom {
	pub fn rectangle() -> Self {
		Self::Rectangle(Rectangle)
	}

	pub fn right_triangle(mirror: Option<MirrorAxis>) -> Self {
		Self::RightTriangle(RightTriangle { mirror })
	}

	pub fn as_atom(&self) -> Option<PanelAtom> {
		match self {
			Self::Rectangle(r) => Some(PanelAtom::Rectangle(*r)),
			Self::RightTriangle(t) => Some(PanelAtom::RightTriangle(*t)),
			_ => None,
		}
	}
}

/// Atoms emitted by [`crate::panels::Quad::decompose`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelAtom {
	Rectangle(Rectangle),
	RightTriangle(RightTriangle),
}

impl From<PanelAtom> for PanelGeom {
	fn from(atom: PanelAtom) -> Self {
		match atom {
			PanelAtom::Rectangle(r) => Self::Rectangle(r),
			PanelAtom::RightTriangle(t) => Self::RightTriangle(t),
		}
	}
}

/// Composites emitted by [`crate::panels::QuadPolyline::decompose`].
#[derive(Debug, Clone, PartialEq)]
pub enum PanelComposite {
	Quad(crate::panels::Quad),
	Joint(crate::panels::Joint),
}

impl From<PanelComposite> for PanelGeom {
	fn from(c: PanelComposite) -> Self {
		match c {
			PanelComposite::Quad(q) => Self::Quad(q),
			PanelComposite::Joint(j) => Self::Joint(j),
		}
	}
}

pub(crate) fn placed_atom(atom: PanelAtom, placement: Placement) -> Placed<PanelGeom> {
	Placed::with_placement(PanelGeom::from(atom), placement)
}
