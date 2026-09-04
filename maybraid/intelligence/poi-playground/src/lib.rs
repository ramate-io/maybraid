//! Visual smoke test for POI discovery, retention, and travel policies.

use bevy::prelude::*;
use journeying_intelligence::{JourneyingIntelligencePlugin, JourneyingIntelligenceUser};
use meandering_intelligence::{MeanderingIntelligencePlugin, MeanderingIntelligenceUser};
use movement_intelligence::{MovementIntelligence, MovementLocation, MovementObjective};
use poi_intelligence::{
	GlobalPoi, LocalPoi, Poi, PoiGoal, PoiGoalState, PoiId, PoiIntelligencePlugin,
	PoiIntelligenceUser, PoiInterest, PoiInterests, PoiKind, PoiKnowledge, PoiLearningPolicy,
	PoiSystems, PoiVisitPolicy, PoiVisitState,
};

const TILE_SIZE: f32 = 256.0;
const GRID_RADIUS: i32 = 8;
const AGENT_Y: f32 = 18.0;
const POI_Y: f32 = 5.0;
const SHELTER: PoiKind = PoiKind::new("playground/shelter");
const VISTA: PoiKind = PoiKind::new("playground/vista");

#[derive(Resource, Default)]
struct PlaygroundPause(bool);

#[derive(Component)]
struct DemoAgent {
	order: u8,
	label: &'static str,
	color: Color,
	speed: f32,
}

#[derive(Component)]
struct StatusText;

type AgentStatus<'a> = (
	&'a DemoAgent,
	&'a PoiKnowledge,
	&'a PoiVisitState,
	Option<&'a PoiGoal>,
	Option<&'a PoiGoalState>,
);

pub struct PoiPlaygroundPlugin;

impl Plugin for PoiPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(PoiIntelligencePlugin)
			.add_plugins(MeanderingIntelligencePlugin)
			.add_plugins(JourneyingIntelligencePlugin)
			.init_resource::<PlaygroundPause>()
			.insert_resource(ClearColor(Color::srgb(0.025, 0.035, 0.055)))
			.add_systems(Startup, setup)
			.add_systems(
				Update,
				(
					toggle_pause,
					walk_toward_objectives.before(PoiSystems::Complete),
					draw_debug_world,
					update_status_text,
				),
			);
	}
}

fn setup(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	setup_camera_and_light(&mut commands);
	setup_ground(&mut commands, &mut meshes, &mut materials);
	setup_pois(&mut commands, &mut meshes, &mut materials);
	setup_agents(&mut commands, &mut meshes, &mut materials);
	setup_hud(&mut commands);
}

fn setup_camera_and_light(commands: &mut Commands) {
	commands.spawn((
		Camera3d::default(),
		Transform::from_xyz(0.0, 3_100.0, 3_300.0).looking_at(Vec3::ZERO, Vec3::Y),
		Projection::Perspective(PerspectiveProjection { near: 1.0, far: 12_000.0, ..default() }),
	));
	commands.insert_resource(GlobalAmbientLight { brightness: 700.0, ..default() });
	commands.spawn((
		DirectionalLight { illuminance: 15_000.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.7, 0.0)),
	));
}

fn setup_ground(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let diameter = TILE_SIZE * (GRID_RADIUS as f32 * 2.0 + 1.0);
	commands.spawn((
		Mesh3d(meshes.add(Plane3d::default().mesh().size(diameter, diameter))),
		MeshMaterial3d(materials.add(Color::srgb(0.075, 0.09, 0.11))),
	));
}

