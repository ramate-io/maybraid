//! Real-physics regressions for the guarded spawn-to-motor handoff.

use super::{actor, app, floor, pending};
use avian3d::prelude::*;
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use characters::LocomotionCapsule;
use lod_avian::PhysicsInteractionLayer;

use crate::{PendingActorSupport, PlacementSearch, SupportProbeOutcome};

const HELD: Vec3 = Vec3::new(0.0, 20.0, 0.0);

fn position(app: &App, entity: Entity) -> anyhow::Result<Vec3> {
	app.world()
		.get::<Position>(entity)
		.map(|position| position.0)
		.ok_or_else(|| anyhow::anyhow!("missing actor position: {entity:?}"))
}

fn touching(app: &App, actor: Entity, support: Entity) -> bool {
	app.world()
		.resource::<ContactGraph>()
		.get(actor, support)
		.is_some_and(|(_, pair)| pair.is_touching() && pair.generates_constraints())
}

fn start_settling(gravity: Option<f32>) -> anyhow::Result<(App, Entity, Entity)> {
	let mut app = app();
	let support = floor(&mut app, 0.0);
	for _ in 0..3 {
		app.update();
	}
	let entity = actor(&mut app, HELD, gravity);
	app.update();
	assert!(
		app.world().get::<PendingActorSupport>(entity).is_some(),
		"placement must await physics confirmation"
	);
	assert!(position(&app, entity)?.y < 3.0, "expected a resolved floor candidate");
	assert!(!touching(&app, entity, support), "the candidate has not stepped yet");
	Ok((app, entity, support))
}

fn wait_for_release(app: &mut App, entity: Entity) -> anyhow::Result<()> {
	for _ in 0..60 {
		app.update();
		if app.world().get::<PendingActorSupport>(entity).is_none() {
			return Ok(());
		}
	}
	anyhow::bail!(
		"actor did not complete physics handoff: position={:?}, pending={:?}",
		app.world().get::<Position>(entity),
		app.world().get::<PendingActorSupport>(entity)
	)
}

fn assert_held(app: &App, entity: Entity, held: Vec3) -> anyhow::Result<()> {
	assert!(app.world().get::<PendingActorSupport>(entity).is_some());
	assert_eq!(position(app, entity)?, held);
	assert_eq!(app.world().get::<GravityScale>(entity).map(|scale| scale.0), Some(0.0));
	assert_eq!(
		app.world().get::<LinearVelocity>(entity).map(|velocity| velocity.0),
		Some(Vec3::ZERO)
	);
	Ok(())
}

#[test]
fn ordinary_spawn_waits_for_contact_then_allows_normal_motion() -> anyhow::Result<()> {
	let (mut app, entity, support) = start_settling(Some(1.0))?;
	wait_for_release(&mut app, entity)?;
	assert!(touching(&app, entity, support), "ordinary release needs solver contact");
	assert_eq!(app.world().get::<GravityScale>(entity).map(|scale| scale.0), Some(1.0));
	let before = position(&app, entity)?;
	app.world_mut().entity_mut(entity).insert(LinearVelocity(Vec3::X * 3.0));
	for _ in 0..6 {
		app.update();
	}
	assert!(position(&app, entity)?.x > before.x + 0.05, "support must stop pinning after release");
	assert!(app.world().get::<PendingActorSupport>(entity).is_none());
	Ok(())
}

#[test]
fn removed_support_reholds_until_replacement_gets_its_own_contact() -> anyhow::Result<()> {
	let (mut app, entity, support) = start_settling(Some(1.3))?;
	assert!(app.world_mut().despawn(support));
	for _ in 0..3 {
		app.update();
		assert_held(&app, entity, HELD)?;
	}
	let replacement = floor(&mut app, 0.0);
	app.update();
	assert!(app.world().get::<PendingActorSupport>(entity).is_some());
	wait_for_release(&mut app, entity)?;
	assert!(touching(&app, entity, replacement));
	assert_eq!(app.world().get::<GravityScale>(entity).map(|scale| scale.0), Some(1.3));
	Ok(())
}

#[test]
fn moved_support_is_revalidated_before_the_next_physics_step() -> anyhow::Result<()> {
	let (mut app, entity, support) = start_settling(Some(0.8))?;
	let away = Vec3::X * 40.0;
	app.world_mut()
		.entity_mut(support)
		.insert((Position(away), Transform::from_translation(away)));
	for _ in 0..3 {
		app.update();
		assert_held(&app, entity, HELD)?;
	}
	app.world_mut()
		.entity_mut(support)
		.insert((Position(Vec3::ZERO), Transform::IDENTITY));
	wait_for_release(&mut app, entity)?;
	assert!(touching(&app, entity, support));
	Ok(())
}

