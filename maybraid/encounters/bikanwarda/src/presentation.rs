//! HCSG presentation plugin for boss encounters.
//!
//! Presents generated boss encounters within channel regions. Each boss gets
//! a unique host with its own presentation lifecycle.
//!
//! Column and pedestal are invisible collision placeholders grounded at published
//! terrain Y. They provide spatial structure for future chain tether attachment.
//! Visual meshes are TODO. Chain attachment is deferred to #1617.

use std::marker::PhantomData;

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use bikanwarda_character::{encounter_meters, BikanwardaSpec};
use damage::Health;
use encounters::EncounterInstance;
use lod::hcsg::PresentationPlugin;
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel, LodSceneStatus};

use crate::generation::BikanwardaBossEncounter;
use crate::identity::chained_encounter_id;

/// **PLACEHOLDER**: Boss health for chained encounter.
///
/// TODO: Derive from combat tuning / boss recipe once encounter balancing is complete.
/// Current value chosen to allow multiple player attack cycles for testing.
pub const PLACEHOLDER_BOSS_HEALTH: f32 = 1000.0;

/// Marks the root of a presented boss encounter.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BossEncounterRoot;

/// Marker requesting actual boss assembly spawn.
#[derive(Component, Clone, Debug, Default)]
pub struct RequestBossAssembly {
	pub spec: BikanwardaSpec,
}

/// Relationship from encounter host to its spawned boss actor.
///
/// The host owns BossEncounterRoot, Health, and EncounterInstance.
/// The actor (child entity) owns BikanwardaRoot, actor components, and attack systems.
/// This relationship enables combat/attack/tether queries to find the actor from the host.
#[derive(Component, Clone, Copy, Debug)]
pub struct BossActor(pub Entity);

/// Invisible collision placeholder for chain tether column.
///
/// Scaled from the authored 15 m × 1 m cylinder, grounded at terrain Y on the
/// pedestal. Visual mesh TODO. Chain attachment depends on #1617.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BossColumn;

/// Invisible collision placeholder for pedestal at column base.
///
/// Scaled from the authored 3 m × 2 m cylinder, grounded at terrain Y.
/// Visual mesh TODO.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BossPedestal;

/// Authored 8 m XZ offset from boss to pillar, at live encounter scale.
pub const COLUMN_OFFSET_XZ: Vec2 = Vec2::new(encounter_meters(8.0), 0.0);
pub const PEDESTAL_RADIUS: f32 = encounter_meters(2.0);
pub const PEDESTAL_HEIGHT: f32 = encounter_meters(3.0);
pub const COLUMN_RADIUS: f32 = encounter_meters(1.0);
pub const COLUMN_HEIGHT: f32 = encounter_meters(15.0);

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
		let terrain_y = self.terrain_y;
		let spec = self.boss.value.actor.recipe.spec.clone();

		// Column stands BESIDE the boss (not underneath), both grounded at terrain Y.
		let column_offset_xz = COLUMN_OFFSET_XZ;

		// Child transforms are LOCAL to root. Root is at boss position (terrain_y + hull_offset).
		// Pedestal and column grounded at terrain_y, so their local Y = terrain_y - position.y
		let local_ground_y = terrain_y - position.y;

		let pedestal_local_y = local_ground_y + PEDESTAL_HEIGHT * 0.5;
		let column_local_y = local_ground_y + PEDESTAL_HEIGHT + COLUMN_HEIGHT * 0.5;

		// Build child scenes for pedestal and column as separate entities
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(bsn! {
				Name::new("boss-pedestal")
				BossPedestal
				Transform::from_translation(Vec3::new(column_offset_xz.x, pedestal_local_y, column_offset_xz.y))
				Visibility::default()
				Collider::cylinder(PEDESTAL_RADIUS, PEDESTAL_HEIGHT)
				RigidBody::Static
			}),
			Box::new(bsn! {
				Name::new("boss-column")
				BossColumn
				Transform::from_translation(Vec3::new(column_offset_xz.x, column_local_y, column_offset_xz.y))
				Visibility::default()
				Collider::cylinder(COLUMN_RADIUS, COLUMN_HEIGHT)
				RigidBody::Static
			}),
		];

		// Boss health (placeholder - see PLACEHOLDER_BOSS_HEALTH doc)
		let boss_health = Health::from_max(PLACEHOLDER_BOSS_HEALTH);

		// Root entity with boss assembly and children for column/pedestal
		bsn! {
			~{name}
			BossEncounterRoot::default()
			instance
			boss_health
			RequestBossAssembly { spec }
			Transform::from_translation(position)
			Visibility::default()
			Children [ {children} ]
		}
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
	use lod::hcsg::{HcsgBoundsPlugin, HcsgClass, ViewerHcsgBounds};

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
