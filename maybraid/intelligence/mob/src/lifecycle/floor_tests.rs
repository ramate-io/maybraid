//! Floor replacement must keep its bounded pin and retry without consuming a slot.

use anyhow::{Result, anyhow};
use avian3d::prelude::*;
use bevy::{ecs::system::RunSystemOnce, prelude::*, time::TimeUpdateStrategy};
use characters::LocomotionCapsule;
use damage::Health;
use lod_avian::PhysicsInteractionLayer;
use maybraid_spawns_support::{PlacementSearch, SupportProbeOutcome, probe_search};
use npc_intelligence::Personality;
use poi_intelligence::{Poi, PoiId, PoiInterests, PoiKind, PoiRegistry, PoiSupport};
use std::time::Duration;

use crate::lifecycle::respawn_mob_members;
use crate::roster::MobInterests;
use crate::{Mob, MobId, MobMemberNeeded, MobRespawn, MobRoster, RosterMember};

const FLOOR: PoiKind = PoiKind::new("test/mob-floor-replace");
const FLOOR_ID: PoiId = PoiId(77);

fn floor_registry(pin: Vec3, radius: f32) -> Result<PoiRegistry> {
	let mut registry = PoiRegistry::default();
	registry.upsert(
		Entity::from_bits(77),
		Poi::new(FLOOR_ID, FLOOR).with_floor_support().with_arrival_radius(radius),
		pin,
		true,
		false,
	)?;
	Ok(registry)
}

#[test]
fn blocked_floor_keeps_the_replacement_pending_until_a_sibling_moves() -> Result<()> {
	let host_pose = Vec3::new(-60.0, 4.0, -40.0);
	let pin = Vec3::new(35.0, 28.0, 18.0);
	let last_pose = Vec3::new(-55.0, 10.0, -38.0);
	let mut world = World::new();
	world.init_resource::<Time>();
	world.init_resource::<Messages<MobMemberNeeded>>();
	world.insert_resource(floor_registry(pin, 0.0)?);
	let occupied = world.spawn_empty().id();
	let mut pending = RosterMember::new(Personality::Civilian, last_pose);
	pending.health.current = 0.0;
	pending.respawn_at = Some(0.0);
	pending.replacements_used = 1;
	pending.remember_respawn_poi(Some(PoiId(88)));
	let mut sibling = RosterMember::new(Personality::Civilian, pin + Vec3::Y);
	sibling.entity = Some(occupied);
	let host = world
		.spawn((
			Mob::new(16.0),
			MobId(3),
			Transform::from_translation(host_pose),
			MobRespawn::default(),
			MobInterests(PoiInterests::one(FLOOR)),
			MobRoster::new(vec![pending, sibling]),
		))
		.id();
	for _ in 0..3 {
		world
			.run_system_once(respawn_mob_members)
			.map_err(|error| anyhow!("{error:?}"))?;
		assert!(world.resource_mut::<Messages<MobMemberNeeded>>().drain().next().is_none());
		let member = world
			.get::<MobRoster>(host)
			.and_then(|roster| roster.get(0))
			.ok_or_else(|| anyhow!("pending slot missing"))?;
		assert!(!member.spawn_requested);
		assert_eq!(member.health.current, 0.0);
		assert_eq!(member.pose, last_pose);
		assert_eq!(member.respawn_at, Some(0.0));
		assert_eq!(member.replacements_used, 1);
		assert_eq!(member.last_respawn_poi, Some(PoiId(88)));
		assert_eq!(member.excluded_pois(), &[PoiId(88)]);
	}
	world
		.get_mut::<MobRoster>(host)
		.and_then(|mut roster| roster.get_mut(1).map(|member| member.pose = host_pose))
		.ok_or_else(|| anyhow!("occupied slot missing"))?;
	world
		.run_system_once(respawn_mob_members)
		.map_err(|error| anyhow!("{error:?}"))?;
	let requests: Vec<_> = world.resource_mut::<Messages<MobMemberNeeded>>().drain().collect();
	assert_eq!(requests.len(), 1);
	assert_eq!(requests[0].support, PoiSupport::Floor);
	assert_eq!(requests[0].support_y, Some(pin.y));
	assert_eq!(requests[0].pose, pin + Vec3::Y * 3.0);
	let member = world
		.get::<MobRoster>(host)
		.and_then(|roster| roster.get(0))
		.ok_or_else(|| anyhow!("pending slot missing"))?;
	assert!(member.spawn_requested);
	assert_eq!(member.health, Health::default());
	assert_eq!(member.last_respawn_poi, Some(FLOOR_ID));
	assert_eq!(member.excluded_pois(), &[PoiId(88), FLOOR_ID]);
	Ok(())
}

#[test]
fn floor_respawn_metadata_leads_to_the_actual_raised_floor_in_spatial_query() -> Result<()> {
	let pin = Vec3::new(35.0, 28.0, 18.0);
	let host = Vec3::new(-60.0, 4.0, -40.0);
	let last = Vec3::new(-55.0, 10.0, -38.0);
	let registry = floor_registry(pin, 0.5)?;
	let (pose, poi, support, support_y) = MobRespawn::default()
		.placement(
			host,
			last,
			MobId(3),
			0,
			1,
			&[],
			&[host + Vec3::X * 10.0],
			Some(&registry),
			Some(&PoiInterests::one(FLOOR)),
		)
		.ok_or_else(|| anyhow!("clear Floor destination should be available"))?;
	assert_eq!(poi, Some(FLOOR_ID));
	assert_eq!(support, PoiSupport::Floor);
	assert_eq!(support_y, Some(pin.y));
	assert_eq!(pose.y, pin.y + 3.0);
	assert!(pose.xz().distance(pin.xz()) <= 0.5 + 1e-4);
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		PhysicsPlugins::default(),
		bevy::asset::AssetPlugin::default(),
		bevy::mesh::MeshPlugin,
	));
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 64.0)));
	app.finish();
	let floor = app
		.world_mut()
		.spawn((
			Transform::from_translation(pin - Vec3::Y * 0.05),
			RigidBody::Static,
			Collider::cuboid(2.0, 0.1, 2.0),
			PhysicsInteractionLayer::fixed_layers(),
		))
		.id();
	for _ in 0..4 {
		app.update();
	}
	let search =
		PlacementSearch::interior_at(Vec3::new(pose.x, pin.y, pose.z), LocomotionCapsule::HUMANOID);
	let outcome = app
		.world_mut()
		.run_system_once(move |spatial: SpatialQuery| {
			probe_search(&spatial, &PhysicsInteractionLayer::Fixed.query_filter(), search)
		})
		.map_err(|error| anyhow!("{error:?}"))?;
	let SupportProbeOutcome::Ready { support: Some(hit), .. } = outcome else {
		return Err(anyhow!("bounded respawn did not find its raised floor: {outcome:?}"));
	};
	assert_eq!(hit.entity, floor);
	assert!((hit.point.y - pin.y).abs() < 0.01);
	Ok(())
}
