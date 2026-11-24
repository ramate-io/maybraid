use super::*;
use bevy::ecs::system::RunSystemOnce;
use poi_intelligence::{complete_poi_goals, PoiGoalCompleted};

#[derive(Component)]
struct TestPlayer;

#[test]
fn neighborhood_geometry_matches_three_centered_cells() {
	for home in [HomeostaticCellIndex(0, 0), HomeostaticCellIndex(-3, 2)] {
		let center = home.center(200.0);
		for offset in [Vec3::new(299.0, 0.0, 299.0), Vec3::new(-299.0, 0.0, -299.0)] {
			let at = center + offset;
			assert!(home.neighborhood_3x3().contains(&HomeostaticCellIndex::containing(at, 200.0)));
			assert!(home.distance_to_neighborhood_edge(at, 200.0) < 0.0);
		}
		assert_eq!(home.distance_to_neighborhood_edge(center + Vec3::X * 260.0, 200.0), -40.0);
		assert_eq!(home.distance_to_neighborhood_edge(center + Vec3::X * 220.0, 200.0), -80.0);
		assert_eq!(home.distance_to_neighborhood_edge(center + Vec3::X * 300.0, 200.0), 0.0);
	}
}

#[test]
fn retreat_preempts_a_journey_and_holds_arrival_until_the_inner_margin() -> anyhow::Result<()> {
	let mut world = World::new();
	world.init_resource::<Time>();
	world.init_resource::<HomeostaticMobConfig>();
	world.init_resource::<Messages<PoiGoalCompleted>>();
	let player = world.spawn((TestPlayer, Transform::from_xyz(270.0, 0.0, 0.0))).id();
	let host = world
		.spawn((
			GlobalTransform::IDENTITY,
			PoiGoal::new(
				4,
				poi_intelligence::PoiId(9),
				None,
				PoiKind::new("test/journey"),
				Vec3::X * 500.0,
				2.0,
				0.0,
				0.0,
			),
		))
		.id();
	world.entity_mut(host).insert(HomeostaticMob {
		home_cell: HomeostaticCellIndex(0, 0),
		mob_entity: host,
		generation: 1,
	});
	world
		.run_system_once(trigger_homeostatic_retreat::<TestPlayer>)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	world.flush();
	world
		.run_system_once(apply_homeostatic_retreat)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	world.flush();
	let hold = world
		.get::<PoiGoalHold>(host)
		.copied()
		.ok_or_else(|| anyhow::anyhow!("missing retreat hold"))?;
	assert_eq!(hold.kind, RETREAT_KIND);
	assert!(world
		.get::<PoiGoal>(host)
		.is_some_and(|goal| hold.matches(goal) && goal.location.point == Vec3::ZERO));
	world
		.run_system_once(complete_poi_goals)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	world.flush();
	assert!(
		world.get::<PoiGoal>(host).is_some(),
		"arrival alone must not release the controller assignment"
	);
	for (x, retained) in [(250.0, true), (221.0, true), (220.0, false)] {
		world.entity_mut(player).insert(Transform::from_xyz(x, 0.0, 0.0));
		world
			.run_system_once(release_homeostatic_retreat::<TestPlayer>)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		assert_eq!(world.get::<HomeostaticRetreat>(host).is_some(), retained);
		assert_eq!(world.get::<PoiGoal>(host).is_some(), retained);
		assert_eq!(world.get::<PoiGoalHold>(host).is_some(), retained);
	}
	Ok(())
}
