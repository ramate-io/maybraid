use super::*;
use bevy::ecs::system::RunSystemOnce;

fn destination(id: u64) -> KnownPoi {
	KnownPoi {
		id: PoiId(id),
		entity: None,
		kind: PoiKind::new("test/held"),
		position: Vec3::ZERO,
		arrival_radius: 1.0,
		salience: 1.0,
		confidence: 1.0,
		sources: PoiSource::EXTERNAL | PoiSource::OBJECTIVE,
		first_observed_at: 0.0,
		last_observed_at: 0.0,
	}
}

#[test]
fn held_arrival_survives_but_a_replacement_generation_can_complete() -> anyhow::Result<()> {
	let mut world = World::new();
	world.init_resource::<Time>();
	world.init_resource::<Messages<PoiGoalCompleted>>();
	let user = world.spawn(GlobalTransform::IDENTITY).id();
	let held = begin_held_poi_goal(&mut world.commands(), user, destination(1), 0.0, None);
	world.flush();
	world
		.run_system_once(complete_poi_goals)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	assert!(world.get::<PoiGoal>(user).is_some_and(|goal| held.matches(goal)));
	let replacement =
		PoiGoal::new(held.generation + 1, held.target, None, held.kind, Vec3::ZERO, 1.0, 0.0, 0.0);
	world.entity_mut(user).insert((
		replacement,
		PoiGoalState {
			generation: replacement.generation,
			target: replacement.target,
			status: PoiGoalStatus::Active,
		},
	));
	world
		.run_system_once(complete_poi_goals)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	world.flush();
	assert!(world.get::<PoiGoal>(user).is_none(), "stale hold must not pin the replacement");
	Ok(())
}

#[test]
fn old_owner_cannot_release_a_foreign_replacement_hold() -> anyhow::Result<()> {
	let mut world = World::new();
	let user = world.spawn_empty().id();
	let old = begin_held_poi_goal(&mut world.commands(), user, destination(1), 0.0, None);
	world.flush();
	let foreign = begin_held_poi_goal(&mut world.commands(), user, destination(2), 0.0, None);
	world.flush();
	old.release(&mut world.commands(), user);
	world.flush();
	assert_eq!(world.get::<PoiGoalHold>(user), Some(&foreign));
	assert!(world.get::<PoiGoal>(user).is_some_and(|goal| foreign.matches(goal)));
	foreign.release(&mut world.commands(), user);
	world.flush();
	assert!(world.get::<PoiGoal>(user).is_none());
	assert!(world.get::<PoiGoalHold>(user).is_none());
	Ok(())
}

#[test]
fn deferred_release_revalidates_a_foreign_replacement_before_applying() -> anyhow::Result<()> {
	let mut world = World::new();
	let user = world.spawn_empty().id();
	let old = begin_held_poi_goal(&mut world.commands(), user, destination(1), 0.0, None);
	world.flush();
	old.release(&mut world.commands(), user);
	// A different controller replaces the assignment after release was queued.
	let goal = PoiGoal::new(
		7,
		PoiId(9),
		None,
		PoiKind::new("test/foreign"),
		Vec3::X * 80.0,
		1.0,
		0.0,
		0.0,
	);
	let foreign = PoiGoalHold { kind: goal.kind, target: goal.target, generation: goal.generation };
	let mut route = RoutingIntelligenceUser::default();
	route.set_destination(goal.location.point);
	world.entity_mut(user).insert((goal, foreign, route));
	world.flush();
	assert_eq!(world.get::<PoiGoalHold>(user), Some(&foreign));
	assert_eq!(world.get::<PoiGoal>(user), Some(&goal));
	assert_eq!(
		world.get::<RoutingIntelligenceUser>(user).and_then(|route| route.destination),
		Some(goal.location.point)
	);
	Ok(())
}
