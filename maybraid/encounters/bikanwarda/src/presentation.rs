//! HCSG presentation plugin for boss encounters.
//!
//! Presents generated boss encounters within channel regions. Each boss gets
//! a unique host with its own presentation lifecycle.
//!
//! Column and pedestal are marker components positioned for chain attachment
//! anchors. Visual geometry (mesh/material) is not yet implemented; markers
//! define the spatial layout only. Actual chain attachment is deferred to #1617.

use std::marker::PhantomData;

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use bikanwarda_character::BikanwardaSpec;
use encounters::EncounterInstance;
use lod::hcsg::PresentationPlugin;
use lod::scene::{LodScene, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;

use crate::generation::BikanwardaBossEncounter;
use crate::identity::chained_encounter_id;

/// Marks the root of a presented boss encounter.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BossEncounterRoot;

/// Marker requesting actual boss assembly spawn.
#[derive(Component, Clone, Debug, Default)]
pub struct RequestBossAssembly {
	pub spec: BikanwardaSpec,
}

/// Placeholder column geometry for chain tether attachment.
///
/// The column provides a visual tethering structure aligned with the boss.
/// Actual chain attachment depends on #1617 (cursor/chain-damage-breakable-3cbf).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BossColumn;

/// Placeholder pedestal geometry at the column base.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BossPedestal;

/// One LOD level: boss character with physics and presentation.
///
/// Includes boss assembly, placeholder column, and pedestal geometry.
/// Column and pedestal are aligned to terrain; chain attachment is deferred to #1617.
impl LodScene for BikanwardaBossEncounter {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}
	
	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		let name = Name::new(format!(
			"boss-encounter-{}",
			chained_encounter_id().slug_str().unwrap_or("unknown")
		));
		let instance = EncounterInstance::new(chained_encounter_id());
		let position = self.boss_position;
		let spec = self.boss.value.actor.recipe.spec.clone();
		
		// Column offset: positioned to the side of the boss (within arena bounds)
		// This creates a tether anchor point for future chain work (#1617)
		let column_offset = Vec3::new(8.0, 0.0, 0.0);
		let column_position = position + column_offset;
		
		(
			bsn! {
				~{name}
				BossEncounterRoot::default()
				instance
				RequestBossAssembly { spec }
				Transform::from_translation(position)
				Visibility::default()
			},
			// Placeholder pedestal at column base (3m high, 2m radius)
			// Grounded at published terrain Y with cylinder collider.
			// TODO: Add visual mesh in a system (Scene trait doesn't have Assets access).
			// Final implementation will use authored geometry; this is a labeled placeholder.
			bsn! {
				Name::new("boss-pedestal")
				BossPedestal
				Transform::from_translation(column_position + Vec3::Y * 1.5)
				Visibility::default()
				Collider::cylinder(2.0, 1.5)
				RigidBody::Static
			},
			// Placeholder column for chain attachment (15m high, 1m radius)
			// Positioned above pedestal; chain anchors will attach at top (#1617).
			// Grounded at published terrain Y (pedestal base) with cylinder collider.
			// TODO: Add visual mesh in a system.
			bsn! {
				Name::new("boss-column")
				BossColumn
				Transform::from_translation(column_position + Vec3::Y * (3.0 + 7.5))
				Visibility::default()
				Collider::cylinder(1.0, 7.5)
				RigidBody::Static
			},
		)
	}
}

/// Presents boss encounters over channel `C` within its regions.
///
/// Each generated encounter gets one host with its own scene. Hosts retire
/// when the viewer leaves the region.
pub struct BossEncounterPresentationPlugin<C>(PhantomData<fn() -> C>);

impl<C> Default for BossEncounterPresentationPlugin<C> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static> Plugin for BossEncounterPresentationPlugin<C> {
	fn build(&self, app: &mut App) {
		app.add_plugins(PresentationPlugin::<C, BikanwardaBossEncounter>::default());
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::app::App;
	use lod::hcsg::{ViewerHcsgBounds, HcsgClass, HcsgBoundsPlugin};

	struct TestChannel;
	
	impl ViewerHcsgBounds for TestChannel {
		const CLASS: HcsgClass = HcsgClass::Near;
		
		fn regions_around(viewer: Vec3) -> Vec<bevy::shape::Aabb3d> {
			vec![bevy::shape::Aabb3d::new(viewer, Vec3::splat(100.0))]
		}
	}

	#[test]
	fn presentation_plugin_builds() {
		let mut app = App::new();
		app.add_plugins((
			bevy::MinimalPlugins,
			HcsgBoundsPlugin::<TestChannel>::default(),
			BossEncounterPresentationPlugin::<TestChannel>::default(),
		));
	}
}
