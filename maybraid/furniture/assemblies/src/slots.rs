//! Slot list a furniture generate pass reads from a built development.

use bevy::prelude::Transform;
use building_components::{FurnitureNode, FurnitureUsageNode};

/// Host-local furniture a higher layer paints into 50 m cells.
pub trait FurnitureSlotSource {
	fn furniture_hosts(&self) -> Vec<(Transform, Vec<FurnitureNode>, Vec<FurnitureUsageNode>)>;
}