#[derive(Resource)]
struct RemoveOnContact {
	actor: Entity,
	support: Entity,
	removed: bool,
}

fn remove_support_after_contact(world: &mut World) {
	let Some(target) = world.get_resource::<RemoveOnContact>() else { return };
	if target.removed {
		return;
	}
	let support = target.support;
	let touched = world
		.resource::<ContactGraph>()
		.get(target.actor, support)
		.is_some_and(|(_, pair)| pair.is_touching());
	if touched {
		world.despawn(support);
		world.resource_mut::<RemoveOnContact>().removed = true;
	}
}

#[test]
fn support_removed_after_step_cannot_release_from_stale_contact_graph() -> anyhow::Result<()> {
	let (mut app, entity, support) = start_settling(Some(1.0))?;
	app.insert_resource(RemoveOnContact { actor: entity, support, removed: false });
	app.add_systems(
		FixedPostUpdate,
		remove_support_after_contact
			.after(PhysicsSystems::StepSimulation)
			.before(crate::actor::handoff::confirm_actor_support),
	);
	for _ in 0..60 {
		app.update();
		if app.world().resource::<RemoveOnContact>().removed {
			break;
		}
	}
	assert!(
		app.world().resource::<RemoveOnContact>().removed,
		"fixture must observe real contact before removing support"
	);
	assert_held(&app, entity, HELD)?;
	Ok(())
}

#[test]
fn retarget_and_disable_cancel_an_in_progress_handoff() -> anyhow::Result<()> {
	for disable in [false, true] {
		let (mut app, entity, _) = start_settling(Some(1.6))?;
		let target = if disable { HELD } else { Vec3::new(4.0, 25.0, -2.0) };
		{
			let mut support = app
				.world_mut()
				.get_mut::<PendingActorSupport>(entity)
				.ok_or_else(|| anyhow::anyhow!("pending actor missing"))?;
			if disable {
				support.set_enabled(false);
			} else {
				support.retarget(target, LocomotionCapsule::HUMANOID, None);
			}
		}
		for _ in 0..4 {
			app.update();
			assert_held(&app, entity, target)?;
		}
		{
			let mut support = app
				.world_mut()
				.get_mut::<PendingActorSupport>(entity)
				.ok_or_else(|| anyhow::anyhow!("pending actor missing"))?;
			support.set_enabled(true);
			support.observe_search(Some(PlacementSearch::interior_at(
				Vec3::new(target.x, 0.0, target.z),
				LocomotionCapsule::HUMANOID,
			)));
		}
		wait_for_release(&mut app, entity)?;
		assert!(position(&app, entity)?.xz().distance(target.xz()) < 1e-3);
		assert_eq!(app.world().get::<GravityScale>(entity).map(|scale| scale.0), Some(1.6));
	}
	Ok(())
}

#[derive(Resource)]
struct PauseOnContact {
	actor: Entity,
	support: Entity,
	paused: bool,
}

fn pause_after_first_contact(
	mut target: ResMut<PauseOnContact>,
	mut time: ResMut<Time<Physics>>,
	contacts: Res<ContactGraph>,
) {
	if !target.paused
		&& contacts
			.get(target.actor, target.support)
			.is_some_and(|(_, pair)| pair.is_touching())
	{
		time.pause();
		target.paused = true;
	}
}

#[test]
fn paused_physics_cannot_count_the_same_contact_twice() -> anyhow::Result<()> {
	let (mut app, entity, support) = start_settling(Some(1.0))?;
	app.insert_resource(PauseOnContact { actor: entity, support, paused: false });
	app.add_systems(
		FixedPostUpdate,
		pause_after_first_contact
			.after(PhysicsSystems::StepSimulation)
			.before(crate::actor::handoff::confirm_actor_support),
	);
	for _ in 0..60 {
		app.update();
		if app.world().resource::<PauseOnContact>().paused {
			break;
		}
	}
	assert!(app.world().resource::<PauseOnContact>().paused, "fixture must pause on real contact");
	assert!(touching(&app, entity, support));
	let elapsed = app.world().resource::<Time<Physics>>().elapsed();
	for _ in 0..8 {
		app.update();
		assert!(
			app.world().get::<PendingActorSupport>(entity).is_some(),
			"paused contact must not complete handoff"
		);
		assert_eq!(app.world().resource::<Time<Physics>>().elapsed(), elapsed);
	}
	app.world_mut().resource_mut::<Time<Physics>>().unpause();
	wait_for_release(&mut app, entity)?;
	Ok(())
}

