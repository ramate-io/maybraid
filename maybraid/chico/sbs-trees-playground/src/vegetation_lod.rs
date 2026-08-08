//! Vegetation LOD refresh: bullseye + spotlight → Avian index → levels → chunk sync.

use avian3d::prelude::{Collider, PhysicsPlugins, RigidBody};
use bevy::prelude::*;
use chico_groves::MonsterGrass;
use chico_vegetation_components::{ComponentsOnly, VegetationStructuralLodProbe};
use lod::{
	Bullseye, LodChunkFulfillBudget, LodRefreshCorePlugin, LodSceneHost,
	LodSceneRefreshRegionPlugin, Spotlight,
};
use lod_avian::AvianLodSceneRefreshPlugin;

/// Channel marker for bullseye [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationBullseye;

/// Channel marker for spotlight [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationSpotlight;

/// Marker on the collider child under a structural vegetation host.
#[derive(Debug, Clone, Copy, Default, Component)]
struct VegetationStructuralCollider;

/// Full modern refresh stack for structural vegetation hosts.
///
/// 1. Camera → [`Bullseye`] / [`Spotlight`] region messages  
/// 2. Avian region index → level messages for [`ComponentsOnly<MonsterGrass>`]  
/// 3. Entity refresh (max fold) + chunk sync / cull
pub struct VegetationLodRefreshPlugin;

impl Plugin for VegetationLodRefreshPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(PhysicsPlugins::default());
		if !app.is_plugin_added::<LodRefreshCorePlugin>() {
			app.add_plugins(LodRefreshCorePlugin);
		}

		app.insert_resource(Bullseye {
			inner: 50.0,
			outer: 500.0,
		})
		.insert_resource(Spotlight { extent: 20.0 })
		.insert_resource(LodChunkFulfillBudget {
			weights_per_frame: 512,
		})
		.add_plugins((
			LodSceneRefreshRegionPlugin::<Bullseye, With<Camera>, VegetationBullseye>::default(),
			LodSceneRefreshRegionPlugin::<Spotlight, With<Camera>, VegetationSpotlight>::default(),
			AvianLodSceneRefreshPlugin::<
				ComponentsOnly<MonsterGrass>,
				VegetationBullseye,
				With<Camera>,
			>::default(),
			AvianLodSceneRefreshPlugin::<
				ComponentsOnly<MonsterGrass>,
				VegetationSpotlight,
				With<Camera>,
			>::default(),
		))
		.add_systems(Update, ensure_vegetation_structural_colliders);
	}
}

/// Attach a static Avian collider child covering the structural footprint.
fn ensure_vegetation_structural_colliders(
	mut commands: Commands,
	hosts: Query<
		(Entity, &VegetationStructuralLodProbe, Option<&Children>),
		With<LodSceneHost>,
	>,
	existing: Query<(), With<VegetationStructuralCollider>>,
) {
	for (entity, probe, children) in &hosts {
		if children
			.is_some_and(|c| c.iter().any(|child| existing.contains(child)))
		{
			continue;
		}
		let r = probe.tree_radius.max(1.0);
		let half_y = r.max(2.0);
		commands.entity(entity).with_children(|parent| {
			parent.spawn((
				VegetationStructuralCollider,
				RigidBody::Static,
				Collider::cuboid(r, half_y, r),
				Transform::from_translation(probe.center),
				Visibility::Hidden,
			));
		});
	}
}
