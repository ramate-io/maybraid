//! Quadrilateral panel: rectangular body + optional signed edge triangles on four sides.

use std::f32::consts::PI;

use bevy_math::Vec3;
use scene_ref::MirrorAxis;

use crate::panels::geometry::{
	fitted_tile_count, placed_geom, PanelGeometry, PanelStyle, Rectangle, RightTriangle,
	DEFAULT_TILE_WIDTH,
};
use crate::placed::{Placed, Placement};

/// Which edge of the quad rectangle an end-cap attaches to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndSide {
	Left,
	Right,
	Top,
	Bottom,
}

/// Quadrilateral panel in lower-left panel space.
///
/// Rectangular body plus optional signed triangular extensions on each of the four edges.
/// Sign convention (same as roof [`crate::roofs::Pitch`] left/right): positive = upright
/// (eave-long / top-long / bottom-long); negative = flipped (ridge-long / rect-long for
/// top/bottom). Top/bottom upright place the right angle on the outer tip so kit poses match
/// the body (yaw \(0\)/`\pi`); flipped places it on the rectangle edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
	/// Rectangular left-to-right span. `None` omits the rectangular body.
	pub length: Option<f32>,
	/// Depth / run (top/eave at \(Z = 0\), bottom/ridge at \(Z = -\texttt{depth}\)).
	pub depth: f32,
	/// Suggested tile width along \(X\); fitted so \(n\) tiles span `length` exactly.
	pub tile_width: f32,
	/// Optional suggested tile depth along \(Z\); when set, the body is fitted on both axes.
	pub tile_height: Option<f32>,
	pub left: Option<f32>,
	pub right: Option<f32>,
	pub top: Option<f32>,
	pub bottom: Option<f32>,
}

impl Default for Quad {
	fn default() -> Self {
		Self {
			length: None,
			depth: 0.0,
			tile_width: DEFAULT_TILE_WIDTH,
			tile_height: None,
			left: None,
			right: None,
			top: None,
			bottom: None,
		}
	}
}

impl Quad {
	pub fn new(depth: f32, tile_width: f32) -> Self {
		Self { depth: depth.max(0.0), tile_width: tile_width.max(1e-4), ..Self::default() }
	}

	pub fn with_length(mut self, length: f32) -> Self {
		self.length = Some(length.max(0.0));
		self
	}

	pub fn with_tile_width(mut self, tile_width: f32) -> Self {
		self.tile_width = tile_width.max(1e-4);
		self
	}

	pub fn with_tile_height(mut self, tile_height: f32) -> Self {
		self.tile_height = Some(tile_height.max(1e-4));
		self
	}

	pub fn with_left(mut self, base: f32) -> Self {
		self.left = Some(base);
		self
	}

	pub fn with_right(mut self, base: f32) -> Self {
		self.right = Some(base);
		self
	}

	pub fn with_top(mut self, base: f32) -> Self {
		self.top = Some(base);
		self
	}

	pub fn with_bottom(mut self, base: f32) -> Self {
		self.bottom = Some(base);
		self
	}

	/// Left end base from a plan-view angle (degrees): \(\texttt{base} = \texttt{depth}
	/// \cdot \tan\theta\).
	pub fn with_left_angle(mut self, angle_degrees: f32) -> Self {
		self.left = Some(self.depth * f32::to_radians(angle_degrees).tan());
		self
	}

	pub fn with_right_angle(mut self, angle_degrees: f32) -> Self {
		self.right = Some(self.depth * f32::to_radians(angle_degrees).tan());
		self
	}

	/// Full \(X\) extent including optional left/right end triangles.
	pub fn extent_x(self) -> f32 {
		self.left.map(|b| b.abs()).unwrap_or(0.0)
			+ self.length.unwrap_or(0.0)
			+ self.right.map(|b| b.abs()).unwrap_or(0.0)
	}

	/// \(X\) offset where the rectangular body starts (after left end triangle).
	pub fn rect_origin_x(self) -> f32 {
		self.left.map(|b| b.abs()).unwrap_or(0.0)
	}

