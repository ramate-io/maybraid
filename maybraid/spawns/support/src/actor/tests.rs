mod handoff;
mod trimesh;

use crate::{
	register_actor_support, ActorSupportSystems, PendingActorSupport, PlacementSearch,
	SupportProbeFailure,
};
use avian3d::prelude::*;
use bevy::{prelude::*, time::TimeUpdateStrategy};
use characters::LocomotionCapsule;
use lod_avian::PhysicsInteractionLayer;
use std::time::Duration;

fn app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		PhysicsPlugins::default(),
		bevy::asset::AssetPlugin::default(),
		bevy::mesh::MeshPlugin,
	));
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 30.0)));
	register_actor_support(&mut app);
	app.finish();
	app
}

fn floor(app: &mut App, y: f32) -> Entity {
	app.world_mut()
		.spawn((
			Transform::from_xyz(0.0, y, 0.0),
			RigidBody::Static,
			Collider::cuboid(12.0, 0.2, 12.0),
			PhysicsInteractionLayer::fixed_layers(),
		))
		.id()
}

fn pending(held: Vec3) -> PendingActorSupport {
	let mut pending = PendingActorSupport::new(held, LocomotionCapsule::HUMANOID, None);
	pending.observe_search(Some(PlacementSearch::interior_at(
		Vec3::ZERO,
		LocomotionCapsule::HUMANOID,
	)));
	pending
}

fn actor(app: &mut App, held: Vec3, gravity: Option<f32>) -> Entity {
	let hull = LocomotionCapsule::HUMANOID;
	let mut actor = app.world_mut().spawn((
		Transform::from_translation(held),
		RigidBody::Dynamic,
		Collider::capsule(hull.radius, hull.length),
		PhysicsInteractionLayer::animated_layers(),
		LinearVelocity(Vec3::new(4.0, -7.0, 2.0)),
		AngularVelocity(Vec3::ONE),
		pending(held),
	));
	if let Some(gravity) = gravity {
		actor.insert(GravityScale(gravity));
	}
	actor.id()
}

#[test]
fn late_floor_keeps_the_same_actor_held_then_restores_gravity() -> anyhow::Result<()> {
	let mut app = app();
	let held = Vec3::new(0.0, 30.0, 0.0);
	let entity = actor(&mut app, held, Some(2.75));
	for _ in 0..6 {
		app.update();
		assert!(app.world().get::<PendingActorSupport>(entity).is_some());
		assert_eq!(app.world().get::<Transform>(entity).map(|pose| pose.translation), Some(held));
		assert_eq!(app.world().get::<Position>(entity).map(|pose| pose.0), Some(held));
		assert_eq!(
			app.world().get::<LinearVelocity>(entity).map(|velocity| velocity.0),
			Some(Vec3::ZERO)
		);
		assert_eq!(
			app.world().get::<AngularVelocity>(entity).map(|velocity| velocity.0),
			Some(Vec3::ZERO)
		);
		assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(0.0));
	}
	floor(&mut app, 0.0);
	for _ in 0..4 {
		app.update();
	}
	assert!(app.world().get::<PendingActorSupport>(entity).is_none());
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(2.75));
	let pose = app
		.world()
		.get::<Transform>(entity)
		.ok_or_else(|| anyhow::anyhow!("actor vanished"))?;
	assert!(pose.translation.y < 3.0, "the holding altitude is not a lower bound");
	assert_eq!(pose.translation.xz(), Vec2::ZERO);
	Ok(())
}

#[test]
fn support_waits_for_late_body_creation_and_dynamic_activation() -> anyhow::Result<()> {
	let mut app = app();
	floor(&mut app, 0.0);
	let held = Vec3::new(0.0, 15.0, 0.0);
	let entity = app.world_mut().spawn((Transform::from_translation(held), pending(held))).id();
	for _ in 0..3 {
		app.update();
	}
	assert!(app.world().get::<PendingActorSupport>(entity).is_some());
	app.world_mut().entity_mut(entity).insert((
		RigidBody::Static,
		Collider::capsule(0.4, 1.0),
		PhysicsInteractionLayer::animated_layers(),
		GravityScale(0.65),
	));
	for _ in 0..3 {
		app.update();
	}
	assert!(app.world().get::<PendingActorSupport>(entity).is_some());
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(0.65));
	app.world_mut().entity_mut(entity).insert(RigidBody::Dynamic);
	for _ in 0..3 {
		app.update();
	}
	assert!(app.world().get::<PendingActorSupport>(entity).is_none());
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(0.65));
	Ok(())
}

