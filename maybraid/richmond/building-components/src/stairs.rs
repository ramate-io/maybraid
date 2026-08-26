//! Stair scene components.
//!
//! IR: [`StairStyle`] + [`StairGeometry`] (a linear [`StraightStair`]) + [`Placement`]
//! → [`StairNode`] (`LodScene`). Circular / rectangular flights are composed by
//! higher-order types as many straight nodes.

pub mod geometry;
pub mod node;
pub mod rough_stonework;
pub mod style;
pub(crate) mod tessellate;
pub mod wood;

pub use geometry::*;
pub use node::StairNode;
pub use rough_stonework::*;
pub use style::StairStyle;
pub use wood::*;
