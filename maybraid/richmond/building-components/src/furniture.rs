//! Furniture / fixture scene components (placeholder wireframes).
//!
//! IR: [`FurnitureStyle`] + [`FurnitureGeometry`] + [`Placement`] + optional
//! [`FurnitureAbutment`] / finish seed → [`FurnitureNode`].

pub mod abutment;
pub mod geometry;
pub mod node;
pub mod style;
pub mod wireframe;

pub use abutment::FurnitureAbutment;
pub use geometry::FurnitureGeometry;
pub use node::{finish_seed_for, FurnitureNode};
pub use style::FurnitureStyle;
pub use wireframe::{FurnitureWireframeAssets, FurnitureWireframePlugin};
