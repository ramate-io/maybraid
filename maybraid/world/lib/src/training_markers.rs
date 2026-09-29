//! Screen-space dots over every standing Training enemy. Like the mob HUD's
//! pins, an enemy off screen clamps to the nearest screen edge.

use bevy::prelude::*;
use combat_hud::CombatHudVisible;
use damage::{Downed, Health};
use mob_intelligence::MemberOf;

use crate::training::TrainingGrounds;
use crate::training_plaza::TrainingBrawler;
use crate::ui::project_mob_pin;

const MARKER_PX: f32 = 10.0;
const MARKER_BORDER_PX: f32 = 1.5;
/// Lifts the dot over a fighter's head.
const MARKER_LIFT_M: f32 = 2.4;
const MARKER_FILL: Color = Color::srgba(1.0, 0.24, 0.2, 0.9);
const ON_SCREEN_RING: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
const OFF_SCREEN_RING: Color = Color::WHITE;

/// The game copies the Training pause menu's Enemy markers row onto this.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingEnemyMarkersEnabled(pub bool);

impl Default for TrainingEnemyMarkersEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// Full-screen root for the dots. Exists only during Training, with markers on.
#[derive(Component)]
pub(crate) struct TrainingEnemyMarkers;

#[derive(Component)]
pub(crate) struct TrainingEnemyMarker {
	target: Entity,
}

impl TrainingEnemyMarkers {
	fn spawn(commands: &mut Commands) -> Entity {
		commands
			.spawn((
				Name::new("training-enemy-markers"),
				Self,
				Node {
					position_type: PositionType::Absolute,
					width: Val::Percent(100.0),
					height: Val::Percent(100.0),
					..default()
				},
				GlobalZIndex(i32::MAX - 9),
				Visibility::Inherited,
				Pickable::IGNORE,
			))
			.id()
	}
}

impl TrainingEnemyMarker {
	fn node(screen: Vec2) -> Node {
		Node {
			position_type: PositionType::Absolute,
			left: Val::Px(screen.x - MARKER_PX * 0.5),
			top: Val::Px(screen.y - MARKER_PX * 0.5),
			width: Val::Px(MARKER_PX),
			height: Val::Px(MARKER_PX),
			border: UiRect::all(Val::Px(MARKER_BORDER_PX)),
			border_radius: BorderRadius::all(Val::Px(MARKER_PX * 0.5)),
			..default()
		}
	}

	fn place(node: &mut Node, screen: Vec2) {
		node.left = Val::Px(screen.x - MARKER_PX * 0.5);
		node.top = Val::Px(screen.y - MARKER_PX * 0.5);
	}

	fn ring(on_screen: bool) -> BorderColor {
		BorderColor::all(if on_screen { ON_SCREEN_RING } else { OFF_SCREEN_RING })
	}

	fn anchor(feet: Vec3) -> Vec3 {
		feet + Vec3::Y * MARKER_LIFT_M
	}
}

type MarkerParts<'a> = (
	Entity,
	&'a TrainingEnemyMarker,
	&'a mut Node,
	&'a mut BorderColor,
	&'a mut Visibility,
);

