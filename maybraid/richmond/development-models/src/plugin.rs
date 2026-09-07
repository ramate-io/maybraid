//! Idempotent plugin for Richmond development models.

use bevy::prelude::*;
use lod::LodRefreshSystems;
use lod_lazy_refs::LodLazyRefsPlugin;
use richmond_building_components::{
	apply_parent_confines, FurnitureWireframePlugin, LabelNode, LabelWireframePlugin,
	MassingSilhouettePlugin,
};
use richmond_building_physics::BuildingWalkColliderPlugin;
use richmond_building_shaders::{RichmondBuildingShadersPlugin, RichmondUrbanMaterialRefPlugin};
use richmond_buildings::wizards_tower::TowerSilhouettePlugin;
use scene_ref::SceneRefPlugin;

use richmond_urbanization::UrbanizationIndex;

use crate::buildings_lod::register_developments_buildings_lod_plugin;
use crate::config::DevelopmentConfig;
use crate::index::DevelopmentEntryStore;
use crate::place::DiscoverablePlace;
use crate::presentation::PaddedTerrainPresenterState;

/// Registers SceneRef, urban MaterialRef, placeholder wireframes, building LOD, and walk colliders.
#[derive(Default)]
pub struct RichmondDevelopmentModelsPlugin;

/// Idempotent registration of [`RichmondDevelopmentModelsPlugin`].
pub fn register_richmond_development_models_plugin(app: &mut App) {
	if app.is_plugin_added::<RichmondDevelopmentModelsPlugin>() {
		return;
	}
	app.add_plugins(RichmondDevelopmentModelsPlugin);
}

impl Plugin for RichmondDevelopmentModelsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SceneRefPlugin>() {
			app.add_plugins(SceneRefPlugin);
		}
		if !app.is_plugin_added::<LodLazyRefsPlugin>() {
			app.add_plugins(LodLazyRefsPlugin);
		}
		if !app.is_plugin_added::<RichmondBuildingShadersPlugin>() {
			app.add_plugins(RichmondBuildingShadersPlugin);
		}
		if !app.is_plugin_added::<RichmondUrbanMaterialRefPlugin>() {
			app.add_plugins(RichmondUrbanMaterialRefPlugin);
		}
		if !app.is_plugin_added::<FurnitureWireframePlugin>() {
			app.add_plugins(FurnitureWireframePlugin);
		}
		if !app.is_plugin_added::<LabelWireframePlugin>() {
			app.add_plugins(LabelWireframePlugin);
		}
		if !app.is_plugin_added::<TowerSilhouettePlugin>() {
			app.add_plugins(TowerSilhouettePlugin);
		}
		if !app.is_plugin_added::<MassingSilhouettePlugin>() {
			app.add_plugins(MassingSilhouettePlugin);
		}
		if !app.is_plugin_added::<BuildingWalkColliderPlugin>() {
			app.add_plugins(BuildingWalkColliderPlugin);
		}
		register_developments_buildings_lod_plugin(app);

		app.init_resource::<DevelopmentEntryStore>()
			.init_resource::<DevelopmentConfig>()
			.init_resource::<UrbanizationIndex>()
			.init_resource::<PaddedTerrainPresenterState>()
			.add_systems(Update, apply_parent_confines.after(LodRefreshSystems::Cull))
			.add_systems(Update, stamp_label_places);
	}
}

/// High usage-area labels become extra local places while those children exist.
fn stamp_label_places(
	mut commands: Commands,
	added: Query<(Entity, &LabelNode), (Added<LabelNode>, Without<DiscoverablePlace>)>,
) {
	for (entity, node) in &added {
		if let Some(place) = DiscoverablePlace::from_label_node(node) {
			commands.entity(entity).insert(place);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_building_components::{LabelGeometry, LabelStyle, Placement};

	#[test]
	fn high_lounge_label_receives_a_place() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_systems(Update, stamp_label_places);
		let lounge = app
			.world_mut()
			.spawn(LabelNode::new(
				LabelStyle::Gray,
				LabelGeometry::rectangle(bevy::math::Vec3::new(5.0, 3.0, 4.0)),
				"Lounge",
				Placement::default(),
			))
			.id();
		let furniture = app
			.world_mut()
			.spawn(LabelNode::new(
				LabelStyle::Blue,
				LabelGeometry::rectangle(bevy::math::Vec3::ONE),
				"Nightstand",
				Placement::default(),
			))
			.id();

		app.update();

		let place = app
			.world()
			.get::<DiscoverablePlace>(lounge)
			.ok_or_else(|| anyhow::anyhow!("lounge should become a place"))?;
		assert_eq!(place.label, crate::DiscoverablePlaceLabel::Lounge);
		assert!(!place.persistent);
		assert!(app.world().get::<DiscoverablePlace>(furniture).is_none());
		Ok(())
	}
}
