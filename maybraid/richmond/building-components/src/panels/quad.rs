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
/// (eave-long / top-long for left/right); negative = flipped (ridge-long / bottom-long).
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
			if base.abs() > 1e-6 {
				let span = self.length.unwrap_or(0.0);
				if span > 1e-6 {
					out.extend(edge_triangles(
						EndSide::Top,
						rect_x0,
						0.0,
						base,
						span,
						tile_width,
						self.tile_height,
					));
				}
			}
		}

		if let Some(base) = self.bottom {
			if base.abs() > 1e-6 {
				let span = self.length.unwrap_or(0.0);
				if span > 1e-6 {
					let z_min = rect_z0 - depth;
					out.extend(edge_triangles(
						EndSide::Bottom,
						rect_x0,
						z_min,
						base,
						span,
						tile_width,
						self.tile_height,
					));
				}
			}
		}

		out
	}
}

fn tile_scale(width: f32, run: f32) -> Vec3 {
	Vec3::new(width.max(1e-4), 1.0, run.max(1e-4))
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
/// Left upright and right flipped use [`MirrorAxis::X`] with positive scale so
/// materials stay single-sided. Top/bottom are the same poses rotated 90° about \(+Y\).
fn edge_triangles(
	side: EndSide,
	x_min: f32,
	z_ref: f32,
	base: f32,
	altitude: f32,
	tile_width: f32,
	tile_height: Option<f32>,
) -> Vec<Placed<PanelGeometry>> {
	let width = base.abs().max(1e-4);
	let altitude = altitude.max(1e-4);
	let nx = fitted_tile_count(width, tile_width);
	let nz = tile_height.map(|th| fitted_tile_count(altitude, th)).unwrap_or(1);
	let dw = width / nx as f32;
	let dh = altitude / nz as f32;

	// Similar-triangle grid: nz rows × (increasing count) of small right triangles
	// of size dw×dh covering the big right triangle of size width×altitude.
	// When nx != nz we still use an nx×nz bounding grid and keep cells under the hypotenuse.
	let mut out = Vec::new();
	for j in 0..nz {
		let row_frac = j as f32 / nz as f32;
		let next_frac = (j + 1) as f32 / nz as f32;
		// Columns that intersect this altitude band under the hypotenuse (from right-angle).
		let i_max = ((1.0 - row_frac) * nx as f32).ceil() as u32;
		let i_lim = i_max.min(nx).max(1);
		for i in 0..i_lim {
			let col_frac = i as f32 / nx as f32;
			let next_col = (i + 1) as f32 / nx as f32;
			// Skip cells whose lower-left is outside the triangle.
			if col_frac + row_frac >= 1.0 - 1e-5 {
				continue;
			}
			let u = i as f32 * dw;
			let v = j as f32 * dh;
			let cell_w = dw;
			let cell_h = dh;
			// On the diagonal band, emit one triangle; interior bands get a full dual pair
			// clipped by only emitting the lower triangle when the cell crosses the hypotenuse.
			let fully_inside = next_col + next_frac <= 1.0 + 1e-5;
			push_edge_cell(
				&mut out,
				side,
				x_min,
				z_ref,
				base >= 0.0,
				u,
				v,
				cell_w,
				cell_h,
				width,
				altitude,
				fully_inside,
			);
		}
	}
	if out.is_empty() {
		// Degenerate fit: single triangle covering the whole edge.
		push_edge_cell(
			&mut out,
			side,
			x_min,
			z_ref,
			base >= 0.0,
			0.0,
			0.0,
			width,
			altitude,
			width,
			altitude,
			false,
		);
	}
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
	cell_w: f32,
	cell_h: f32,
	_full_w: f32,
	_full_h: f32,
	fully_inside: bool,
) {
	let scale = tile_scale(cell_w, cell_h);
	match (side, upright) {
		// Left eave-long: right angle on the rectangle edge, mirrored on X.
		(EndSide::Left, true) => {
			let origin = Vec3::new(x_min + _full_w - u, 0.0, z_ref - v);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
				Placement::new(origin, 0.0).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
					Placement::new(Vec3::new(origin.x - cell_w, 0.0, origin.z - cell_h), PI)
						.with_scale(scale),
				));
			}
		}
		// Left ridge-long: complement at the rectangle's ridge corner.
		(EndSide::Left, false) => {
			let origin = Vec3::new(x_min + _full_w - u, 0.0, z_ref - _full_h + v);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
				Placement::new(origin, PI).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
					Placement::new(Vec3::new(origin.x - cell_w, 0.0, origin.z + cell_h), 0.0)
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
					Placement::new(Vec3::new(origin.x + cell_w, 0.0, origin.z - cell_h), PI)
						.with_scale(scale),
				));
			}
		}
		// Right ridge-long: mirrored complement at the rectangle's ridge corner.
		(EndSide::Right, false) => {
			let origin = Vec3::new(x_min + u, 0.0, z_ref - _full_h + v);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
				Placement::new(origin, PI).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
					Placement::new(Vec3::new(origin.x + cell_w, 0.0, origin.z + cell_h), 0.0)
						.with_scale(scale),
				));
			}
		}
		// Top upright: rotate left-upright 90° about +Y (depth along +X of local kit → -Z world after yaw).
		(EndSide::Top, true) => {
			let origin = Vec3::new(x_min + v, 0.0, z_ref + _full_w - u);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
				Placement::new(origin, -PI * 0.5).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
					Placement::new(Vec3::new(origin.x + cell_h, 0.0, origin.z - cell_w), PI * 0.5)
						.with_scale(scale),
				));
			}
		}
		(EndSide::Top, false) => {
			let origin = Vec3::new(x_min + _full_h - v, 0.0, z_ref + _full_w - u);
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
				Placement::new(origin, PI * 0.5).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
					Placement::new(Vec3::new(origin.x - cell_h, 0.0, origin.z - cell_w), -PI * 0.5)
						.with_scale(scale),
				));
			}
		}
		// Bottom upright / flipped: mirror of top, past the ridge.
		(EndSide::Bottom, true) => {
			let origin = Vec3::new(x_min + v, 0.0, z_ref - (_full_w - u));
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
				Placement::new(origin, PI * 0.5).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
					Placement::new(Vec3::new(origin.x + cell_h, 0.0, origin.z + cell_w), -PI * 0.5)
						.with_scale(scale),
				));
			}
		}
		(EndSide::Bottom, false) => {
			let origin = Vec3::new(x_min + _full_h - v, 0.0, z_ref - (_full_w - u));
			out.push(placed_geom(
				PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
				Placement::new(origin, -PI * 0.5).with_scale(scale),
			));
			if fully_inside {
				out.push(placed_geom(
					PanelGeometry::RightTriangle(RightTriangle { mirror: Some(MirrorAxis::X) }),
					Placement::new(Vec3::new(origin.x - cell_h, 0.0, origin.z + cell_w), PI * 0.5)
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
}