/// One dot per standing member of a Training Brawler squad. A downed or dead
/// fighter loses its dot; leaving Training, or turning markers off, drops the
/// whole layer.
pub(crate) fn sync_training_enemy_markers(
	mut commands: Commands,
	grounds: Res<TrainingGrounds>,
	enabled: Option<Res<TrainingEnemyMarkersEnabled>>,
	hud: Option<Res<CombatHudVisible>>,
	camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
	mut roots: Query<
		(Entity, &mut Visibility),
		(With<TrainingEnemyMarkers>, Without<TrainingEnemyMarker>),
	>,
	squads: Query<(), With<TrainingBrawler>>,
	fighters: Query<(Entity, &MemberOf, &Health, &GlobalTransform), Without<Downed>>,
	mut markers: Query<MarkerParts<'_>, Without<TrainingEnemyMarkers>>,
) {
	if !grounds.0 || enabled.is_some_and(|enabled| !enabled.0) {
		for (root, _) in &roots {
			commands.entity(root).try_despawn();
		}
		return;
	}
	let root = match roots.single_mut() {
		Ok((root, mut visibility)) => {
			let shown = hud.is_none_or(|hud| hud.0);
			visibility.set_if_neq(if shown { Visibility::Inherited } else { Visibility::Hidden });
			root
		}
		Err(_) => TrainingEnemyMarkers::spawn(&mut commands),
	};
	let standing: Vec<(Entity, Vec3)> = fighters
		.iter()
		.filter(|(_, member, health, _)| squads.contains(member.mob) && !health.is_dead())
		.map(|(entity, _, _, transform)| (entity, transform.translation()))
		.collect();
	let camera = camera.single().ok();
	let mut marked = Vec::with_capacity(standing.len());
	for (marker, of, mut node, mut ring, mut visibility) in &mut markers {
		let Some(&(target, feet)) = standing.iter().find(|(target, _)| *target == of.target) else {
			commands.entity(marker).despawn();
			continue;
		};
		marked.push(target);
		let projected = camera.and_then(|(camera, eye)| {
			project_mob_pin(camera, eye, TrainingEnemyMarker::anchor(feet))
		});
		let Some((screen, on_screen)) = projected else {
			visibility.set_if_neq(Visibility::Hidden);
			continue;
		};
		TrainingEnemyMarker::place(&mut node, screen);
		ring.set_if_neq(TrainingEnemyMarker::ring(on_screen));
		visibility.set_if_neq(Visibility::Inherited);
	}
	let Some((camera, eye)) = camera else {
		return;
	};
	for &(target, feet) in standing.iter().filter(|(target, _)| !marked.contains(target)) {
		let Some((screen, on_screen)) =
			project_mob_pin(camera, eye, TrainingEnemyMarker::anchor(feet))
		else {
			continue;
		};
		commands.entity(root).with_child((
			Name::new("training-enemy-marker"),
			TrainingEnemyMarker { target },
			TrainingEnemyMarker::node(screen),
			BackgroundColor(MARKER_FILL),
			TrainingEnemyMarker::ring(on_screen),
			Visibility::Inherited,
			Pickable::IGNORE,
		));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn run(world: &mut World, system: &mut impl System<In = (), Out = ()>) -> anyhow::Result<()> {
		system.run((), world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		Ok(())
	}

	fn markers(world: &mut World) -> Vec<Entity> {
		world.query::<&TrainingEnemyMarker>().iter(world).map(|marker| marker.target).collect()
	}

	#[test]
	fn the_marker_layer_lives_only_while_training() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(true));
		let mut system = IntoSystem::into_system(sync_training_enemy_markers);
		system.initialize(&mut world);
		run(&mut world, &mut system)?;
		let roots = world.query_filtered::<(), With<TrainingEnemyMarkers>>().iter(&world).count();
		assert_eq!(roots, 1);

		world.insert_resource(TrainingGrounds(false));
		run(&mut world, &mut system)?;
		let roots = world.query_filtered::<(), With<TrainingEnemyMarkers>>().iter(&world).count();
		assert_eq!(roots, 0);
		Ok(())
	}

	#[test]
	fn turning_markers_off_drops_the_layer() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(true));
		world.insert_resource(TrainingEnemyMarkersEnabled(true));
		let mut system = IntoSystem::into_system(sync_training_enemy_markers);
		system.initialize(&mut world);
		run(&mut world, &mut system)?;
		let count = |world: &mut World| {
			world.query_filtered::<(), With<TrainingEnemyMarkers>>().iter(world).count()
		};
		assert_eq!(count(&mut world), 1);

		world.insert_resource(TrainingEnemyMarkersEnabled(false));
		run(&mut world, &mut system)?;
		assert_eq!(count(&mut world), 0, "off drops the layer");

		world.insert_resource(TrainingEnemyMarkersEnabled(true));
		run(&mut world, &mut system)?;
		assert_eq!(count(&mut world), 1, "on brings it back");
		Ok(())
	}

	#[test]
	fn downed_and_foreign_fighters_lose_their_dots() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(true));
		let squad = world.spawn(TrainingBrawler).id();
		let stranger = world.spawn_empty().id();
		let fighter = |mob| {
			(MemberOf { mob, slot: 0 }, Health::from_max(10.0), GlobalTransform::IDENTITY)
		};
		let standing = world.spawn(fighter(squad)).id();
		let down = Downed { source: None, point: Vec3::ZERO, at: 0.0 };
		let downed = world.spawn((fighter(squad), down)).id();
		let foreign = world.spawn(fighter(stranger)).id();
		for target in [standing, downed, foreign] {
			world.spawn((
				TrainingEnemyMarker { target },
				Node::default(),
				BorderColor::default(),
				Visibility::Inherited,
			));
		}
		let mut system = IntoSystem::into_system(sync_training_enemy_markers);
		system.initialize(&mut world);
		run(&mut world, &mut system)?;
		assert_eq!(markers(&mut world), vec![standing]);
		Ok(())
	}

	#[test]
	fn dots_ride_above_the_head() {
		let anchor = TrainingEnemyMarker::anchor(Vec3::new(1.0, 2.0, 3.0));
		assert!(anchor.abs_diff_eq(Vec3::new(1.0, 4.4, 3.0), 1e-5));
	}
}