#[test]
fn zero_and_absent_gravity_survive_pending_retries() {
	for gravity in [Some(0.0), None] {
		let mut app = app();
		let entity = actor(&mut app, Vec3::Y * 20.0, gravity);
		for _ in 0..3 {
			app.update();
		}
		// Domain refreshes only the current column, never the gravity snapshot.
		if let Some(mut pending) = app.world_mut().get_mut::<PendingActorSupport>(entity) {
			pending.observe_search(Some(PlacementSearch::interior_at(
				Vec3::ZERO,
				LocomotionCapsule::HUMANOID,
			)));
		}
		floor(&mut app, 0.0);
		for _ in 0..3 {
			app.update();
		}
		assert!(
			app.world().get::<PendingActorSupport>(entity).is_none(),
			"gravity={gravity:?}, pending={:?}",
			app.world().get::<PendingActorSupport>(entity)
		);
		assert_eq!(app.world().get::<GravityScale>(entity).map(|value| value.0), gravity);
	}
}

#[test]
fn failed_final_clearance_keeps_the_actor_pending_until_geometry_changes() -> anyhow::Result<()> {
	let mut app = app();
	floor(&mut app, 0.0);
	let roof = floor(&mut app, 4.0);
	let entity = actor(&mut app, Vec3::Y * 20.0, Some(1.4));
	app.world_mut().entity_mut(entity).insert({
		let mut pending =
			PendingActorSupport::new(Vec3::Y * 20.0, LocomotionCapsule::HUMANOID, Some(3.5));
		pending.observe_search(Some(PlacementSearch::interior_at(
			Vec3::ZERO,
			LocomotionCapsule::HUMANOID,
		)));
		pending
	});
	for _ in 0..4 {
		app.update();
	}
	let pending = app
		.world()
		.get::<PendingActorSupport>(entity)
		.ok_or_else(|| anyhow::anyhow!("blocked actor released"))?;
	assert_eq!(pending.failure, Some(SupportProbeFailure::Clearance));
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(0.0));
	app.world_mut().despawn(roof);
	for _ in 0..4 {
		app.update();
	}
	assert!(app.world().get::<PendingActorSupport>(entity).is_none());
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(1.4));
	Ok(())
}

#[test]
fn disabled_support_waits_even_with_a_valid_floor() {
	let mut app = app();
	floor(&mut app, 0.0);
	let entity = actor(&mut app, Vec3::Y * 20.0, Some(1.8));
	if let Some(mut pending) = app.world_mut().get_mut::<PendingActorSupport>(entity) {
		pending.set_enabled(false);
	}
	for _ in 0..4 {
		app.update();
	}
	assert!(app.world().get::<PendingActorSupport>(entity).is_some());
	if let Some(mut pending) = app.world_mut().get_mut::<PendingActorSupport>(entity) {
		pending.set_enabled(true);
	}
	app.update();
	assert!(
		app.world().get::<PendingActorSupport>(entity).is_some(),
		"validated placement still awaits physics confirmation"
	);
	for _ in 0..3 {
		app.update();
	}
	assert!(app.world().get::<PendingActorSupport>(entity).is_none());
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(1.8));
}

/// A downstream motor can run during the fixed frame before the support hold.
fn fixed_motor_stamp(
	mut actors: Query<
		(&mut GravityScale, &mut LinearVelocity, &mut AngularVelocity),
		With<PendingActorSupport>,
	>,
) {
	for (mut gravity, mut velocity, mut angular) in &mut actors {
		gravity.0 = 17.0;
		velocity.0 = Vec3::new(3.0, -30.0, 7.0);
		angular.0 = Vec3::ONE;
	}
}