	/// Full \(Z\) extent magnitude including optional top/bottom extensions.
	pub fn extent_z(self) -> f32 {
		self.top.map(|b| b.abs()).unwrap_or(0.0)
			+ self.depth
			+ self.bottom.map(|b| b.abs()).unwrap_or(0.0)
	}

	/// Expand into placed rectangle / right-triangle atoms (identity parent).
	pub fn decompose(self, style: PanelStyle) -> Vec<Placed<PanelGeometry>> {
		let depth = self.depth.max(0.0);
		let tile_width = self.tile_width.max(1e-4);
		let mut out = Vec::new();

		let left_w = self.left.map(|b| b.abs()).unwrap_or(0.0);
		let top_w = self.top.map(|b| b.abs()).unwrap_or(0.0);
		// Top extensions shift the rectangle in +Z so the full extent still
		// lower-left-anchors at the outermost top tip when present.
		let rect_z0 = -top_w;

		if let Some(base) = self.left {
			if base.abs() > 1e-6 && depth > 1e-6 {
				out.extend(edge_triangles(
					EndSide::Left,
					0.0,
					rect_z0,
					base,
					depth,
					tile_width,
					self.tile_height,
				));
			}
		}

		let rect_x0 = left_w;

		if let Some(length) = self.length {
			if length > 1e-6 && depth > 1e-6 {
				out.extend(body_tiles(
					rect_x0,
					rect_z0,
					length,
					depth,
					tile_width,
					self.tile_height,
					style,
				));
			}
		}

		if let Some(base) = self.right {
			if base.abs() > 1e-6 && depth > 1e-6 {
				let x_min = rect_x0 + self.length.unwrap_or(0.0);
				out.extend(edge_triangles(
					EndSide::Right,
					x_min,
					rect_z0,
					base,
					depth,
					tile_width,
					self.tile_height,
				));
			}
		}

		if let Some(base) = self.top {
			// Eave line: rectangle + only upright (positive) end triangles.
			let (x_min, span) = eave_ridge_span(self.left, self.length, self.right, true);
			if base.abs() > 1e-6 && span > 1e-6 {
				out.extend(edge_triangles(
					EndSide::Top,
					x_min,
					rect_z0,
					base,
					span,
					tile_width,
					self.tile_height,
				));
			}
		}

		if let Some(base) = self.bottom {
			// Ridge line: rectangle + only flipped (negative) end triangles.
			let (x_min, span) = eave_ridge_span(self.left, self.length, self.right, false);
			if base.abs() > 1e-6 && span > 1e-6 {
				let z_min = rect_z0 - depth;
				out.extend(edge_triangles(
					EndSide::Bottom,
					x_min,
					z_min,
					base,
					span,
					tile_width,
					self.tile_height,
				));
			}
		}

		out
	}
}

fn tile_scale(width: f32, run: f32) -> Vec3 {
	Vec3::new(width.max(1e-4), 1.0, run.max(1e-4))
}

/// \(X\) range of the eave (`at_eave`) or ridge (`!at_eave`) silhouette.
///
/// Upright (positive) left/right extend the **eave**; flipped (negative) extend the **ridge**.
/// Returns `(x_min, span)`.
fn eave_ridge_span(
	left: Option<f32>,
	length: Option<f32>,
	right: Option<f32>,
	at_eave: bool,
) -> (f32, f32) {
	let left_w = left.map(|b| b.abs()).unwrap_or(0.0);
	let right_w = right.map(|b| b.abs()).unwrap_or(0.0);
	let len = length.unwrap_or(0.0);
	let include_left = left.is_some_and(|b| if at_eave { b > 1e-6 } else { b < -1e-6 });
	let include_right = right.is_some_and(|b| if at_eave { b > 1e-6 } else { b < -1e-6 });
	let x_min = if include_left { 0.0 } else { left_w };
	let x_max = left_w + len + if include_right { right_w } else { 0.0 };
	(x_min, (x_max - x_min).max(0.0))
}

