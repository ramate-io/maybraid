//! Reusable development archetypes beyond the original courtyard and village families.

macro_rules! delegate_components {
	($ty:ty, $field:ident) => {
		impl richmond_building_components::BuildingComponents for $ty {
			fn panel_nodes_for_level(
				&self,
				level: lod::gen::LodSceneLevel,
			) -> richmond_building_components::Layers<richmond_building_components::PanelNode> {
				self.$field.panel_nodes_for_level(level)
			}
			fn partition_nodes_for_level(
				&self,
				level: lod::gen::LodSceneLevel,
			) -> richmond_building_components::Layers<richmond_building_components::PartitionNode>
			{
				self.$field.partition_nodes_for_level(level)
			}
			fn floor_nodes_for_level(
				&self,
				level: lod::gen::LodSceneLevel,
			) -> richmond_building_components::Layers<richmond_building_components::FloorNode> {
				self.$field.floor_nodes_for_level(level)
			}
			fn roof_nodes_for_level(
				&self,
				level: lod::gen::LodSceneLevel,
			) -> richmond_building_components::Layers<richmond_building_components::RoofNode> {
				self.$field.roof_nodes_for_level(level)
			}
			fn stair_nodes_for_level(
				&self,
				level: lod::gen::LodSceneLevel,
			) -> richmond_building_components::Layers<richmond_building_components::StairNode> {
				self.$field.stair_nodes_for_level(level)
			}
			fn door_nodes_for_level(
				&self,
				level: lod::gen::LodSceneLevel,
			) -> richmond_building_components::Layers<richmond_building_components::DoorNode> {
				self.$field.door_nodes_for_level(level)
			}
			fn joint_nodes_for_level(
				&self,
				level: lod::gen::LodSceneLevel,
			) -> richmond_building_components::Layers<richmond_building_components::JointNode> {
				self.$field.joint_nodes_for_level(level)
			}
			fn furniture_nodes_for_level(
				&self,
				level: lod::gen::LodSceneLevel,
			) -> richmond_building_components::Layers<richmond_building_components::FurnitureNode>
			{
				self.$field.furniture_nodes_for_level(level)
			}
			fn label_nodes_for_level(
				&self,
				level: lod::gen::LodSceneLevel,
			) -> richmond_building_components::Layers<richmond_building_components::LabelNode> {
				self.$field.label_nodes_for_level(level)
			}
		}
	};
}

mod highrise;
mod neighborhood;
mod skybridge_bazaar;
mod temple;
mod wizards_tower;

pub use highrise::{
	ApartmentMonotower, SingleHighrise, SingleHighriseFloorPlan, SingleHighrisePlan,
	SingleHighriseShaftSlot, SingleHighriseStorey,
};
pub use neighborhood::SuburbanHomes;
pub use skybridge_bazaar::{Skybridge, SkybridgeBazaar};
pub use temple::TempleComplex;
pub use wizards_tower::SolitaryWizardsTower;

#[cfg(test)]
mod tests {
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;
	use richmond_buildings::{Confines, Fit};

	use super::{SingleHighrise, SolitaryWizardsTower};

	#[test]
	fn solitary_archetypes_use_requested_vertical_envelope() -> anyhow::Result<()> {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-20.0, 0.0, -20.0),
			Vec3::new(20.0, 80.0, 20.0),
		));
		let (highrise, _) = SingleHighrise::fit_to_confines(&confines, NoiseParams::default())?;
		let (wizard, _) = SolitaryWizardsTower::fit_to_confines(&confines, NoiseParams::default())?;
		assert!(highrise.storey_count() >= 20);
		assert!(wizard.bounds.max.y <= confines.bounds.max.y + 1e-3);
		Ok(())
	}
}