#[test]
fn retarget_before_update_survives_the_next_fixed_physics_tick() -> anyhow::Result<()> {
	let mut app = app();
	let entity = actor(&mut app, Vec3::Y * 20.0, Some(2.2));
	app.world_mut()
		.entity_mut(entity)
		.insert(pending(Vec3::Y * 20.0).with_nearby_terrain(8.0));
	app.update();
	app.add_systems(
		FixedPostUpdate,
		fixed_motor_stamp
			.after(PhysicsSystems::Prepare)
			.before(ActorSupportSystems::Hold),
	);
	let retargeted = Vec3::new(7.0, 25.0, -3.0);
	{
		let mut pending = app
			.world_mut()
			.get_mut::<PendingActorSupport>(entity)
			.ok_or_else(|| anyhow::anyhow!("actor should still be waiting"))?;
		pending.failure = Some(SupportProbeFailure::Clearance);
		pending.retarget(retargeted, LocomotionCapsule::HUMANOID, None);
		assert!(
			matches!(pending.search_policy(), crate::ActorSupportSearchPolicy::NearbyTerrain { max_radius } if max_radius == 8.0)
		);
		assert_eq!(pending.candidate, 0);
		assert!(!pending.exhausted());
		assert_eq!(pending.resume_gravity(), Some(Some(2.2)));
		assert_eq!(pending.search(), None);
		assert_eq!(pending.failure, None);
	}
	// Intentionally run physics before another Update/adapter pass. The old
	// held position must never overwrite the producer's new arrival request.
	app.world_mut().run_schedule(FixedPostUpdate);
	assert_eq!(app.world().get::<Transform>(entity).map(|pose| pose.translation), Some(retargeted));
	assert_eq!(app.world().get::<Position>(entity).map(|pose| pose.0), Some(retargeted));
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(0.0));
	assert_eq!(
		app.world().get::<LinearVelocity>(entity).map(|velocity| velocity.0),
		Some(Vec3::ZERO)
	);
	assert_eq!(
		app.world().get::<AngularVelocity>(entity).map(|velocity| velocity.0),
		Some(Vec3::ZERO)
	);
	let support = floor(&mut app, 0.0);
	app.world_mut().entity_mut(support).insert(Transform::from_xyz(
		retargeted.x,
		0.0,
		retargeted.z,
	));
	if let Some(mut pending) = app.world_mut().get_mut::<PendingActorSupport>(entity) {
		pending.observe_search(Some(PlacementSearch::interior_at(
			Vec3::new(retargeted.x, 0.0, retargeted.z),
			LocomotionCapsule::HUMANOID,
		)));
	}
	for _ in 0..4 {
		app.update();
	}
	assert!(app.world().get::<PendingActorSupport>(entity).is_none());
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(2.2));
	Ok(())
}

#[test]
fn absent_gravity_hold_is_applied_before_fixed_simulation() {
	let mut app = app();
	let held = Vec3::Y * 20.0;
	let entity = actor(&mut app, held, None);
	app.update();
	// Force the deferred-insertion path immediately before an actual physics
	// tick. Absent GravityScale otherwise means normal gravity to Avian.
	app.world_mut().entity_mut(entity).remove::<GravityScale>();
	app.world_mut().run_schedule(FixedPostUpdate);
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(0.0));
	assert_eq!(app.world().get::<Position>(entity).map(|pose| pose.0), Some(held));
	assert_eq!(
		app.world().get::<LinearVelocity>(entity).map(|velocity| velocity.0),
		Some(Vec3::ZERO)
	);
	floor(&mut app, 0.0);
	for _ in 0..4 {
		app.update();
	}
	assert!(app.world().get::<PendingActorSupport>(entity).is_none());
	assert!(app.world().get::<GravityScale>(entity).is_none());
}

#[test]
fn configured_seventy_degree_support_accepts_captured_slope_and_rejects_steeper_geometry() {
	for (slope_degrees, accepted) in [(57.0_f32, true), (72.0_f32, false)] {
		let mut app = app();
		app.insert_resource(characters::CharacterLocomotion {
			max_slope_angle: 70.0_f32.to_radians(),
		});
		let rotation = Quat::from_rotation_z(slope_degrees.to_radians());
		let support = app
			.world_mut()
			.spawn((
				Transform::from_rotation(rotation),
				Position(Vec3::ZERO),
				Rotation(rotation),
				RigidBody::Static,
				Collider::cuboid(12.0, 0.2, 12.0),
				PhysicsInteractionLayer::fixed_layers(),
			))
			.id();
		for _ in 0..2 {
			app.update();
		}
		assert!(app
			.world()
			.get::<Rotation>(support)
			.is_some_and(|actual| actual.0.angle_between(rotation) < 1e-3));
		let entity = actor(&mut app, Vec3::Y * 20.0, Some(1.25));
		for _ in 0..4 {
			app.update();
		}
		assert_eq!(
			app.world().get::<PendingActorSupport>(entity).is_none(),
			accepted,
			"slope={slope_degrees}"
		);
		if !accepted {
			assert_eq!(
				app.world()
					.get::<PendingActorSupport>(entity)
					.and_then(|pending| pending.failure),
				Some(SupportProbeFailure::Clearance)
			);
		}
	}
}

fn refresh_nearby_test_columns(mut actors: Query<&mut PendingActorSupport>) {
	for mut pending in &mut actors {
		if !pending.exhausted() {
			let search = PlacementSearch::outdoor_hint(pending.search_xz(), 0.0, pending.hull());
			pending.observe_search(Some(search));
		}
	}
}

fn blocked_destination(app: &mut App, width: f32) {
	floor(app, 0.0);
	app.world_mut().spawn((
		// This solid fully contains the support disk at its starting point.
		// Clearance must reject containment as well as crossing surfaces.
		Transform::from_xyz(0.0, 10.0, 0.0),
		Position(Vec3::new(0.0, 10.0, 0.0)),
		RigidBody::Static,
		Collider::cuboid(width, 40.0, width),
		PhysicsInteractionLayer::fixed_layers(),
	));
	for _ in 0..3 {
		app.update();
	}
}