fn setup_pois(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let local_mesh = meshes.add(Sphere::new(8.0));
	let global_mesh = meshes.add(Cylinder::new(10.0, 32.0));
	let shelter_local = materials.add(Color::srgb(0.2, 0.85, 0.48));
	let vista_local = materials.add(Color::srgb(0.35, 0.65, 1.0));
	let shelter_global = materials.add(Color::srgb(1.0, 0.75, 0.18));
	let vista_global = materials.add(Color::srgb(0.95, 0.3, 0.85));

	for tile_x in -GRID_RADIUS..=GRID_RADIUS {
		for tile_z in -GRID_RADIUS..=GRID_RADIUS {
			let global = (tile_x * 31 + tile_z * 17).rem_euclid(7) == 0;
			let kind = if (tile_x + tile_z).rem_euclid(2) == 0 { SHELTER } else { VISTA };
			let id =
				((tile_x + GRID_RADIUS) * (GRID_RADIUS * 2 + 1) + tile_z + GRID_RADIUS + 1) as u64;
			let point = tile_center(IVec2::new(tile_x, tile_z));
			let mesh = if global { global_mesh.clone() } else { local_mesh.clone() };
			let material = match (kind, global) {
				(SHELTER, false) => shelter_local.clone(),
				(VISTA, false) => vista_local.clone(),
				(SHELTER, true) => shelter_global.clone(),
				_ => vista_global.clone(),
			};
			let mut entity = commands.spawn((
				Mesh3d(mesh),
				MeshMaterial3d(material),
				Transform::from_xyz(point.x, POI_Y, point.y),
				Poi::new(PoiId(id), kind).with_arrival_radius(24.0),
				LocalPoi,
			));
			if global {
				entity.insert(GlobalPoi);
			}
		}
	}
}

fn setup_agents(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let mesh = meshes.add(Capsule3d::new(15.0, 30.0));
	let starts = [
		(
			"meander · weighted",
			Vec3::new(-700.0, AGENT_Y, -700.0),
			Color::srgb(0.95, 0.95, 0.95),
			TravelDemo::Meander(PoiVisitPolicy::Weighted {
				novelty_weight: 2.5,
				revisit_cooldown_secs: 20.0,
				repeat_weight: 0.5,
			}),
		),
		(
			"meander · cycle 4",
			Vec3::new(700.0, AGENT_Y, -700.0),
			Color::srgb(1.0, 0.35, 0.2),
			TravelDemo::Meander(PoiVisitPolicy::Cycle {
				roster_size: 4,
				reshuffle_each_cycle: false,
			}),
		),
		(
			"journey · weighted",
			Vec3::new(-180.0, AGENT_Y, 180.0),
			Color::srgb(0.2, 1.0, 0.95),
			TravelDemo::Journey(PoiVisitPolicy::Weighted {
				novelty_weight: 3.0,
				revisit_cooldown_secs: 30.0,
				repeat_weight: 0.25,
			}),
		),
		(
			"journey · cycle 4",
			Vec3::new(180.0, AGENT_Y, 180.0),
			Color::srgb(0.95, 0.25, 1.0),
			TravelDemo::Journey(PoiVisitPolicy::Cycle {
				roster_size: 4,
				reshuffle_each_cycle: false,
			}),
		),
	];

	for (order, (label, start, color, demo)) in starts.into_iter().enumerate() {
		let interests =
			PoiInterests::new([PoiInterest::new(SHELTER, 1.0), PoiInterest::new(VISTA, 1.25)]);
		let learning = PoiLearningPolicy {
			local_radius: 360.0,
			local_scan_interval: 0.15,
			global_scan_interval: 0.6,
			learning_rate_per_second: 40.0,
			retention_secs: 600.0,
			max_known: 512,
			candidates_per_scan: 64,
			..default()
		};
		let mut entity = commands.spawn((
			Mesh3d(mesh.clone()),
			MeshMaterial3d(materials.add(color)),
			Transform::from_translation(start),
			DemoAgent { order: order as u8, label, color, speed: 180.0 },
			PoiIntelligenceUser::new(interests).with_policy(learning),
			PoiKnowledge::default(),
			PoiVisitState::default(),
			MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(start, 8.0))),
		));
		match demo {
			TravelDemo::Meander(visit_policy) => {
				let mut meandering = MeanderingIntelligenceUser::new(360.0);
				meandering.visit_policy = visit_policy;
				entity.insert(meandering);
			}
			TravelDemo::Journey(visit_policy) => {
				let mut journeying = JourneyingIntelligenceUser::new(order as u64 + 41);
				journeying.tile_size = TILE_SIZE;
				journeying.min_tile_distance = 2;
				journeying.max_tile_distance = 6;
				journeying.tile_probes = 24;
				journeying.selection_interval = 0.2;
				journeying.visit_policy = visit_policy;
				entity.insert(journeying);
			}
		}
	}
}

