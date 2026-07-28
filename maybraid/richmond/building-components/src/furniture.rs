//! Furniture / fixture scene components (placeholder wireframes).
//!
//! IR: [`FurnitureStyle`] + [`FurnitureGeometry`] + [`Placement`] → [`FurnitureNode`].

pub mod geometry;
pub mod node;
pub mod style;
pub mod wireframe;

pub use geometry::FurnitureGeometry;
pub use node::FurnitureNode;
pub use style::FurnitureStyle;
pub use wireframe::{FurnitureWireframeAssets, FurnitureWireframePlugin};
