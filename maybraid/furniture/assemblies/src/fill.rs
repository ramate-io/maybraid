//! Map a stamped Richmond slot to a painted assembly, when a kit exists.

use richmond_building_components::{FurnitureGeometry, FurnitureNode};

use crate::bed::BedParams;
use crate::chair::ChairParams;
use crate::chest::ChestParams;
use crate::counter::CounterParams;
use crate::parts::{pose_parts, Assembly, PlacedPart};

/// Paint [`FurnitureGeometry::Bed`] / [`Chair`](FurnitureGeometry::Chair) /
/// [`Chest`](FurnitureGeometry::Chest) / [`Counter`](FurnitureGeometry::Counter).
/// Other kinds stay wireframe-only until they have assemblies.
pub fn try_assembly(node: &FurnitureNode) -> Option<Assembly> {
	let seed = node.finish_seed;
	Some(match node.geometry {
		FurnitureGeometry::Bed => BedParams { finish_seed: seed }.build().assembly(),
		FurnitureGeometry::Chair => ChairParams { finish_seed: seed }.build().assembly(),
		FurnitureGeometry::Chest => ChestParams { finish_seed: seed }.build().assembly(),
		FurnitureGeometry::Counter => CounterParams { finish_seed: seed }.build().assembly(),
		_ => return None,
	})
}

/// Unit-slot parts composed under the slot placement (yaw / scale / abutment).
pub fn posed_assembly(node: &FurnitureNode) -> Option<Vec<PlacedPart>> {
	let assembly = try_assembly(node)?;
	Some(pose_parts(node, &assembly.parts))
}

/// Thin wall just past the kit \(+Z\) face (already aimed at [`FurnitureNode::abutment`]).
pub fn abutment_wall(node: &FurnitureNode) -> Option<richmond_building_components::Placement> {
	node.abutment?;
	Some(node.placement.compose_child(richmond_building_components::Placement {
		translation: bevy::math::Vec3::new(0.0, 0.0, 0.52),
		yaw: 0.0,
		pitch: 0.0,
		roll: 0.0,
		scale: bevy::math::Vec3::new(1.08, 1.08, 0.04),
	}))
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_building_components::{FurnitureAbutment, Placement};

	#[test]
	fn painted_kinds_fill_and_others_do_not() -> anyhow::Result<()> {
		let bed = FurnitureNode::bed(Placement::IDENTITY).with_finish_seed(7);
		if try_assembly(&bed).is_none() {
			return Err(anyhow::anyhow!("bed should paint"));
		}
		let dresser = FurnitureNode::dresser(Placement::IDENTITY).with_finish_seed(7);
		if try_assembly(&dresser).is_some() {
			return Err(anyhow::anyhow!("dresser should stay wireframe-only"));
		}
		Ok(())
	}

	#[test]
	fn wall_follows_abutment_yaw() -> anyhow::Result<()> {
		let slot = FurnitureNode::bed(Placement::IDENTITY).with_abutment(FurnitureAbutment::NegZ);
		let wall = abutment_wall(&slot).ok_or_else(|| anyhow::anyhow!("expected a wall"))?;
		if wall.translation.z >= 0.0 {
			return Err(anyhow::anyhow!(
				"NegZ abutment should put the wall on −Z, got {}",
				wall.translation.z
			));
		}
		Ok(())
	}
}
