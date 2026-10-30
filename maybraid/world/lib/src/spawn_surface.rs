//! Adapt mob arrivals to the shared support probe and physics hold lifecycle.

use bevy::prelude::*;
use characters::LocomotionCapsule;
use durham::DurhamSurface;
use lod::hcsg::HcsgStorage;
use maybraid_spawns_support::{
	register_actor_support, terrain_height_at, ActorSupportSystems, PendingActorSupport,
	PlacementSearch,
};
use mob_characters::CharacterSceneRecipe;
use mob_intelligence::{MobRespawnSupport, MobSlot};
use mob_scenes::MobSceneSystems;
use poi_intelligence::PoiSupport;

const INITIAL_MOB_SEARCH_RADIUS: f32 = 8.0;

pub(crate) struct SpawnSurfacePlugin;

#[derive(Component)]
struct MobSupportQueued;

impl Plugin for SpawnSurfacePlugin {
	fn build(&self, app: &mut App) {
		register_actor_support(app);
		app.add_systems(
			Update,
			prepare_mob_support
				.after(MobSceneSystems::Respawn)
				.in_set(MobSceneSystems::Surface)
				.in_set(ActorSupportSystems::Prepare),
		);
	}
}

/// Every pending member retries independently. Initial host elevation can be a
/// temporary holding altitude; only respawn metadata contributes an authored
/// minimum body height. Floor support keeps its own bounded band.
fn prepare_mob_support(
	mut commands: Commands,
	surface: DurhamSurface,
	storage: Option<Res<HcsgStorage>>,
	members: Query<
		(
			Entity,
			&Transform,
			Option<&LocomotionCapsule>,
			Option<&CharacterSceneRecipe>,
			Option<&MobRespawnSupport>,
			Option<&PendingActorSupport>,
		),
		(With<MobSlot>, Or<(Without<MobSupportQueued>, With<PendingActorSupport>)>),
	>,
) {
	for (entity, transform, hull, recipe, respawn, pending) in &members {
		let mut support = pending.cloned().unwrap_or_else(|| {
			let hull = hull
				.copied()
				.or_else(|| recipe.map(CharacterSceneRecipe::locomotion_capsule))
				.unwrap_or(LocomotionCapsule::HUMANOID);
			let minimum = respawn.map(|_| transform.translation.y);
			// Materialization may stamp gravity later. The shared runtime captures
			// it only after the dynamic body and collider exist.
			let pending = PendingActorSupport::new(transform.translation, hull, minimum);
			if respawn.is_none() {
				pending.with_nearby_terrain(INITIAL_MOB_SEARCH_RADIUS)
			} else {
				pending
			}
		});
		if support.exhausted() {
			warn!(target: "maybraid.mobs", ?entity,
				at = ?support.held_position(), reason = ?support.failure,
				"omitting mob member after bounded support search exhausted");
			commands.entity(entity).try_despawn();
			continue;
		}
		let point = support.held_position();
		let search_xz = support.search_xz();
		let search = match respawn {
			Some(respawn) if respawn.support == PoiSupport::Floor => respawn.surface_y.map(|y| {
				PlacementSearch::interior_at(Vec3::new(point.x, y, point.z), support.hull())
			}),
			_ => terrain_height_at(&surface, storage.as_deref(), search_xz)
				.map(|y| PlacementSearch::outdoor_hint(search_xz, y, support.hull())),
		};
		support.observe_search(search);
		commands.entity(entity).insert((support, MobSupportQueued));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use avian3d::prelude::{
		Collider, GravityScale, LinearVelocity, PhysicsPlugins, Position, RigidBody,
	};
	use bevy::ecs::system::RunSystemOnce;
	use durham::{BaseTerrainNoise, Terrain, TerrainCellLayout, TerrainConfig, WorldBaseTerrain};
	use lod::gen::Id;
	use lod_avian::PhysicsInteractionLayer;
	use std::sync::Arc;

	fn support_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
		));
		app.insert_resource(TerrainCellLayout::default());
		app.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(
			42,
		))));
		app.init_resource::<HcsgStorage>();
		app.add_plugins(SpawnSurfacePlugin);
		app.finish();
		app
	}

	fn member(app: &mut App, pose: Vec3, slot: u16) -> Entity {
		app.world_mut()
			.spawn((
				Transform::from_translation(pose),
				MobSlot(slot),
				LocomotionCapsule::HUMANOID,
				RigidBody::Dynamic,
				Collider::capsule(0.4, 1.0),
				GravityScale(1.25),
				LinearVelocity(Vec3::new(10.0, -20.0, 4.0)),
			))
			.id()
	}

	fn fixed_floor(app: &mut App, xz: Vec2, height: f32) {
		app.world_mut().spawn((
			Transform::from_xyz(xz.x, height - 0.1, xz.y),
			RigidBody::Static,
			Collider::cuboid(4.0, 0.1, 4.0),
			PhysicsInteractionLayer::fixed_layers(),
		));
	}

	fn publish_origin(app: &mut App) -> f32 {
		let terrain = Terrain::base_cell_for_test(
			app.world().resource::<TerrainCellLayout>(),
			0,
			0,
			app.world().resource::<WorldBaseTerrain>().0.clone(),
		);
		let height = terrain.sdf.terrain().height_at_with_all_modulations(8.0, 8.0);
		let cell = terrain.cell;
		app.world()
			.resource::<HcsgStorage>()
			.publish(Id::from_cell(cell), Arc::new(terrain), cell);
		height
	}

	fn advance(app: &mut App) {
		for _ in 0..4 {
			app.update();
		}
	}

	#[test]
	fn unpublished_members_retry_independently_and_drop_from_holding_altitude() -> anyhow::Result<()>
	{
		let mut app = support_app();
		let pose = Vec3::new(8.0, 1_500.0, 8.0);
		let near = member(&mut app, pose, 0);
		let far = member(&mut app, Vec3::new(80_008.0, 1_500.0, 8.0), 1);
		advance(&mut app);
		assert!(app.world().get::<PendingActorSupport>(near).is_some());
		assert!(app.world().get::<PendingActorSupport>(far).is_some());
		assert_eq!(app.world().get::<Transform>(near).map(|t| t.translation), Some(pose));
		assert_eq!(app.world().get::<GravityScale>(near).map(|g| g.0), Some(0.0));
		assert_eq!(app.world().get::<LinearVelocity>(near).map(|v| v.0), Some(Vec3::ZERO));

		let ground = publish_origin(&mut app);
		advance(&mut app);
		assert!(
			app.world().get::<PendingActorSupport>(near).is_some(),
			"height alone is insufficient"
		);
		fixed_floor(&mut app, pose.xz(), ground);
		advance(&mut app);
		assert!(app.world().get::<PendingActorSupport>(near).is_none());
		assert!(app.world().get::<PendingActorSupport>(far).is_some());
		let placed = app
			.world()
			.get::<Transform>(near)
			.ok_or_else(|| anyhow::anyhow!("near body missing"))?
			.translation;
		assert!((placed.y - (ground + LocomotionCapsule::HUMANOID.spawn_height())).abs() < 0.35);
		assert!(
			placed.y < pose.y - 100.0,
			"placeholder height must not become an authored minimum"
		);
		assert_eq!(app.world().get::<GravityScale>(near).map(|g| g.0), Some(1.25));

		let moved = placed + Vec3::new(2.0, 6.0, 0.0);
		app.world_mut()
			.entity_mut(near)
			.insert((Transform::from_translation(moved), Position(moved)));
		advance(&mut app);
		assert!(app.world().get::<PendingActorSupport>(near).is_none());
		let after = app
			.world()
			.get::<Transform>(near)
			.ok_or_else(|| anyhow::anyhow!("released body missing"))?
			.translation;
		assert!(after.y > placed.y + 4.0, "released members are not continuously snapped");
		Ok(())
	}

	#[test]
	fn floor_arrival_waits_for_its_floor_and_keeps_the_authored_member_offset() -> anyhow::Result<()>
	{
		let mut app = support_app();
		let pose = Vec3::new(8.0, 15.0, 8.0);
		let floor = member(&mut app, pose, 0);
		app.world_mut()
			.entity_mut(floor)
			.insert(MobRespawnSupport::new(PoiSupport::Floor, Some(12.0)));
		fixed_floor(&mut app, pose.xz(), 0.0);
		advance(&mut app);
		assert!(
			app.world().get::<PendingActorSupport>(floor).is_some(),
			"lower ground is not the authored floor"
		);
		assert_eq!(app.world().get::<Transform>(floor).map(|t| t.translation), Some(pose));
		fixed_floor(&mut app, pose.xz(), 12.0);
		advance(&mut app);
		assert!(app.world().get::<PendingActorSupport>(floor).is_none());
		assert_eq!(app.world().get::<Transform>(floor).map(|t| t.translation), Some(pose));
		assert_eq!(app.world().get::<GravityScale>(floor).map(|g| g.0), Some(1.25));
		Ok(())
	}

	#[test]
	fn terrain_respawn_retains_its_authored_clearance() {
		let mut app = support_app();
		let ground = publish_origin(&mut app);
		let pose = Vec3::new(8.0, ground + 2.5, 8.0);
		let body = member(&mut app, pose, 0);
		app.world_mut()
			.entity_mut(body)
			.insert(MobRespawnSupport::new(PoiSupport::Terrain, Some(ground)));
		fixed_floor(&mut app, pose.xz(), ground);
		advance(&mut app);
		assert!(app.world().get::<PendingActorSupport>(body).is_none());
		assert_eq!(app.world().get::<Transform>(body).map(|t| t.translation), Some(pose));
	}
	fn obstacle(app: &mut App, center: Vec3, width: f32) {
		app.world_mut().spawn((
			Transform::from_translation(center),
			RigidBody::Static,
			Collider::cuboid(width, 2_000.0, width),
			PhysicsInteractionLayer::fixed_layers(),
		));
	}

	fn broad_floor(app: &mut App, center: Vec3) {
		app.world_mut().spawn((
			Transform::from_translation(center - Vec3::Y * 0.1),
			RigidBody::Static,
			Collider::cuboid(30.0, 0.1, 30.0),
			PhysicsInteractionLayer::fixed_layers(),
		));
	}

	#[test]
	fn configured_seventy_degree_support_accepts_a_steep_terrain_collider() -> anyhow::Result<()> {
		let mut app = support_app();
		app.insert_resource(characters::CharacterLocomotion {
			max_slope_angle: 70.0_f32.to_radians(),
		});
		let ground = publish_origin(&mut app);
		let body = member(&mut app, Vec3::new(8.0, ground + 20.0, 8.0), 0);
		// Just inside the configured limit avoids making the assertion depend on
		// floating-point normal reconstruction at an exactly equal boundary.
		let rotation = Quat::from_rotation_z(69.9_f32.to_radians());
		app.world_mut().spawn((
			Transform::from_xyz(8.0, ground, 8.0).with_rotation(rotation),
			Position(Vec3::new(8.0, ground, 8.0)),
			avian3d::prelude::Rotation(rotation),
			RigidBody::Static,
			Collider::cuboid(30.0, 0.1, 30.0),
			PhysicsInteractionLayer::fixed_layers(),
		));
		advance(&mut app);
		assert!(
			app.world().get::<PendingActorSupport>(body).is_none(),
			"motor-walkable slopes accept arrivals"
		);
		let pose = app
			.world()
			.get::<Transform>(body)
			.ok_or_else(|| anyhow::anyhow!("slope body missing"))?
			.translation;
		assert!(
			(pose.xz() - Vec2::splat(8.0)).length() < 0.01,
			"valid original terrain support stays exact"
		);
		let outcome = app
			.world_mut()
			.run_system_once(move |spatial: avian3d::prelude::SpatialQuery| {
				maybraid_spawns_support::probe_search_with_locomotion(
					&spatial,
					&PhysicsInteractionLayer::Fixed.query_filter(),
					PlacementSearch::outdoor_hint(
						Vec2::splat(8.0),
						ground,
						LocomotionCapsule::HUMANOID,
					),
					&characters::CharacterLocomotion { max_slope_angle: 70.0_f32.to_radians() },
				)
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let maybraid_spawns_support::SupportProbeOutcome::Ready { support: Some(hit), .. } =
			outcome
		else {
			return Err(anyhow::anyhow!("the rotated support must remain probeable: {outcome:?}"));
		};
		assert!(
			(hit.normal.y.acos().to_degrees() - 69.9).abs() < 0.2,
			"the fixture must exercise its authored slope, actual normal {:?}",
			hit.normal
		);
		Ok(())
	}

	#[test]
	fn blocked_initial_member_uses_bounded_nearby_support() -> anyhow::Result<()> {
		let mut app = support_app();
		let ground = publish_origin(&mut app);
		let center = Vec3::new(8.0, ground, 8.0);
		broad_floor(&mut app, center);
		obstacle(&mut app, center, 1.0);
		let body = member(&mut app, center + Vec3::Y * 200.0, 0);
		for _ in 0..12 {
			app.update();
		}
		assert!(
			app.world().get::<PendingActorSupport>(body).is_none(),
			"a nearby clear column should release"
		);
		let pose = app
			.world()
			.get::<Transform>(body)
			.ok_or_else(|| anyhow::anyhow!("relocated body missing"))?
			.translation;
		let distance = (pose.xz() - center.xz()).length();
		assert!(distance > 0.5 && distance <= INITIAL_MOB_SEARCH_RADIUS + 0.01);
		assert!(pose.y < center.y + 3.0);
		let hull = LocomotionCapsule::HUMANOID;
		let result = app
			.world_mut()
			.run_system_once(move |spatial: avian3d::prelude::SpatialQuery| {
				maybraid_spawns_support::clearance_blocked(
					&spatial,
					&PhysicsInteractionLayer::Fixed.query_filter(),
					pose,
					hull,
					None,
				)
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!result, "the accepted standing capsule must clear fixed geometry");
		Ok(())
	}

	#[test]
	fn blocked_floor_respawn_keeps_its_authored_column() -> anyhow::Result<()> {
		let mut app = support_app();
		let floor_y = 12.0;
		let pose = Vec3::new(8.0, floor_y + 2.5, 8.0);
		broad_floor(&mut app, Vec3::new(8.0, floor_y, 8.0));
		obstacle(&mut app, Vec3::new(8.0, floor_y, 8.0), 1.0);
		let body = member(&mut app, pose, 0);
		app.world_mut()
			.entity_mut(body)
			.insert(MobRespawnSupport::new(PoiSupport::Floor, Some(floor_y)));
		for _ in 0..40 {
			app.update();
		}
		let pending = app
			.world()
			.get::<PendingActorSupport>(body)
			.ok_or_else(|| anyhow::anyhow!("blocked authored floor must stay pending"))?;
		assert_eq!(pending.search_xz(), pose.xz());
		assert!(!pending.exhausted(), "an exact floor request never opts into nearby search");
		assert_eq!(app.world().get::<Transform>(body).map(|t| t.translation), Some(pose));
		Ok(())
	}

	#[test]
	fn exhausted_initial_member_is_omitted_without_releasing_gravity() {
		let mut app = support_app();
		let ground = publish_origin(&mut app);
		let center = Vec3::new(8.0, ground, 8.0);
		broad_floor(&mut app, center);
		obstacle(&mut app, center, 40.0);
		let body = member(&mut app, center + Vec3::Y * 200.0, 0);
		for _ in 0..40 {
			app.update();
		}
		assert!(
			!app.world().entities().contains(body),
			"unsupported initial member must not count as a held success"
		);
	}
}
