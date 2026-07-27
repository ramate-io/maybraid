//! Roof / cap scene components.
//!
//! Pipeline: [`geometry`] → [`geometry_components`] → named roof scene types.

pub mod geometry;
pub mod geometry_components;
pub mod rough_stone_perch_roof;
pub mod rough_stone_spire_roof;
pub mod scene;
pub mod wood_perch_deck;

pub use geometry::*;
pub use geometry_components::RoofComponent;
pub use rough_stone_perch_roof::RoughStonePerchRoof;
pub use rough_stone_spire_roof::RoughStoneSpireRoof;
pub use scene::roof_scene;
pub use wood_perch_deck::WoodPerchDeck;
