//! Shared panel geometry and tessellation (rectangles, right triangles, quads, joints).
//!
//! Panel space is **lower-left** anchored like roof pitch layouts:
//! - **X** along the long edge (length)
//! - **Z** depth / run (top/eave at \(Z = 0\), bottom/ridge at \(Z = -\texttt{depth}\))
//! - **Y** is kit thickness; domain nodes own extra orientation (roof pitch about \(+X\), wall stand-up, etc.)
//!
//! Decomposition does not require an `LodScene`:
//!
//! ```text
//! QuadPolyline.decompose()      → Placed<Quad | Joint>
//! PanelGeometry::flatten(style) → Placed<Rectangle | RightTriangle | Joint>
//! ```

mod geometry;
mod joint;
mod kit_space;
mod polyline;
mod quad;

pub use geometry::{
	fitted_tile_count, PanelGeometry, PanelStyle, Rectangle, RightTriangle, DEFAULT_TILE_WIDTH,
};
pub use joint::{Joint, JOINT_BASE_RADIUS, JOINT_KIT_HALF, JOINT_RADIUS_PER_SLOPE_RAD};
pub use kit_space::{to_centered_rect_placement, with_wall_standup_pitch};
pub use polyline::{
	roll_along_slope, yaw_along_xz, EdgePolygons, QuadPolyline, DEFAULT_MIN_EDGE_TRIANGLE_ANGLE,
	DEFAULT_MIN_JOINT_ANGLE,
};

pub(crate) use polyline::wrap_pi;
pub use quad::Quad;
