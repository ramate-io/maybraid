//! Shared panel geometry and tessellation (rectangles, right triangles, quads, joints).
//!
//! Panel space is **lower-left** anchored like roof pitch layouts:
//! - **X** along the long edge (length)
//! - **Z** depth / run (front / eave at \(Z = 0\), back / ridge at \(Z = -\texttt{depth}\))
//! - **Y** is kit thickness / height; domain nodes own extra orientation (roof pitch about \(+X\), etc.)
//!
//! Decomposition is style-agnostic and does not require an `LodScene`:
//!
//! ```text
//! QuadPolyline.decompose() → Placed<Quad | Joint>
//! Quad.decompose(policy)   → Placed<Rectangle | RightTriangle>
//! ```

mod geometry;
mod joint;
mod polyline;
mod quad;

pub use geometry::{
	fitted_tile_count, PanelAtom, PanelComposite, PanelGeom, Rectangle, RightTriangle,
	TessellatePolicy, DEFAULT_TILE_WIDTH,
};
pub use joint::{Joint, JOINT_BASE_RADIUS, JOINT_KIT_HALF, JOINT_RADIUS_PER_SLOPE_RAD};
pub use polyline::{
	roll_along_slope, yaw_along_xz, QuadPolyline, DEFAULT_MIN_EDGE_TRIANGLE_ANGLE,
	DEFAULT_MIN_JOINT_ANGLE,
};

pub(crate) use polyline::wrap_pi;
pub use quad::Quad;