/// Two complementary right triangles fill one tile square along +X.
fn unit_square_pair(x: f32, z: f32, width: f32, run: f32) -> [Placed<PanelGeometry>; 2] {
	let scale = tile_scale(width, run);
	[
		placed_geom(
			PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
			Placement::new(Vec3::new(x, 0.0, z), 0.0).with_scale(scale),
		),
		placed_geom(
			PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
			Placement::new(Vec3::new(x + width, 0.0, z - run), PI).with_scale(scale),
		),
	]
}

fn body_tiles(
	x0: f32,
	z0: f32,
	length: f32,
	depth: f32,
	tile_width: f32,
	tile_height: Option<f32>,
	style: PanelStyle,
) -> Vec<Placed<PanelGeometry>> {
	let nx = fitted_tile_count(length, tile_width);
	let width = length / nx as f32;
	let nz = tile_height.map(|th| fitted_tile_count(depth, th)).unwrap_or(1);
	let run = depth / nz as f32;
	let mut out = Vec::with_capacity((nx * nz * 2) as usize);
	for j in 0..nz {
		let z = z0 - j as f32 * run;
		for i in 0..nx {
			let x = x0 + i as f32 * width;
			if style.has_rectangle {
				out.push(placed_geom(
					PanelGeometry::Rectangle(Rectangle),
					Placement::new(Vec3::new(x, 0.0, z), 0.0).with_scale(tile_scale(width, run)),
				));
			} else {
				out.extend(unit_square_pair(x, z, width, run));
			}
		}
	}
	out
}

/// End / edge triangle(s). Positive base → upright; negative → flipped.
///
/// Tessellation is in edge-local \((u, v)\) = (outward, along-edge). Top/bottom use the same
/// `scale(u, v)` as left/right with **no** ±90° yaw (body dual-pair \(0\)/`\pi` only).
///
/// For now (tile size ignored): even half-split into three co-oriented half-scale right
/// triangles plus the flipped complement of the right-angle corner square.
fn edge_triangles(
	side: EndSide,
	x_min: f32,
	z_ref: f32,
	base: f32,
	altitude: f32,
	_tile_width: f32,
	_tile_height: Option<f32>,
) -> Vec<Placed<PanelGeometry>> {
	let width = base.abs().max(1e-4);
	let altitude = altitude.max(1e-4);
	let upright = base >= 0.0;
	let hu = width * 0.5;
	let hv = altitude * 0.5;

	let mut out = Vec::with_capacity(4);
	// Corner square at the right angle: primary + flipped complement.
	push_edge_cell(&mut out, side, x_min, z_ref, upright, 0.0, 0.0, hu, hv, width, altitude, true);
	// Two further co-oriented half-scale triangles along the legs.
	push_edge_cell(&mut out, side, x_min, z_ref, upright, hu, 0.0, hu, hv, width, altitude, false);
	push_edge_cell(&mut out, side, x_min, z_ref, upright, 0.0, hv, hu, hv, width, altitude, false);
	out
}

