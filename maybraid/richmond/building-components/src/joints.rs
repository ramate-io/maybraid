//! Joint scene components (circular / post fillers at creases and kinks).
//!
//! IR: [`JointStyle`] + [`JointGeometry`] + [`Placement`] → [`JointNode`] (`LodScene`).

pub mod geometry;
pub mod node;
pub mod rough_stonework;
pub mod style;

pub use geometry::*;
pub use node::JointNode;
pub use rough_stonework::*;
pub use style::JointStyle;
