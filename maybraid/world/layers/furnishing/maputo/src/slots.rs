//! World-space furniture a generate pass reads from the development index.

use bevy::ecs::system::{ReadOnlySystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::Res;
use building_components::FurnitureNode;
use furniture_usage_areas::expand_usages;
use lod::gen::{Id, Version};
use richmond::{BuiltDevelopment, DevelopmentEntryStore, DevelopmentHosts};
use urbanization_layer_model::Urbanization;

use crate::cell::world_slot;

/// One built development's world-space High slots.
///
/// `id` is the development-index cell (Richmond's development cell). Maputo
/// caches and bins from that id.
pub struct FurnishedDevelopment {
	pub id: Id,
	pub version: Version,
	pub slots: Vec<FurnitureNode>,
}

/// What Maputo reads from the ground under it.
///
/// World-space slots for each built development overlapping a region, plus the
/// development index those cell ids come from. Urbanization's contract does
/// not grow a furniture method for this.
pub trait FurnitureSlots: Send + Sync + 'static {
	type Read: ReadOnlySystemParam + 'static;

	fn slots_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<FurnishedDevelopment>;
}

impl<T: 'static> FurnitureSlots for Urbanization<richmond::Richmond<T>> {
	type Read = Res<'static, DevelopmentEntryStore>;

	fn slots_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<FurnishedDevelopment> {
		read.developments_overlapping_tracked(region)
			.into_iter()
			.map(|(id, version, development)| FurnishedDevelopment {
				id,
				version,
				slots: world_slots_of(development),
			})
			.collect()
	}
}

fn world_slots_of(development: &BuiltDevelopment) -> Vec<FurnitureNode> {
	let mut out = Vec::new();
	for host in development.hosts() {
		let transform = host.transform();
		for node in host.furniture_nodes() {
			out.push(world_slot(transform, node));
		}
		for node in expand_usages(host.furniture_usage_nodes()) {
			out.push(world_slot(transform, node));
		}
	}
	out
}