fn push_edge_cell(
	out: &mut Vec<Placed<PanelGeometry>>,
	side: EndSide,
	x_min: f32,
	z_ref: f32,
	upright: bool,
	u: f32,
	v: f32,
	cell_u: f32,
	cell_v: f32,
	full_u: f32,
	full_v: f32,
	fully_inside: bool,
) {
	let scale = tile_scale(cell_u, cell_v); // outward → kit X, along-edge → kit Z
	match (side, upright) {
		// Left eave-long: right angle on the rectangle edge, mirrored on X.
		(EndSide::Left, true) => {
			let origin = Vec3::new(x_min + full_u - u, 0.0, z_ref - v);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
				Placement::new(origin, 0.0).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
					Placement::new(Vec3::new(origin.x - cell_u, 0.0, origin.z - cell_v), PI)
						.with_scale(scale),
				));
			}
		}
		// Left ridge-long: complement at the rectangle's ridge corner.
		(EndSide::Left, false) => {
			let origin = Vec3::new(x_min + full_u - u, 0.0, z_ref - full_v + v);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
				Placement::new(origin, PI).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
					Placement::new(Vec3::new(origin.x - cell_u, 0.0, origin.z + cell_v), 0.0)
						.with_scale(scale),
				));
			}
		}
		// Right eave-long: primary at the rectangle edge.
		(EndSide::Right, true) => {
			let origin = Vec3::new(x_min + u, 0.0, z_ref - v);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
				Placement::new(origin, 0.0).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
					Placement::new(Vec3::new(origin.x + cell_u, 0.0, origin.z - cell_v), PI)
						.with_scale(scale),
				));
			}
		}
		// Right ridge-long: mirrored complement at the rectangle's ridge corner.
		(EndSide::Right, false) => {
			let origin = Vec3::new(x_min + u, 0.0, z_ref - full_v + v);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
				Placement::new(origin, PI).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
					Placement::new(Vec3::new(origin.x + cell_u, 0.0, origin.z + cell_v), 0.0)
						.with_scale(scale),
				));
			}
		}
		// Top upright (top-long): tip RA, scale(u,v), yaw 0 only.
		(EndSide::Top, true) => {
			let tip_z = z_ref + full_u;
			let origin = Vec3::new(x_min + v, 0.0, tip_z - u);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
				Placement::new(origin, 0.0).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
					Placement::new(Vec3::new(origin.x + cell_u, 0.0, origin.z - cell_v), PI)
						.with_scale(scale),
				));
			}
		}
		// Top flipped (rect-long): rect RA, scale(u,v), yaw π only.
		(EndSide::Top, false) => {
			let origin = Vec3::new(x_min + full_v - v, 0.0, z_ref + u);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
				Placement::new(origin, PI).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
					Placement::new(Vec3::new(origin.x - cell_u, 0.0, origin.z + cell_v), 0.0)
						.with_scale(scale),
				));
			}
		}
		// Bottom upright (bottom-long): tip RA, scale(u,v), yaw π only.
		(EndSide::Bottom, true) => {
			let tip_z = z_ref - full_u;
			let origin = Vec3::new(x_min + v, 0.0, tip_z + u);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
				Placement::new(origin, PI).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
					Placement::new(Vec3::new(origin.x + cell_u, 0.0, origin.z + cell_v), 0.0)
						.with_scale(scale),
				));
			}
		}
		// Bottom flipped (rect-long): rect RA, scale(u,v), yaw 0 only.
		(EndSide::Bottom, false) => {
			let origin = Vec3::new(x_min + v, 0.0, z_ref - u);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
				Placement::new(origin, 0.0).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
					Placement::new(Vec3::new(origin.x + cell_u, 0.0, origin.z - cell_v), PI)
						.with_scale(scale),
				));
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn rectangle_only_fits_tiles_to_length() -> anyhow::Result<()> {
		let pieces = Quad::new(2.0, 1.0).with_length(3.0).decompose(PanelStyle::TRIANGLES_ONLY);
		assert_eq!(pieces.len(), 6);
		assert_eq!(pieces[0].translation().x, 0.0);
		assert_eq!(pieces[0].scale(), Vec3::new(1.0, 1.0, 2.0));
		assert_eq!(pieces[2].translation().x, 1.0);
		Ok(())
	}

	#[test]
	fn with_rectangle_style_emits_rect_tiles() -> anyhow::Result<()> {
		let pieces = Quad::new(2.0, 1.0).with_length(3.0).decompose(PanelStyle::WITH_RECTANGLE);
		assert_eq!(pieces.len(), 3);
		assert!(pieces.iter().all(|p| matches!(p.geom, PanelGeometry::Rectangle(_))));
		Ok(())
	}

	#[test]
	fn left_end_shifts_rectangle() -> anyhow::Result<()> {
		let pieces = Quad::new(1.0, 1.0)
			.with_length(2.0)
			.with_left(0.5)
			.decompose(PanelStyle::TRIANGLES_ONLY);
		assert!(matches!(
			pieces[0].geom,
			PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) })
		));
		assert!((pieces[0].translation().x - 0.5).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn edge_even_split_emits_four_half_scale_tris() -> anyhow::Result<()> {
		// One edge only: 3 co-oriented half-scale + 1 complement.
		let pieces = Quad::new(2.0, 1.0).with_left(1.0).decompose(PanelStyle::TRIANGLES_ONLY);
		assert_eq!(pieces.len(), 4);
		assert!(pieces.iter().all(|p| {
			let s = p.scale();
			(s.x - 0.5).abs() < 1e-4 && (s.z - 1.0).abs() < 1e-4
		}));
		Ok(())
	}

	#[test]
	fn top_attaches_to_rectangle_and_reaches_tip() -> anyhow::Result<()> {
		// top=0.5, depth=2, length=4 → tip at Z = 0; scale(u,v) with yaw 0.
		let pieces = Quad::new(2.0, 1.0)
			.with_length(4.0)
			.with_top(0.5)
			.decompose(PanelStyle::TRIANGLES_ONLY);
		let top: Vec<_> = pieces
			.iter()
			.filter(|p| (p.scale().x - 0.25).abs() < 1e-3)
			.collect();
		assert_eq!(top.len(), 4);
		assert!(
			top.iter().all(|p| {
				let y = p.yaw().abs();
				y < 1e-3 || (y - PI).abs() < 1e-3
			}),
			"top must not use ±π/2 yaw"
		);
		assert!(top.iter().all(|p| (p.scale().z - 2.0).abs() < 1e-3));
		let tip_z = top.iter().map(|p| p.translation().z).fold(f32::NEG_INFINITY, f32::max);
		assert!((tip_z - 0.0).abs() < 1e-4, "top upright RA on tip, tip_z={tip_z}");
		Ok(())
	}

	#[test]
	fn bottom_span_includes_only_flipped_ends() -> anyhow::Result<()> {
		// left=+1 (eave only), right=-0.5 (ridge only), length=4 → ridge x in [1, 5.5]
		let (x_min, span) = eave_ridge_span(Some(1.0), Some(4.0), Some(-0.5), false);
		assert!((x_min - 1.0).abs() < 1e-4);
		assert!((span - 4.5).abs() < 1e-4);

		let pieces = Quad::new(2.0, 1.0)
			.with_length(4.0)
			.with_left(1.0)
			.with_right(-0.5)
			.with_bottom(-0.25)
			.decompose(PanelStyle::TRIANGLES_ONLY);
		let bottom: Vec<_> = pieces
			.iter()
			.filter(|p| (p.scale().x - 0.125).abs() < 1e-3)
			.collect();
		assert_eq!(bottom.len(), 4);
		assert!(
			bottom.iter().all(|p| {
				let y = p.yaw().abs();
				y < 1e-3 || (y - PI).abs() < 1e-3
			}),
			"bottom must not use ±π/2 yaw"
		);
		assert!(bottom.iter().all(|p| (p.scale().z - 2.25).abs() < 1e-3));
		let origin_x_min = bottom.iter().map(|p| p.translation().x).fold(f32::INFINITY, f32::min);
		assert!(
			(origin_x_min - 1.0).abs() < 1e-4,
			"bottom should start after upright left, x_min={origin_x_min}"
		);
		Ok(())
	}

	#[test]
	fn top_span_includes_only_upright_ends() -> anyhow::Result<()> {
		// left=+1 (eave), right=-0.5 (ridge only), length=4 → eave x in [0, 5]
		let (x_min, span) = eave_ridge_span(Some(1.0), Some(4.0), Some(-0.5), true);
		assert!((x_min - 0.0).abs() < 1e-4);
		assert!((span - 5.0).abs() < 1e-4);

		let pieces = Quad::new(2.0, 1.0)
			.with_length(4.0)
			.with_left(1.0)
			.with_right(-0.5)
			.with_top(0.5)
			.decompose(PanelStyle::TRIANGLES_ONLY);
		let top: Vec<_> = pieces
			.iter()
			.filter(|p| (p.scale().x - 0.25).abs() < 1e-3 && (p.scale().z - 2.5).abs() < 1e-3)
			.collect();
		assert_eq!(top.len(), 4);
		let origin_x_min = top.iter().map(|p| p.translation().x).fold(f32::INFINITY, f32::min);
		assert!((origin_x_min - 0.0).abs() < 1e-4);
		Ok(())
	}
}