#[derive(Clone, Copy)]
enum TravelDemo {
	Meander(PoiVisitPolicy),
	Journey(PoiVisitPolicy),
}

fn setup_hud(commands: &mut Commands) {
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(12.0),
				left: Val::Px(12.0),
				padding: UiRect::all(Val::Px(12.0)),
				..default()
			},
			BackgroundColor(Color::srgba(0.015, 0.02, 0.035, 0.88)),
		))
		.with_children(|parent| {
			parent.spawn((
				Text::new("POI intelligence"),
				TextFont { font_size: bevy::text::FontSize::Px(17.0), ..default() },
				TextColor(Color::WHITE),
				StatusText,
			));
		});
}

fn toggle_pause(keys: Res<ButtonInput<KeyCode>>, mut paused: ResMut<PlaygroundPause>) {
	if keys.just_pressed(KeyCode::Space) {
		paused.0 = !paused.0;
	}
}

fn walk_toward_objectives(
	time: Res<Time>,
	paused: Res<PlaygroundPause>,
	mut agents: Query<(&DemoAgent, &MovementIntelligence, &mut Transform)>,
) {
	if paused.0 {
		return;
	}
	for (agent, movement, mut transform) in &mut agents {
		let target = movement.objective.location().point;
		let delta = target.xz() - transform.translation.xz();
		let distance = delta.length();
		if distance <= movement.objective.location().radius {
			continue;
		}
		let step = (agent.speed * time.delta_secs()).min(distance);
		let next = delta.normalize_or_zero() * step;
		transform.translation.x += next.x;
		transform.translation.z += next.y;
	}
}

fn draw_debug_world(
	mut gizmos: Gizmos,
	agents: Query<(&DemoAgent, &GlobalTransform, Option<&PoiGoal>)>,
) {
	let edge = TILE_SIZE * (GRID_RADIUS as f32 + 0.5);
	for line in -GRID_RADIUS..=GRID_RADIUS + 1 {
		let offset = (line as f32 - 0.5) * TILE_SIZE;
		let color = Color::srgba(0.35, 0.42, 0.52, 0.24);
		gizmos.line(Vec3::new(-edge, 0.3, offset), Vec3::new(edge, 0.3, offset), color);
		gizmos.line(Vec3::new(offset, 0.3, -edge), Vec3::new(offset, 0.3, edge), color);
	}
	for (agent, transform, goal) in &agents {
		if let Some(goal) = goal {
			gizmos.line(
				transform.translation() + Vec3::Y * 20.0,
				goal.location.point + Vec3::Y * 20.0,
				agent.color,
			);
		}
	}
}

fn update_status_text(
	paused: Res<PlaygroundPause>,
	agents: Query<AgentStatus<'_>>,
	mut text: Query<&mut Text, With<StatusText>>,
) {
	let Ok(mut text) = text.single_mut() else {
		return;
	};
	let mut rows: Vec<_> = agents.iter().collect();
	rows.sort_by_key(|(agent, ..)| agent.order);
	let mut status = format!(
		"POI intelligence   [Space] pause{}\n\
		 green/blue sphere = local   gold/pink pillar = local + global\n\
		 colored line = current goal\n\n",
		if paused.0 { "d" } else { "" }
	);
	for (agent, knowledge, visits, goal, state) in rows {
		let target = goal.map_or_else(
			|| "—".to_string(),
			|goal| {
				format!(
					"{} @ {:.0},{:.0}",
					goal.target.0, goal.location.point.x, goal.location.point.z
				)
			},
		);
		let generation = state.map_or(0, |state| state.generation);
		status.push_str(&format!(
			"{}  known {:>3}  roster {:>2}  gen {:>2}  target {target}\n",
			agent.label,
			knowledge.len(),
			visits.cycle_roster().len(),
			generation,
		));
	}
	*text = Text::new(status);
}

fn tile_center(tile: IVec2) -> Vec2 {
	(tile.as_vec2() + Vec2::splat(0.5)) * TILE_SIZE
}