#[test]
fn initial_nearby_policy_uses_the_same_probe_and_exact_destinations_remain_held(
) -> anyhow::Result<()> {
	for nearby in [false, true] {
		let mut app = app();
		app.add_systems(Update, refresh_nearby_test_columns.in_set(ActorSupportSystems::Prepare));
		blocked_destination(&mut app, 1.0);
		let entity = actor(&mut app, Vec3::Y * 20.0, Some(1.6));
		if nearby {
			let support = pending(Vec3::Y * 20.0).with_nearby_terrain(8.0);
			app.world_mut().entity_mut(entity).insert(support);
		}
		for _ in 0..5 {
			app.update();
		}
		if nearby {
			assert!(app.world().get::<PendingActorSupport>(entity).is_none());
			let accepted = app
				.world()
				.get::<Transform>(entity)
				.ok_or_else(|| anyhow::anyhow!("actor missing"))?
				.translation;
			assert!(accepted.xz().length() > 1.0 && accepted.xz().length() <= 8.0);
			assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(1.6));
		} else {
			let pending = app
				.world()
				.get::<PendingActorSupport>(entity)
				.ok_or_else(|| anyhow::anyhow!("exact actor unexpectedly released"))?;
			assert_eq!(pending.search_xz(), Vec2::ZERO);
			assert!(!pending.exhausted());
		}
	}
	Ok(())
}

#[test]
fn blocked_nearby_search_exhausts_its_finite_radius_without_releasing() -> anyhow::Result<()> {
	let mut app = app();
	app.add_systems(Update, refresh_nearby_test_columns.in_set(ActorSupportSystems::Prepare));
	blocked_destination(&mut app, 40.0);
	let entity = actor(&mut app, Vec3::Y * 20.0, Some(1.8));
	app.world_mut()
		.entity_mut(entity)
		.insert(pending(Vec3::Y * 20.0).with_nearby_terrain(8.0));
	for _ in 0..36 {
		app.update();
		let pending = app
			.world()
			.get::<PendingActorSupport>(entity)
			.ok_or_else(|| anyhow::anyhow!("blocked actor released"))?;
		assert!(pending.search_xz().length() <= 8.0001);
	}
	let pending = app
		.world()
		.get::<PendingActorSupport>(entity)
		.ok_or_else(|| anyhow::anyhow!("blocked actor missing"))?;
	assert!(pending.exhausted());
	assert_eq!(pending.candidate, 32);
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(0.0));
	assert_eq!(
		app.world().get::<Transform>(entity).map(|transform| transform.translation),
		Some(Vec3::Y * 20.0)
	);
	Ok(())
}

#[test]
fn unavailable_presentation_and_search_do_not_consume_nearby_candidates() -> anyhow::Result<()> {
	let mut app = app();
	let entity = actor(&mut app, Vec3::Y * 20.0, Some(1.25));
	app.world_mut()
		.entity_mut(entity)
		.insert(pending(Vec3::Y * 20.0).with_nearby_terrain(8.0));
	for _ in 0..5 {
		app.update();
	}
	{
		let mut pending = app
			.world_mut()
			.get_mut::<PendingActorSupport>(entity)
			.ok_or_else(|| anyhow::anyhow!("waiting actor missing"))?;
		assert_eq!(pending.candidate, 0);
		assert!(!pending.exhausted());
		pending.observe_search(None);
	}
	for _ in 0..5 {
		app.update();
	}
	let pending = app
		.world()
		.get::<PendingActorSupport>(entity)
		.ok_or_else(|| anyhow::anyhow!("waiting actor missing"))?;
	assert_eq!(pending.candidate, 0);
	assert!(!pending.exhausted());
	Ok(())
}

#[test]
fn airborne_arrival_rejects_full_containment_inside_a_solid_collider() -> anyhow::Result<()> {
	let mut app = app();
	blocked_destination(&mut app, 40.0);
	let entity = actor(&mut app, Vec3::Y * 20.0, Some(1.25));
	let mut support = pending(Vec3::Y * 20.0);
	support.observe_search(Some(PlacementSearch::airborne_at(
		Vec3::ZERO,
		LocomotionCapsule::HUMANOID,
	)));
	app.world_mut().entity_mut(entity).insert(support);
	for _ in 0..4 {
		app.update();
	}
	let pending = app
		.world()
		.get::<PendingActorSupport>(entity)
		.ok_or_else(|| anyhow::anyhow!("airborne actor released inside a solid collider"))?;
	assert_eq!(pending.failure, Some(SupportProbeFailure::Clearance));
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(0.0));
	assert_eq!(
		app.world().get::<Transform>(entity).map(|pose| pose.translation),
		Some(Vec3::Y * 20.0)
	);
	Ok(())
}