#[test]
fn original_gravity_survives_actual_physics_handoff() -> anyhow::Result<()> {
	for gravity in [None, Some(0.0), Some(-1.0), Some(1e-8)] {
		let (mut app, entity, support) = start_settling(gravity)?;
		wait_for_release(&mut app, entity)?;
		assert_eq!(app.world().get::<GravityScale>(entity).map(|scale| scale.0), gravity);
		if gravity.is_none() || gravity.is_some_and(|scale| scale > 0.0) {
			assert!(touching(&app, entity, support), "gravity={gravity:?}");
		}
	}
	Ok(())
}

#[test]
fn compound_floor_does_not_hide_its_own_wall_or_overhang() -> anyhow::Result<()> {
	for overhang in [false, true] {
		let mut app = app();
		let obstacle = if overhang {
			(Vec3::new(0.0, 1.6, 0.0), Collider::cuboid(12.0, 0.2, 12.0))
		} else {
			(Vec3::new(0.45, 1.5, 0.0), Collider::cuboid(0.2, 3.0, 12.0))
		};
		app.world_mut().spawn((
			Transform::IDENTITY,
			RigidBody::Static,
			Collider::compound(vec![
				(Vec3::ZERO, Quat::IDENTITY, Collider::cuboid(12.0, 0.2, 12.0)),
				(obstacle.0, Quat::IDENTITY, obstacle.1),
			]),
			PhysicsInteractionLayer::fixed_layers(),
		));
		for _ in 0..3 {
			app.update();
		}
		let entity = actor(&mut app, HELD, Some(1.0));
		for _ in 0..12 {
			app.update();
			assert_held(&app, entity, HELD)?;
		}
	}
	Ok(())
}

#[test]
fn adjacent_coplanar_floor_seam_settles_without_lateral_relocation() -> anyhow::Result<()> {
	let mut app = app();
	for x in [-3.0, 3.0] {
		app.world_mut().spawn((
			Transform::from_xyz(x, 0.0, 0.0),
			RigidBody::Static,
			Collider::cuboid(6.0, 0.2, 12.0),
			PhysicsInteractionLayer::fixed_layers(),
		));
	}
	for _ in 0..3 {
		app.update();
	}
	let entity = actor(&mut app, HELD, Some(1e-8));
	app.update();
	assert!(app.world().get::<PendingActorSupport>(entity).is_some());
	wait_for_release(&mut app, entity)?;
	assert!(position(&app, entity)?.xz().length() < 1e-3);
	assert_eq!(app.world().get::<GravityScale>(entity).map(|scale| scale.0), Some(1e-8));
	Ok(())
}

#[test]
fn slope_clearance_lift_is_bounded_and_cannot_escape_the_interior_band() -> anyhow::Result<()> {
	for narrow_band in [false, true] {
		let mut app = app();
		// At 75 degrees the central support disk's height is insufficient for
		// the wider capsule. The default locomotion slope limit still accepts it.
		let rotation = Quat::from_rotation_z(75.0_f32.to_radians());
		app.world_mut().spawn((
			Transform::from_rotation(rotation),
			Position(Vec3::ZERO),
			Rotation(rotation),
			RigidBody::Static,
			Collider::cuboid(12.0, 0.2, 12.0),
			PhysicsInteractionLayer::fixed_layers(),
		));
		for _ in 0..3 {
			app.update();
		}
		let mut search = PlacementSearch::interior_at(Vec3::Y * 0.5, LocomotionCapsule::HUMANOID);
		let outcome = app
			.world_mut()
			.run_system_once(move |spatial: SpatialQuery| {
				crate::probe_search(
					&spatial,
					&PhysicsInteractionLayer::Fixed.query_filter(),
					search,
				)
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let SupportProbeOutcome::Ready { pose, support: Some(hit) } = outcome else {
			anyhow::bail!("sloped floor fixture did not produce a candidate: {outcome:?}");
		};
		if narrow_band {
			search.max_surface_y = hit.point.y + 0.01;
		}
		let entity = actor(&mut app, HELD, Some(0.0));
		let mut request = pending(HELD);
		request.observe_search(Some(search));
		app.world_mut().entity_mut(entity).insert(request);
		app.update();
		if narrow_band {
			for _ in 0..10 {
				app.update();
				assert_held(&app, entity, HELD)?;
			}
		} else {
			let lifted = position(&app, entity)?;
			assert!(
				lifted != HELD && app.world().get::<PendingActorSupport>(entity).is_some(),
				"expected a lifted candidate awaiting physics, position={lifted:?}, pending={:?}",
				app.world().get::<PendingActorSupport>(entity)
			);
			assert!(
				lifted.y > pose.position.y + 0.01,
				"fixture must need full-hull lift: {lifted:?} vs {:?}",
				pose.position
			);
			assert!(lifted.y <= pose.position.y + 1.0 + 1e-3);
			assert!(lifted.y <= search.max_surface_y + search.hull.spawn_height() + 1e-3);
			wait_for_release(&mut app, entity)?;
		}
	}
	Ok(())
}
