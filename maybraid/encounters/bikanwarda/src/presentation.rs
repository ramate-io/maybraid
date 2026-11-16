//! HCSG presentation plugin for boss encounters.
//!
//! Presents generated boss encounters within channel regions. Each boss gets
//! a unique host with its own presentation lifecycle.

use std::marker::PhantomData;

use bevy::prelude::*;
use encounters::EncounterInstance;
use lod::hcsg::PresentationPlugin;
use lod::scene::{LodScene, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;

use crate::generation::BikanwardaBossEncounter;
use crate::identity::chained_encounter_id;

/// Marks the root of a presented boss encounter.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BossEncounterRoot;

/// One LOD level: boss character with physics and presentation.
///
/// Future work: integrate with bikanwarda-character spawn and presentation.
/// Placeholder implementation demonstrates the generation contract.
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
		
		bsn! {
			~{name}
			BossEncounterRoot::default()
			instance
			Transform::from_translation(position)
			Visibility::default()
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
