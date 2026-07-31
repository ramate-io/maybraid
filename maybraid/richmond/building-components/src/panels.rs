//! Shared panel geometry and tessellation (rectangles, right triangles).
//!
//! Panel space is **lower-left** anchored like roof pitch layouts:
//! - **X** along the long edge (length)
//! - **Z** depth / run (top/eave at \(Z = 0\), bottom/ridge at \(Z = -\texttt{depth}\))
//! - **Y** is kit thickness; domain nodes own extra orientation (roof pitch about \(+X\), wall stand-up, etc.)
//!
//! IR: [`PanelStyle`] + [`PanelGeometry`] + [`Placement`] → [`PanelNode`] (`LodScene`).
//!
//! ```text
//! TessellatedTriangle.decompose() → Placed<RightTriangle>
//! PanelGeometry::flatten(caps)    → Placed<Rectangle | RightTriangle>
//! ```
//!
//! Crease / kink fillers are the separate [`crate::joints`] domain (`JointNode`), not panel geometry.
mod geometry;
mod kit_space;
mod node;
mod placement;
mod style;
mod tessellated_triangle;
mod triangle;

pub use geometry::{
	fitted_tile_count, PanelGeometry, PanelKitCaps, Rectangle, RightTriangle, DEFAULT_TILE_WIDTH,
};
pub use kit_space::{to_centered_rect_placement, with_wall_standup_pitch};
pub use node::PanelNode;
pub use placement::{roll_along_slope, yaw_along_xz, DEFAULT_MIN_JOINT_ANGLE};
pub use style::PanelStyle;
pub use tessellated_triangle::TessellatedTriangle;
pub use triangle::{dihedral_kink, triangle_normal};

pub(crate) use placement::wrap_pi;
