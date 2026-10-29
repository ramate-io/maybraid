//! Ground newly spawned agents on composed Durham height.

use bevy::prelude::*;
use characters::LocomotionCapsule;
use durham::DurhamSurface;
use lod::hcsg::HcsgStorage;
use mob_intelligence::{MobRespawnSupport, MobSlot};
use mob_scenes::MobSceneSystems;
use poi_intelligence::PoiSupport;
use richmond::PaddedTerrain;

pub(crate) struct SpawnSurfacePlugin;

impl Plugin for SpawnSurfacePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			lift_new_mob_members_above_composed
				.after(MobSceneSystems::Respawn)
				.in_set(MobSceneSystems::Surface),
		);
	}
}

/// Keep authored floor arrivals and their existing member offset. Terrain
/// members, including initial spawns, lift onto published padded ground when
/// available, retaining the legacy composed-ground correction elsewhere.
fn lift_new_mob_members_above_composed(
	surface: DurhamSurface,
	storage: Option<Res<HcsgStorage>>,
	mut members: Query<(&mut Transform, Option<&MobRespawnSupport>), Added<MobSlot>>,
) {
	let clearance = LocomotionCapsule::HUMANOID.spawn_height();
	for (mut transform, support) in &mut members {
		if support.is_some_and(|support| support.0 == PoiSupport::Floor) {
			continue;
		}
		type Ground = terrain_layer_model::OnTerrain<durham::Durham>;
		let point = transform.translation;
		let padded = match storage
			.as_deref()
			.map(|storage| PaddedTerrain::<Ground>::published_height_at(storage, point.x, point.z))
		{
			Some(Ok(height)) => height,
			Some(Err(_)) => continue,
			None => None,
		};
		if let Some(y) = padded {
			transform.translation.y = transform.translation.y.max(y + clearance);
		} else {
			transform.translation = surface.lift_origin_above_composed(point, clearance);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use durham::{BaseTerrainNoise, Terrain, TerrainCellLayout, TerrainConfig, WorldBaseTerrain};
	use lod::gen::Id;
	use lod::hcsg::HcsgStorage;
	use std::sync::Arc;

	fn surface_world(publish_origin: bool) -> World {
		let mut world = World::new();
		let layout = TerrainCellLayout::default();
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		let storage = HcsgStorage::default();
		if publish_origin {
			let terrain = Terrain::base_cell_for_test(&layout, 0, 0, base.clone());
			let cell = terrain.cell;
			storage.publish(Id::from_cell(cell), Arc::new(terrain), cell);
		}
		world.insert_resource(storage);
		world.insert_resource(layout);
		world.insert_resource(WorldBaseTerrain(base));
		world
	}

	#[test]
	fn padded_ground_keeps_respawn_offset_and_corrects_initial_terrain_members(
	) -> anyhow::Result<()> {
		use richmond::{PadComplex, TerrainWithPads};
		let mut world = surface_world(true);
		let layout = world.resource::<TerrainCellLayout>();
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		let terrain = Terrain::base_cell_for_test(layout, 0, 0, base);
		let raw = terrain.sdf.terrain().height_at_with_all_modulations(8.0, 8.0);
		let height = raw - 40.0;
		let pad = PadComplex::building_skirt(
			Vec2::new(8.0, 8.0),
			Vec2::splat(20.0),
			0.0,
			height,
			Default::default(),
		);
		let padded = TerrainWithPads::compose(&terrain, [&pad]);
		let cell = terrain.cell;
		type Ground = terrain_layer_model::OnTerrain<durham::Durham>;
		world.resource::<HcsgStorage>().publish(
			Id::from_cell(cell),
			Arc::new(PaddedTerrain::<Ground>::new(padded)),
			cell,
		);
		let respawn_pose = Vec3::new(8.0, height + 2.5, 8.0);
		let respawn = world
			.spawn((
				Transform::from_translation(respawn_pose),
				MobSlot(0),
				MobRespawnSupport(PoiSupport::Terrain),
			))
			.id();
		let initial = world
			.spawn((
				Transform::from_xyz(8.0, height - 100.0, 8.0),
				MobSlot(1),
				MobRespawnSupport(PoiSupport::Terrain),
			))
			.id();
		world
			.run_system_once(lift_new_mob_members_above_composed)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let kept = world
			.get::<Transform>(respawn)
			.ok_or_else(|| anyhow::anyhow!("respawn should exist"))?
			.translation;
		let raised = world
			.get::<Transform>(initial)
			.ok_or_else(|| anyhow::anyhow!("initial member should exist"))?
			.translation;
		assert_eq!(kept, respawn_pose);
		assert!((raised.y - (height + LocomotionCapsule::HUMANOID.spawn_height())).abs() < 1e-3);
		assert!(raised.y < raw);
		Ok(())
	}

	#[test]
	fn new_mob_members_lift_onto_composed_height() -> anyhow::Result<()> {
		let mut world = surface_world(true);
		let buried = world.spawn((Transform::from_xyz(8.0, 4.0, 8.0), MobSlot(0))).id();
		let floor_pose = Vec3::new(24.0, 18.0, 24.0);
		let floor = world
			.spawn((
				Transform::from_translation(floor_pose),
				MobSlot(1),
				MobRespawnSupport(PoiSupport::Floor),
			))
			.id();
		world
			.run_system_once(lift_new_mob_members_above_composed)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let mut state = bevy::ecs::system::SystemState::<DurhamSurface>::new(&mut world);
		let surface = state.get(&world)?;
		let ground = surface
			.composed_height_at(Vec2::new(8.0, 8.0))
			.ok_or_else(|| anyhow::anyhow!("cell should be published"))?;
		drop(state);

		let raised = world
			.get::<Transform>(buried)
			.ok_or_else(|| anyhow::anyhow!("body should exist"))?
			.translation;
		assert_eq!(raised.xz(), Vec2::new(8.0, 8.0));
		assert!((raised.y - (ground + LocomotionCapsule::HUMANOID.spawn_height())).abs() < 1e-4);
		assert!(raised.y > 4.0);
		assert_eq!(
			world.get::<Transform>(floor).map(|transform| transform.translation),
			Some(floor_pose)
		);

		Ok(())
	}

	#[test]
	fn unpublished_cells_keep_the_authored_mob_pose() -> anyhow::Result<()> {
		let mut world = surface_world(false);
		let pose = Vec3::new(8.0, 4.0, 8.0);
		let member = world.spawn((Transform::from_translation(pose), MobSlot(0))).id();
		world
			.run_system_once(lift_new_mob_members_above_composed)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world.get::<Transform>(member).map(|transform| transform.translation),
			Some(pose)
		);
		Ok(())
	}
}
